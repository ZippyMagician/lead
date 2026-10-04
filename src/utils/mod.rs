pub mod error;
pub mod number;
pub mod symbols;

use std::cell::RefCell;
use std::rc::Rc;

use super::list::List;
use super::scalar::Scalar::{self as S};

use typed_arena;

pub(crate) type Arena = Rc<RefCell<typed_arena::Arena<List>>>;

// Check to ensure lists contain matching scalar values
#[inline]
pub fn compatible(list: &[S]) -> bool {
    if list.len() == 0 {
        return true;
    }

    let first = unsafe { list.get_unchecked(0) };

    for i in list.iter().skip(1) {
        match (first, i) {
            (S::Box(_), S::Box(_)) => {}
            (S::Box(_), _) | (_, S::Box(_)) => return false,
            (S::Char(_), S::Char(_)) => {}
            (S::Char(_), _) | (_, S::Char(_)) => return false,
            (S::Boolean(_), S::Boolean(_)) => {}
            (S::Boolean(_), _) | (_, S::Boolean(_)) => return false,
            _ => {}
        }
    }

    true
}
