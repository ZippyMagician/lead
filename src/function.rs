use std::rc::Rc;

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
    am: bool,
    ad: bool,
    // Opts
    op: Opts,
}

pub type Monad = Rc<dyn Fn(List, &Opts) -> Status<List>>;
pub type Dyad = Rc<dyn Fn(List, List, &Opts) -> Status<List>>;

impl Function {
    pub fn new(m: Monad, d: Dyad, rm: u8, rd: [u8; 2], a: Arena, s: Scope) -> Self {
        Self {
            m,
            d,
            am: false,
            ad: false,
            op: Opts { rm, rd, a, s },
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
    ) -> Self {
        Self {
            m,
            d,
            am,
            ad,
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

// Options for functions, e.g. rank override when applicable
#[derive(Clone)]
#[repr(C)]
pub struct Opts {
    pub rm: u8,
    pub rd: [u8; 2],
    pub a: Arena,
    pub s: Scope,
}
