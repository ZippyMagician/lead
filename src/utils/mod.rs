pub mod error;
pub mod function;
pub mod number;
pub mod symbols;

use std::cell::RefCell;
use std::fmt::Write;
use std::hint::assert_unchecked;
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
pub fn is_instant(node: &Node<'_>, scope: &Scope) -> bool {
    use Node::*;
    match node {
        Primitive(_) => true,
        PrimitiveString(_) => true,
        PrimitiveBoxes(_) => true,
        Assign { body, .. } if body.last().is_none_or(|n| is_instant(n, scope)) => true,
        Literal(s) => scope.borrow().get(s).is_some_and(|m| m.is_instant()),
        LiteralInternal(_) => todo!("Literal internal"),
        Group(body) if body.last().is_none_or(|n| is_instant(n, scope)) => true,
        _ => false,
    }
}

// Shorthand for this ridiculous code block
macro_rules! run_block {
    ($a:ident, $o:ident, $n:ident) => {{
        let sc = $o.s.clone();
        let mut s = sc.borrow_mut();
        s.set_instant("y", $a);
        // s.set_function("v", generate_block($n.clone(), $o.s.clone(), $o.a.clone()));
        drop(s);
        let mut e = Exec::new_imm($o.a.clone(), $o.s.clone());
        e.exec($n.clone())?;
        Ok(e.right.unwrap_or(List::empty()))
    }};

    ($a:ident, $b:ident, $o:ident, $n:ident) => {{
        let sc = $o.s.clone();
        let mut s = sc.borrow_mut();
        s.set_instant("x", $a);
        s.set_instant("y", $b);
        // s.set_function("v", generate_block($n.clone(), $o.s.clone(), $o.a.clone()));
        drop(s);
        let mut e = Exec::new_imm($o.a.clone(), $o.s.clone());
        e.exec($n.clone())?;
        Ok(e.right.unwrap_or(List::empty()))
    }};
}

#[inline]
pub fn generate_block(nodes: Vec<Node<'_>>, scope: Scope, arena: Arena) -> Function {
    // SAFETY: All inputs are leaked by the Lexer, and all Nodes take slices from that input.
    let n1: Vec<Node<'static>> = unsafe { std::mem::transmute(nodes) };
    let n2 = n1.clone();

    crate::function::function!(
        99, true, move |a, o| {
            if o.rm as usize >= a.dims() {
                run_block!(a, o, n1)
            } else {
                apply_loosem(&o.a, a, o.rm, |v| run_block!(v, o, n1))
            }
        };
        [99, 99], true, move |a, b, o| {
            if o.rd[0] as usize >= a.dims() && o.rd[1] as usize >= b.dims() {
                run_block!(a, b, o, n2)
            } else {
                apply_loosed(&o.a, a, b, o.rd, |l, r| run_block!(l, r, o, n2))
            }
        };
        arena,
        scope,
        None
    )
}

// const BOXES: &str = "┌┬┐├┼┤└┴┘│─";
const BOXES_TL: &str = "┌├┌├┌├┌├┬┼┬┼┬┼┬┼";
const BOXES_TR: &str = "┐┤┬┼┐┤┬┼┐┤┬┼┐┤┬┼";
const BOXES_BR: &str = "┘┘┴┴┤┤┼┼┘┘┴┴┤┤┼┼";
const BOXES_BL: &str = "└└└└├├├├┴┴┴┴┼┼┼┼";

#[inline(always)]
fn g(b: &'static str, mask: u8) -> char {
    unsafe {
        assert_unchecked((mask as usize) < b.len());
        b.chars().nth(mask as usize).unwrap_unchecked()
    }
}

pub(crate) fn display_boxed_lines(
    lines: Vec<&str>,
    mx: Option<usize>,
    my: Option<usize>,
    mask: u8,
) -> Result<Vec<String>, std::fmt::Error> {
    //     ╓─╖╙─╜║┌┬┐├┼┤└┴┘│─
    let y = lines.len();
    let x = if y == 0 { 0 } else { lines[0].chars().count() };

    let my = my.unwrap_or(y);
    let mx = mx.unwrap_or(x);
    let mut collected = Vec::with_capacity(my + 4);
    for _ in 0..my + 2 {
        collected.push(String::with_capacity(mx + 4));
    }

    collected[0].push(g(BOXES_TL, mask));
    collected[my + 1].push(g(BOXES_BL, mask));
    for _ in 0..mx + 2 {
        collected[0].push('─');
        collected[my + 1].push('─');
    }
    collected[0].push(g(BOXES_TR, mask));
    collected[my + 1].push(g(BOXES_BR, mask));

    let yoff = (my - lines.len()) / 2;
    for i in 0..my {
        let f = &mut collected[1 + i];
        let line = if yoff > i || lines.len() + yoff <= i {
            ""
        } else {
            &lines[i - yoff]
        };
        let l = line.chars().count();
        let xoff = (mx - l + 2) / 2;
        if (mx - l) % 2 != 0 {
            write!(f, "│{}{}{}│", " ".repeat(xoff), line, " ".repeat(xoff + 1))?;
        } else {
            write!(f, "│{0}{1}{0}│", " ".repeat(xoff), line)?;
        }
    }

    Ok(collected)
}

// Display boxed value
#[inline]
pub(crate) fn display_boxed<T: AsRef<str>>(
    inner: T,
    mx: Option<usize>,
    my: Option<usize>,
) -> Result<String, std::fmt::Error> {
    //     private boxes: string = "╓─╖╙─╜║┌┬┐├┼┤└┴┘│─";
    let inner = inner.as_ref().trim_end();
    let lines = inner.split('\n');

    display_boxed_lines(lines.collect(), mx, my, 0b0000).map(|n| n.join("\n"))
}
