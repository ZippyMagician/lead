pub mod error;
pub mod function;
pub mod number;
pub mod symbols;

use std::cell::RefCell;
use std::rc::Rc;

use super::exec::Exec;
use super::function::Function;
use super::list::List;
use super::parse::Node;
use super::scalar::Scalar::{self as S};
use super::scope::Scope;
use function::{apply_loosed, apply_loosem};

use typed_arena;

pub(crate) type Arena = Rc<RefCell<typed_arena::Arena<List>>>;

// Check to ensure lists contain matching scalar values
#[inline]
pub fn compatible(list: &[S]) -> bool {
    if list.is_empty() {
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
        Assign { body, .. } if body.last().is_none_or(|n| is_instant(n, scope.clone())) => true,
        Literal(s) => scope.borrow().get(s).is_some_and(|m| m.is_instant()),
        LiteralInternal(_) => todo!("Literal internal"),
        Group(body) if body.last().is_none_or(|n| is_instant(n, scope.clone())) => true,
        _ => false,
    }
}

// Shorthand for this ridiculous code block
macro_rules! run_block {
    ($a:ident, $o:ident, $n:ident) => {{
        let mut s = $o.s.borrow_mut();
        s.set_instant("y", $a);
        drop(s);
        let mut e = Exec::new_imm($o.a.as_ref().unwrap().clone(), $o.s.clone());
        e.exec($n.clone())?;
        Ok(e.right.unwrap_or(List::empty()))
    }};

    ($a:ident, $b:ident, $o:ident, $n:ident) => {{
        let mut s = $o.s.borrow_mut();
        s.set_instant("x", $a);
        s.set_instant("y", $b);
        drop(s);
        let mut e = Exec::new_imm($o.a.as_ref().unwrap().clone(), $o.s.clone());
        e.exec($n.clone())?;
        Ok(e.right.unwrap_or(List::empty()))
    }};
}

#[inline]
pub fn generate_block(nodes: Vec<Node<'_>>, scope: Scope, arena: Arena) -> Function {
    // SAFETY: All inputs are leaked by the Lexer, and all Nodes take slices from that input.
    let n1: Vec<Node<'static>> = unsafe { std::mem::transmute(nodes) };
    let n2 = n1.clone();

    let mut f = Function::new(
        Rc::new(move |a, o| {
            if o.rm as usize >= a.dims() {
                run_block!(a, o, n1)
            } else {
                apply_loosem(a, o.rm, |v| run_block!(v, o, n1))
            }
        }),
        Rc::new(move |a, b, o| {
            if o.rd[0] as usize >= a.dims() && o.rd[1] as usize >= b.dims() {
                run_block!(a, b, o, n2)
            } else {
                apply_loosed(a, b, o.rd, |l, r| run_block!(l, r, o, n2))
            }
        }),
        99,
        [99, 99],
        scope,
    );
    f.assign_arena(arena);
    f
}
