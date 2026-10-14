use std::fmt::{self, Debug};
use std::rc::Rc;

use crate::builtins::symbols::Symbol;
use crate::list::List;
use crate::scope::Scope;
use crate::utils::Arena;
use crate::utils::error::Status;
use crate::utils::function::*;

#[derive(Clone)]
pub struct Function {
    // Monad / Dyad
    f: Func,

    // Is it applyable? (i.e., `@` works)
    pub(crate) am: bool,
    pub(crate) ad: bool,
    // Underlying symbol
    pub(crate) s: Option<Symbol>,
    // Opts
    op: Opts,
}

pub type Func = Rc<dyn Fn(List, Option<List>, &Opts) -> Status<List>>;
pub type Monad = Rc<dyn Fn(List, &Opts) -> Status<List>>;
pub type Dyad = Rc<dyn Fn(List, List, &Opts) -> Status<List>>;

impl Function {
    pub fn new(
        f: Func,
        rm: u8,
        rd: [u8; 2],
        a: Arena,
        s: Scope,
        sym: Option<Symbol>,
    ) -> Self {
        Self {
            f,
            am: false,
            ad: false,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    pub fn scalar(sc: List, a: Arena, s: Scope) -> Self {
        Self {
            f: Rc::new(move |_, _, _| Ok(sc.clone())),
            am: false,
            ad: false,
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
        a: Arena,
        s: Scope,
        sym: Option<Symbol>,
    ) -> Self {
        Self {
            f,
            am,
            ad,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    #[inline(always)]
    pub fn monadic_call(&self, a: List) -> Status<List> {
        (self.f)(a, None, &self.op)
    }

    #[inline(always)]
    pub fn dyadic_call(&self, a: List, b: List) -> Status<List> {
        (self.f)(a, Some(b), &self.op)
    }

    #[inline(always)]
    pub fn call_fn(&self, a: List, b: Option<List>) -> Status<List> {
        (self.f)(a, b, &self.op)
    }

    // Applies new rank to monadic function
    #[inline]
    pub fn apply_rm(&mut self, rm: u8) {
        self.s = None;
        self.op.rm = rm;
        if !self.am {
            let m = self.f.clone();
            self.f = Rc::new(move |a, b, o| {
                if b.is_some() {
                    (m)(a, b, &o)
                } else {
                    apply_loosem(&o.a, a, o.rm, |a| (m)(a, None, &o))
                }
            });
            self.am = true;
        }
    }

    // Applies new rank to dyadic function
    #[inline]
    pub fn apply_rd(&mut self, rd: [u8; 2]) {
        self.s = None;
        self.op.rd = rd;
        if !self.ad {
            let d = self.f.clone();
            self.f = Rc::new(move |a, b, o| {
                if let Some(b) = b {
                    apply_loosed(&o.a, a, b, o.rd, |a, b| (d)(a, Some(b), &o))
                } else {
                    (d)(a, None, &o)
                }
            });
            self.ad = true;
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
        crate::function::Function::new(
            ::std::rc::Rc::new($f),
            99,
            [99, 99],
            $arena,
            $scope,
            None,
        )
    };

    ($rm:literal, $rd:expr, $am:ident, $ad:ident, $f:expr; $a:ident, $s:ident) => {{
        crate::function::Function::new_applied(
            ::std::rc::Rc::new($f),
            $rm,
            $rd,
            $am,
            $ad,
            $a,
            $s,
            None,
        )
    }};

    ($rm:literal, $am:ident, $monad:expr; $rd:expr, $ad:ident, $dyad:expr; $a:ident, $s:ident, $sym:expr) => {{
        crate::function::Function::new_applied(
            ::std::rc::Rc::new(|a, b, o| {
                if let Some(b) = b {
                    $dyad(a, b, o)
                } else { $monad(a, o) }
            }),
            $rm,
            $rd,
            $am,
            $ad,
            $a,
            $s,
            $sym,
        )
    }}
}

pub(crate) use function;
