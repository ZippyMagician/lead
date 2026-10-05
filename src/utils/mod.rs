pub mod error;
pub mod function;
pub mod number;
pub mod symbols;

use std::cell::RefCell;
use std::rc::Rc;

use super::list::List;
use super::parse::Node;
use super::scalar::Scalar::{self as S};
use super::scope::Scope;

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

#[inline]
pub fn is_instant(node: &Node<'_>, scope: Scope) -> bool {
    use Node::*;
    match node {
        Primitive(_) => true,
        PrimitiveString(_) => true,
        PrimitiveBoxes(_) => true,
        Assign { body, .. } if body.last().map_or(true, |n| is_instant(n, scope.clone())) => true,
        Literal(s) => scope.borrow().get(s).map_or(false, |m| m.is_instant()),
        LiteralInternal(_) => todo!("Literal internal"),
        Group(body) if body.last().map_or(true, |n| is_instant(n, scope.clone())) => true,
        _ => false,
    }
}
