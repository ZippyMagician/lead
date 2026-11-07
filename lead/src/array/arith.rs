use std::ops::*;

use crate::types::list::{Array, List};
use crate::types::scalar::{self, Boxed, Scalar};
use crate::utils::error::{Status, err};
use crate::utils::list::*;
use crate::utils::symbols::*;

use List::{Booleans as B, Boxes as X, Doubles as F, Integers as I, String as C};

use ordered_float::OrderedFloat;

macro_rules! impl_self_ops {
    ($($t:ty)|*) => {$(
        impl_self_ops! {
            $t, Add, add;
            $t, Sub, sub;
            $t, Mul, mul;
            $t, Div, div;
            $t, Rem, rem;
        }
    )*};

    ($($t:ty, $tr:ty, $name:ident);*;) => {$(
        impl $tr for Array<$t> {
            type Output = Self;
            fn $name(mut self, mut rhs: Self) -> Self::Output {
                scalar_op!(self, rhs, $name).unwrap()
            }
        }
    )*};
}

impl_self_ops![i64 | OrderedFloat<f64> | u8];

#[inline]
pub fn add_generic(a: List, b: List) -> Status<List> {
    if a.is_empty() {
        std::hint::cold_path();
        return Ok(b);
    } else if b.is_empty() {
        std::hint::cold_path();
        return Ok(a);
    }

    match (a, b) {
        (X(_), _) | (_, X(_)) => {
            std::hint::cold_path();
            err!(Box)
        }
        (I(a), I(b)) => Ok(I(a.add(b))),
        (F(a), F(b)) => Ok(F(a.add(b))),
        (F(a), b) => add_float_list(a, b),
        (a, F(b)) => add_float_list(b, a),
        (C(a), b) => add_string(a, b),
        (a, C(b)) => add_string(b, a),
        (a, b) => {
            let count = usize::max(a.len(), b.len());
            let rank = if a.len() > b.len() { a.rank() } else { b.rank() };
            iter_ints!(a, |a| iter_ints!(b, |b| unsafe {
                Array::<i64>::alloc_iter(
                    rank,
                    a.cycle().zip(b.cycle()).map(|(l, r)| l + r).take(count),
                )
            }))
        }
    }
}

#[inline]
fn add_float_list(a: Array<OrderedFloat<f64>>, b: List) -> Status<List> {
    if a.len() >= b.len() {
        if a.len() % b.len() != 0 {
            std::hint::cold_path();
            return err!(Domain);
        }

        let rank = a.rank();
        iter_floats!(b, |it| unsafe {
            Array::<OrderedFloat<f64>>::alloc_iter(
                rank,
                a.list().iter().zip(it.cycle()).map(|(l, r)| l + r).take(a.len()),
            )
        })
    } else {
        if b.len() % a.len() != 0 {
            std::hint::cold_path();
            return err!(Domain);
        }

        let rank = b.rank();

        iter_floats!(b, |it| unsafe {
            Array::<OrderedFloat<f64>>::alloc_iter(
                rank,
                it.zip(a.list().iter().cycle()).map(|(l, r)| l + r).take(b.len()),
            )
        })
    }
}

#[inline]
fn add_string(a: Array<u8>, b: List) -> Status<List> {
    if a.len() >= b.len() {
        if a.len() % b.len() != 0 {
            std::hint::cold_path();
            return err!(Domain);
        }

        let rank = a.rank();
        iter_ints!(b, |it| unsafe {
            Array::<u8>::try_alloc_iter(
                rank,
                a.list()
                    .iter()
                    .zip(it.cycle())
                    .map(|(l, r)| {
                        let hi = *l as i64 + r;
                        if hi < 0 || hi > 255 {
                            std::hint::cold_path();
                            return err!(Value);
                        }
                        Ok(hi as u8)
                    })
                    .take(a.len()),
            )
            .map(List::String)
        })
    } else {
        if b.len() % a.len() != 0 {
            std::hint::cold_path();
            return err!(Domain);
        }

        let rank = b.rank();

        iter_ints!(b, |it| unsafe {
            Array::<u8>::try_alloc_iter(
                rank,
                it.zip(a.list().iter().cycle())
                    .map(|(r, l)| {
                        let hi = *l as i64 + r;
                        if hi < 0 || hi > 255 {
                            std::hint::cold_path();
                            return err!(Value);
                        }
                        Ok(hi as u8)
                    })
                    .take(b.len()),
            )
            .map(List::String)
        })
    }
}
