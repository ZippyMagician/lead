#![allow(unused_variables)]

use std::ops::*;

use crate::function::Opts;
use crate::list::List;
use crate::scalar::Scalar::{self, Boolean, Char, Float, Number};
use crate::utils::error::{Status, err};
use crate::utils::function::*;
use crate::utils::symbols::{for_0, scalar_op};

use smallvec::{SmallVec, smallvec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Symbol {
    Plus,
    Dash,
    Star,
    Percent,
    Pipe,
    Tilde,
    LeftArrow,
    Dollar,
}

// plusm | + | 0 | Absolute value / Cast to int
pub(super) fn abs(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Number(n) => Number(n.abs()),
        Float(n) => Float(n.abs()),
        Boolean(n) => Number(*n as i32),
        Char(c) => Number(*c as u32 as i32),
        Scalar::Box(ptr) => Number(ptr.addr() as i32),
    });
    Ok(a)
}

// plusd | + | 0, 0 | Addition
pub(super) fn add(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, add)
}

// dashm | - | 0 | Negate / Swap case / Boolean not
pub(super) fn negate(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char(c) if c.is_lowercase() => Char(c.to_ascii_uppercase()),
        Char(c) => Char(c.to_ascii_lowercase()),
        Number(n) => Number(-*n),
        Float(n) => Float(-*n),
        Boolean(b) => Boolean(!*b),
        Scalar::Box(ptr) => return err!(Box),
    });
    Ok(a)
}

// dashd | - | 0, 0 | Subtraction
pub(super) fn sub(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, sub)
}

// starm | * | 0 | Sign / Case
pub(super) fn signum(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char(c) if c.is_lowercase() => Number(-1),
        Char(c) if c.is_uppercase() => Number(1),
        Char(_) => Number(0),
        Number(n) => Number(n.signum()),
        Float(n) => Number(n.signum() as i32),
        Boolean(b) if *b => Number(1),
        Boolean(_) => Number(0),
        Scalar::Box(ptr) => Number(0),
    });
    Ok(a)
}

// stard | * | 0, 0 | Multiplication
pub(super) fn mul(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, mul)
}

// percentm | % | 0 | Reciprocal
pub(super) fn reciprocal(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char('\0') | Number(0) | Float(0.0) | Boolean(false) => {
            std::hint::cold_path();
            return err!(NaN);
        }
        Char(c) => Float(f32::recip(*c as u32 as f32)),
        Number(n) => Float(f32::recip(*n as f32)),
        Float(f) => Float(f32::recip(*f)),
        Boolean(_) => Float(1.0),
        _ => return err!(NaN),
    });
    Ok(a)
}

// percentd | % | 0, 0 | Division
pub(super) fn div(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, div)
}

// pipem | | | 0 | Boolean not
pub(super) fn not(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char(c) if *c == '\0' => Boolean(true),
        Number(n) if *n == 0 => Boolean(true),
        Float(f) if *f == 0. => Boolean(true),
        Boolean(b) => Boolean(!*b),
        Scalar::Box(_) => return err!(Box),
        _ => Boolean(false),
    });
    Ok(a)
}

// piped | | | 0, 0 | Residue (Modulus)
pub(super) fn residue(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(b, a, rem)
}

// tildem | ~ | 0 | 1-range / Rising alphabet
pub(super) fn range(mut a: List, o: &Opts) -> Status<List> {
    if a.is_scalar() {
        List::range(Scalar::Number(1), a.list()[0])
    } else {
        for_0!(a; [v] = Scalar::boxed(
            &o.a, List::range(Scalar::Number(1), *v)?)?
        );
        Ok(a)
    }
}

// tilded | ~ | 99, 0 | Index
pub(super) fn index(a: List, b: List, o: &Opts) -> Status<List> {
    apply_loosed(&o.a, a, b, o.rd, |a, b| {
        if a.is_empty() {
            std::hint::cold_path();
            return err!(Domain);
        }

        let index = b.list()[0];
        if !index.is_box() {
            let i = index.to_num()? as usize;
            let rank: SmallVec<[usize; 4]> = if a.dims() == 0 {
                smallvec![1]
            } else {
                a.rank().iter().copied().take(a.dims() - 1).collect()
            };
            let r: usize = rank.iter().copied().product();
            if i >= unsafe { *a.rank().last().unwrap_unchecked() } {
                return err!(Domain);
            }
            let values = a.list()[i * r..i * r + r].to_vec();
            List::mat(values, a.dims() - 1, rank)
        } else {
            let index = unsafe { index.unbox_unchecked() }
                .list()
                .iter()
                .map(|n| n.to_num().map(|n| n as usize))
                .collect::<Status<Vec<usize>>>()?;

            if a.dims() < index.len() {
                return err!(Domain);
            }
            let rank = a.rank();

            let mut i = 0;
            let mut prefix = 1;
            for j in 0..index.len() {
                i += index[j] * prefix;
                prefix *= rank[rank.len() - j - 1];
            }

            let rank: SmallVec<[usize; 4]> =
                rank.iter().copied().take(a.dims() - index.len()).collect();
            let ct: usize = rank.iter().copied().product();
            if ct == 0 {
                Ok(List::scalar(a.list()[i]))
            } else {
                List::mat(a.list()[i..i + ct].to_vec(), a.dims() - index.len(), rank)
            }
        }
    })
}

// leftarrowm | < | 99 | Box
pub(crate) fn boxed(a: List, o: &Opts) -> Status<List> {
    Ok(List::scalar(Scalar::boxed(&o.a, a)?))
}

// leftarrowd | < | 0, 0 | Less than
pub(crate) fn lt(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op! {
        a, b, [l, r] =
        { Scalar::Boolean(*l < *r) };
        { Scalar::Boolean(l < *r)  };
        { Scalar::Boolean(*l < r)  };
    }
}

// dollarm | $ | 99 | Shape
pub(crate) fn shape(a: List, _: &Opts) -> Status<List> {
    List::from_vec(a.rank().iter().map(|n| Scalar::Number(*n as i32)).collect())
}

// dollard | $ | 1, 99 | Reshape
pub(crate) fn reshape(a: List, b: List, o: &Opts) -> Status<List> {
    apply_loosed(&o.a, a, b, o.rd, |a, b| {
        let rank = a
            .list()
            .iter()
            .map(|n| n.to_num().map(|n| n as usize))
            .collect::<Status<SmallVec<[usize; 4]>>>()?;
        b.with_rank(&rank)
    })
}
