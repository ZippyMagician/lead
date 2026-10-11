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
    // Monad
    m: Monad,
    // Dyad
    d: Dyad,

    // Is it applyable? (i.e., `@` works)
    pub(crate) am: bool,
    pub(crate) ad: bool,
    // Underlying symbol
    pub(crate) s: Option<Symbol>,
    // Opts
    op: Opts,
}

pub type Monad = Rc<dyn Fn(List, &Opts) -> Status<List>>;
pub type Dyad = Rc<dyn Fn(List, List, &Opts) -> Status<List>>;

impl Function {
    pub fn new(
        m: Monad,
        d: Dyad,
        rm: u8,
        rd: [u8; 2],
        a: Arena,
        s: Scope,
        sym: Option<Symbol>,
    ) -> Self {
        Self {
            m,
            d,
            am: false,
            ad: false,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    pub fn scalar(sc: List, a: Arena, s: Scope) -> Self {
        let scr = sc.clone();
        Self {
            m: Rc::new(move |_, _| Ok(sc.clone())),
            d: Rc::new(move |_, _, _| Ok(scr.clone())),
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
        m: Monad,
        d: Dyad,
        rm: u8,
        rd: [u8; 2],
        am: bool,
        ad: bool,
        a: Arena,
        s: Scope,
        sym: Option<Symbol>,
    ) -> Self {
        Self {
            m,
            d,
            am,
            ad,
            s: sym,
            op: Opts { rm, rd, a, s },
        }
    }

    #[inline]
    pub fn monadic_call(&self, a: List) -> Status<List> {
        (self.m)(a, &self.op)
    }

    #[inline]
    pub fn dyadic_call(&self, a: List, b: List) -> Status<List> {
        (self.d)(a, b, &self.op)
    }

    // Applies new rank to monadic function
    #[inline]
    pub fn apply_rm(&mut self, rm: u8) {
        if self.am {
            self.op.rm = rm;
        } else {
            let m = self.m.clone();
            self.m = Rc::new(move |a, o| apply_loosem(&o.a, a, o.rm, |a| (m)(a, &o)));
            self.am = true;
        }
    }

    // Applies new rank to dyadic function
    #[inline]
    pub fn apply_rd(&mut self, rd: [u8; 2]) {
        if self.ad {
            self.op.rd = rd;
        } else {
            let d = self.d.clone();
            self.d = Rc::new(move |a, b, o| apply_loosed(&o.a, a, b, o.rd, |a, b| (d)(a, b, &o)));
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
    ($monad:expr; $dyad:expr; $arena:expr, $scope:expr) => {
        crate::function::Function::new(
            ::std::rc::Rc::new($monad),
            ::std::rc::Rc::new($dyad),
            99,
            [99, 99],
            $arena,
            $scope,
            None,
        )
    };

    ($rm:literal, $am:ident, $monad:expr; $rd:expr, $ad:ident, $dyad:expr; $a:ident, $s:ident) => {{
        crate::function::function!($rm, $am, $monad; $rd, $ad, $dyad; $a, $s, None)
    }};

    ($rm:literal, $am:ident, $monad:expr; $rd:expr, $ad:ident, $dyad:expr; $a:ident, $s:ident, $sym:expr) => {{
        crate::function::Function::new_applied(
            ::std::rc::Rc::new($monad),
            ::std::rc::Rc::new($dyad),
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
