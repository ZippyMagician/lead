#![allow(unused_variables)]

use std::ops::*;

use crate::function::Opts;
use crate::list::List;
use crate::scalar::Scalar::{self, Boolean, Float, Number};
use crate::scalar::scalar_op;
use crate::utils::error::{Status, err};
use crate::utils::symbols::foreach_0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Symbol {
    Plus,
    Dash,
    Star,
    Percent,
    Pipe,
}

// plusm | + | 0 | Absolute value / Cast to int
pub(super) fn abs(mut a: List, _: &Opts) -> Status<List> {
    foreach_0!(map; a; [v] match v {
        Number(n) => Number(n.abs()),
        Float(n) => Float(n.abs()),
        Boolean(n) => Number(*n as i32),
        Scalar::Box(ptr) => Number(ptr.addr() as i32),
    });
}

// plusd | + | 0, 0 | Addition
pub(super) fn add(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, add)
}

// dashm | - | 0 | Negate / Swap case
pub(super) fn negate(a: List, _: &Opts) -> Status<List> {
    todo!()
}

// dashd | - | 0, 0 | Subtraction
pub(super) fn sub(a: List, b: List, _: &Opts) -> Status<List> {
    todo!()
}

// starm | * | 0 | Sign
pub(super) fn signum(a: List, _: &Opts) -> Status<List> {
    todo!()
}

// stard | * | 0, 0 | Multiplication
pub(super) fn mul(a: List, b: List, _: &Opts) -> Status<List> {
    todo!()
}

// percentm | % | 0 | Reciprocal
pub(super) fn reciprocal(a: List, _: &Opts) -> Status<List> {
    todo!()
}

// percentd | % | 0, 0 | Division
pub(super) fn div(a: List, b: List, _: &Opts) -> Status<List> {
    todo!()
}

// pipem | | | 0 | Boolean not
pub(super) fn not(a: List, _: &Opts) -> Status<List> {
    todo!()
}

// piped | | | 0, 0 | Residue (Modulus)
pub(super) fn residue(a: List, b: List, _: &Opts) -> Status<List> {
    todo!()
}
