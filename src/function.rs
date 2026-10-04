use std::rc::Rc;

use crate::list::List;
use crate::utils::Arena;
use crate::utils::error::Status;

#[derive(Clone)]
pub struct Function {
    // Monad
    m: Rc<dyn Fn(List, &Opts) -> Status<List>>,
    // Dyad
    d: Rc<dyn Fn(List, List, &Opts) -> Status<List>>,
    // Opts
    op: Opts,
}

impl Function {
    pub fn new(
        m: Rc<dyn Fn(List, &Opts) -> Status<List>>,
        d: Rc<dyn Fn(List, List, &Opts) -> Status<List>>,
        rm: u8,
        rd: [u8; 2],
    ) -> Self {
        Self {
            m,
            d,
            op: Opts { rm, rd, a: None },
        }
    }

    #[inline]
    pub fn assign_arena(&mut self, a: Arena) {
        self.op.a = Some(a);
    }

    #[inline]
    pub fn monadic_call(&self, a: List) -> Status<List> {
        (self.m)(a, &self.op)
    }

    #[inline]
    pub fn dyadic_call(&self, a: List, b: List) -> Status<List> {
        (self.d)(a, b, &self.op)
    }
}

// Options for functions, e.g. rank override when applicable
#[derive(Clone)]
#[repr(C)]
pub struct Opts {
    pub rm: u8,
    pub rd: [u8; 2],
    pub a: Option<Arena>,
}
