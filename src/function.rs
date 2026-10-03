use std::rc::Rc;

use crate::list::List;
use crate::utils::error::Status;

#[derive(Clone)]
#[allow(dead_code)]
pub struct Function {
    // Monad
    m: Rc<dyn Fn(List, &Opts) -> Status<List>>,
    // Dyad
    d: Rc<dyn Fn(List, List, &Opts) -> Status<List>>,
    // Opts
    op: Opts,
    // Monadic rank
    rkm: u8,
    // Dyadic rank
    rkd: [u8; 2],
}

impl Function {
    pub fn new(
        m: Rc<dyn Fn(List, &Opts) -> Status<List>>,
        d: Rc<dyn Fn(List, List, &Opts) -> Status<List>>,
        rkm: u8,
        rkd: [u8; 2],
    ) -> Self {
        Self {
            m,
            d,
            op: Opts::empty(),
            rkm,
            rkd,
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
}

// Options for functions, e.g. rank override when applicable
#[derive(Clone, Debug)]
#[repr(C)]
pub struct Opts {
    pub ro: Option<[u8; 2]>,
}

impl Opts {
    #[inline]
    pub fn empty() -> Self {
        Self { ro: None }
    }
}
