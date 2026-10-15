use std::fmt::{self, Debug};
use std::rc::Rc;

use crate::builtins::symbols::Symbol;
use crate::exec::Value;
use crate::list::List;
use crate::scope::Scope;
use crate::utils::Arena;
use crate::utils::error::Status;
use crate::utils::function::*;

const APPLIED_MONADIC: u8 = 0b0001;
const APPLIED_DYADIC: u8 = 0b0010;
const VALUE_MONADIC: u8 = 0b0100;
const VALUE_DYADIC: u8 = 0b1000;

#[derive(Clone)]
pub struct Function {
    // Monad / Dyad
    f: Func,

    // Is it applyable? (i.e., `@` works)
    pub(crate) fl: u8,
    // pub(crate) am: bool,
    // pub(crate) ad: bool,
    // Underlying symbol
    pub(crate) s: Option<Symbol>,
    // Opts
    op: Opts,
}

macro_rules! f {
    ($v:expr, $f:expr) => {{ $v & $f != 0 }};
}

pub type Func = Rc<dyn Fn(Value, Option<Value>, &Opts) -> Status<Value>>;
pub type Monad = Rc<dyn Fn(List, &Opts) -> Status<List>>;
pub type Dyad = Rc<dyn Fn(List, List, &Opts) -> Status<List>>;

impl Function {
    pub fn new(f: Func, rm: u8, rd: [u8; 2], a: Arena, s: Scope, sym: Option<Symbol>) -> Self {
        Self {
            f,
            fl: 0,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    pub fn scalar(sc: List, a: Arena, s: Scope) -> Self {
        let v = Value::List(sc);
        Self {
            f: Rc::new(move |_, _, _| Ok(v.clone())),
            fl: VALUE_MONADIC | VALUE_DYADIC,
            s: None,
            op: Opts {
                rm: 99,
                rd: [99, 99],
                a,
                s,
            },
        }
    }

    pub fn new_applied(
        f: Func,
        rm: u8,
        rd: [u8; 2],
        am: bool,
        ad: bool,
        vm: bool,
        vd: bool,
        a: Arena,
        s: Scope,
        sym: Option<Symbol>,
    ) -> Self {
        Self {
            f,
            fl: (am as u8) * APPLIED_MONADIC
                | (ad as u8) * APPLIED_DYADIC
                | (vm as u8) * VALUE_MONADIC
                | (vd as u8) * VALUE_DYADIC,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    #[inline(always)]
    pub fn monadic_call(&self, a: Value) -> Status<Value> {
        (self.f)(a, None, &self.op)
    }

    #[inline(always)]
    pub fn dyadic_call(&self, a: Value, b: Value) -> Status<Value> {
        (self.f)(a, Some(b), &self.op)
    }

    #[inline(always)]
    pub fn call_fn(&self, a: Value, b: Option<Value>) -> Status<Value> {
        (self.f)(a, b, &self.op)
    }

    // Applies new rank to monadic function
    #[inline]
    pub fn apply_rm(&mut self, rm: u8) {
        self.s = None;
        self.op.rm = rm;
        if !f!(self.fl, APPLIED_MONADIC) {
            let m = self.f.clone();
            let vm = f!(self.fl, VALUE_MONADIC);
            self.f = Rc::new(move |a, b, o| {
                if b.is_some() {
                    (m)(a, b, &o)
                } else if vm && a.is_map() {
                    (m)(a, None, &o)
                } else {
                    let a = a.expect_list()?;
                    Ok(apply_loosem(&o.a, a, o.rm, |a| {
                        (m)(Value::List(a), None, &o)?.expect_list()
                    })?
                    .into())
                }
            });
            self.fl |= APPLIED_MONADIC;
        }
    }

    // Applies new rank to dyadic function
    #[inline]
    pub fn apply_rd(&mut self, rd: [u8; 2]) {
        self.s = None;
        self.op.rd = rd;
        if !f!(self.fl, APPLIED_DYADIC) {
            let d = self.f.clone();
            let vd = f!(self.fl, VALUE_DYADIC);
            self.f =
                Rc::new(move |a, b, o| {
                    if let Some(b) = b {
                        if vd && (a.is_map() || b.is_map()) {
                            (d)(a, Some(b), &o)
                        } else {
                            Ok(apply_loosed(
                                &o.a,
                                a.expect_list()?,
                                b.expect_list()?,
                                o.rd,
                                |a, b| (d)(Value::List(a), Some(Value::List(b)), &o)?.expect_list(),
                            )?
                            .into())
                        }
                    } else {
                        (d)(a, None, &o)
                    }
                });
            self.fl |= APPLIED_DYADIC;
        }
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Function")
    }
}

// Options for functions, e.g. rank override when applicable
#[derive(Clone)]
#[repr(C)]
pub struct Opts {
    pub rm: u8,
    pub rd: [u8; 2],
    pub a: Arena,
    pub s: Scope,
}

macro_rules! function {
    ($f:expr; $arena:expr, $scope:expr) => {
        crate::function::Function::new(::std::rc::Rc::new($f), 99, [99, 99], $arena, $scope, None)
    };

    ($rm:literal, $rd:expr, $am:ident, $ad:ident, $f:expr; $a:ident, $s:ident) => {{
        crate::function::Function::new_applied(
            ::std::rc::Rc::new($f),
            $rm,
            $rd,
            $am,
            $ad,
            true,
            true,
            $a,
            $s,
            None,
        )
    }};

    (@emit dyad false; $dyad:expr, $a:ident, $b:ident, $o:ident) => {{
        $dyad($a.expect_list()?, $b.expect_list()?, $o).map(crate::exec::Value::List)
    }};

    (@emit dyad true; $dyad:expr, $a:ident, $b:ident, $o:ident) => {{
        $dyad($a, $b, $o)
    }};

    (@emit monad false; $monad:expr, $a:ident, $o:ident) => {{
        $monad($a.expect_list()?, $o).map(crate::exec::Value::List)
    }};

    (@emit monad true; $monad:expr, $a:ident, $o:ident) => {{
        $monad($a, $o)
    }};

    ($rm:literal, $am:ident, $vm:ident, $monad:expr; $rd:expr, $ad:ident, $vd:ident, $dyad:expr; $a:ident, $s:ident, $sym:expr) => {{
        crate::function::Function::new_applied(
            ::std::rc::Rc::new(|a, b, o| {
                if let Some(b) = b {
                    function!(@emit dyad $vd; $dyad, a, b, o)
                } else {
                    function!(@emit monad $vm; $monad, a, o)
                }
            }),
            $rm,
            $rd,
            $am,
            $ad,
            $vm,
            $vd,
            $a,
            $s,
            $sym,
        )
    }};
}

pub(crate) use function;
