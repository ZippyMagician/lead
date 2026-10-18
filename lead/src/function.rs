use std::fmt::{self, Debug};
use std::rc::Rc;

use crate::builtins::symbols::Symbol;
use crate::scope::Scope;
use crate::types::Type;
use crate::types::list::List;
use crate::utils::Arena;
use crate::utils::error::Status;
use crate::utils::function::*;

// Applied: is general rank support baked into the definition?
// General: does it take + yield a generic Type, or only Lists?
const APPLIED_MONADIC: u8 = 0b0001;
const APPLIED_DYADIC: u8 = 0b0010;
const GENERAL_MONADIC: u8 = 0b0100;
const GENERAL_DYADIC: u8 = 0b1000;

#[derive(Clone)]
pub struct Function {
    // Monad / Dyad
    f: Func,

    pub(crate) fl: u8,
    // Underlying symbol
    pub(crate) s: Option<Symbol>,
    // Opts
    op: Opts,
}

macro_rules! f {
    ($v:expr, $f:expr) => {{ $v & $f != 0 }};
}

pub type Func = Rc<dyn Fn(Type, Option<Type>, &Opts) -> Status<Type>>;

impl Function {
    pub fn new(f: Func, rm: u8, rd: [u8; 2], a: Arena, s: Scope, sym: Option<Symbol>) -> Self {
        Self { f, fl: 0, s: sym, op: Opts { rm, rd, a, s } }
    }

    pub fn scalar(sc: List, a: Arena, s: Scope) -> Self {
        let v: Type = sc.into();
        Self {
            f: Rc::new(move |_, _, _| Ok(v.clone())),
            fl: GENERAL_MONADIC | GENERAL_DYADIC,
            s: None,
            op: Opts { rm: 99, rd: [99, 99], a, s },
        }
    }

    pub fn new_applied(
        f: Func, rm: u8, rd: [u8; 2], am: bool, ad: bool, vm: bool, vd: bool, a: Arena, s: Scope,
        sym: Option<Symbol>,
    ) -> Self {
        Self {
            f,
            fl: (am as u8) * APPLIED_MONADIC
                | (ad as u8) * APPLIED_DYADIC
                | (vm as u8) * GENERAL_MONADIC
                | (vd as u8) * GENERAL_DYADIC,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    #[inline(always)]
    pub fn monadic_call(&self, a: Type) -> Status<Type> {
        (self.f)(a, None, &self.op)
    }

    #[inline(always)]
    pub fn dyadic_call(&self, a: Type, b: Type) -> Status<Type> {
        (self.f)(a, Some(b), &self.op)
    }

    #[inline(always)]
    pub fn call_fn(&self, a: Type, b: Option<Type>) -> Status<Type> {
        (self.f)(a, b, &self.op)
    }

    // Applies new rank to monadic function
    #[inline]
    pub fn apply_rm(&mut self, rm: u8) {
        self.s = None;
        self.op.rm = rm;
        if !f!(self.fl, APPLIED_MONADIC) {
            let m = self.f.clone();
            let vm = f!(self.fl, GENERAL_MONADIC);
            self.f = Rc::new(move |a, b, o| {
                if b.is_some() {
                    (m)(a, b, &o)
                } else if vm && !a.is_list() {
                    (m)(a, None, &o)
                } else {
                    let a = a.to_list()?;
                    Ok(apply_loosem(&o.a, a, o.rm, |a| (m)(Type::from(a), None, &o)?.to_list())?
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
            let vd = f!(self.fl, GENERAL_DYADIC);
            self.f = Rc::new(move |a, b, o| {
                if let Some(b) = b {
                    if vd && (!a.is_list() || !b.is_list()) {
                        (d)(a, Some(b), &o)
                    } else {
                        Ok(apply_loosed(&o.a, a.to_list()?, b.to_list()?, o.rd, |a, b| {
                            (d)(Type::from(a), Some(Type::from(b)), &o)?.to_list()
                        })?
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

#[macro_export]
macro_rules! function {
    ($f:expr; $arena:expr, $scope:expr) => {
        $crate::function::Function::new(::std::rc::Rc::new($f), 99, [99, 99], $arena, $scope, None)
    };

    ($rm:literal, $rd:expr, $am:ident, $ad:ident, $f:expr; $a:ident, $s:ident) => {{
        $crate::function::Function::new_applied(
            ::std::rc::Rc::new($f),
            $rm, $rd, $am, $ad, true, true, $a, $s, None,)
    }};

    (@emit dyad false; $dyad:expr, $a:ident, $b:ident, $o:ident) => {{
        $dyad($a.to_list()?, $b.to_list()?, $o).map($crate::types::Type::from)
    }};

    (@emit dyad true; $dyad:expr, $a:ident, $b:ident, $o:ident) => {{
        $dyad($a, $b, $o)
    }};

    (@emit monad false; $monad:expr, $a:ident, $o:ident) => {{
        $monad($a.to_list()?, $o).map($crate::types::Type::from)
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
            }), $rm, $rd, $am, $ad, $vm, $vd, $a, $s, $sym,
        )
    }};
}

pub(crate) use function;

