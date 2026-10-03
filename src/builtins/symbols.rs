#![allow(unused_variables)]

use std::ops::*;

use crate::function::Opts;
use crate::list::List;
use crate::scalar::{Scalar, scalar_op};
use crate::utils::error::{Status, err};

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
    let values = a.list_mut();
    for v in values.iter_mut() {
        *v = match v {
            Scalar::Number(n) => Scalar::Number(n.abs()),
            Scalar::Float(n) => Scalar::Float(n.abs()),
            Scalar::Boolean(n) => Scalar::Number(*n as i32),
            Scalar::Box(ptr) => Scalar::Number(ptr.addr() as i32),
        }
    }
    Ok(a)
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
