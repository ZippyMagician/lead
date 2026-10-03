use std::rc::Rc;

use crate::list::List;
use crate::scope::Scope;

#[derive(Clone)]
#[allow(dead_code)]
pub struct Function {
    // Monad
    m: Rc<dyn Fn(List, Option<Scope>) -> List>,
    // Dyad
    d: Rc<dyn Fn(List, List, Option<Scope>) -> List>,
    // Depth
    dp: Option<u8>,
    // Monadic rank
    rkm: u8,
    // Dyadic rank
    rkd: [u8; 2],
}

impl Function {
    pub fn new(
        m: Rc<dyn Fn(List, Option<Scope>) -> List>,
        d: Rc<dyn Fn(List, List, Option<Scope>) -> List>,
        rkm: u8,
        rkd: [u8; 2],
    ) -> Self {
        Self {
            m,
            d,
            dp: None,
            rkm,
            rkd,
        }
    }
}
