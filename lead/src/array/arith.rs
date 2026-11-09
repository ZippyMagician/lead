use std::ops::*;

use crate::types::list::Array;
use crate::utils::error::err;
use crate::utils::symbols::*;

use num_traits::Zero;
use num_traits::ops::wrapping;
use ordered_float::{Float, OrderedFloat};

macro_rules! create_op_typed {
    ($t1:ty, $t2:ty, $t3:ty, $name:ident, $name2:ident, $mac:ident, |$a:ident, $b:ident| $body:block, $swap:block) => {
        #[inline(always)]
        pub fn $name($a: $t1, $b: $t2) -> $t3 {
            if $a.len() >= $b.len() {
                if $a.len() % $b.len() != 0 {
                    std::hint::cold_path();
                    return err!(Domain);
                }
                let rank = $a.rank();
                $mac!($b, |it| unsafe {
                    Array::try_alloc_iter(
                        rank,
                        $a.list()
                            .iter()
                            .zip(it.cycle())
                            .map(|($a, $b)| {
                                let t = $b;
                                let $b = *$a;
                                let $a = t;
                                $body
                            })
                            .take($a.len()),
                    )
                    .map(List::from)
                })
            } else {
                if $b.len() % $a.len() != 0 {
                    std::hint::cold_path();
                    return err!(Domain);
                }
                let rank = $b.rank();
                $mac!($b, |it| unsafe {
                    Array::try_alloc_iter(
                        rank,
                        it.zip($a.list().iter().cycle())
                            .map(|($a, $b)| {
                                let t = *$b;
                                let $b = $a;
                                let $a = t;
                                $body
                            })
                            .take($b.len()),
                    )
                    .map(List::from)
                })
            }
        }

        #[inline(always)]
        pub fn $name2($a: $t1, $b: $t2) -> $t3 {
            if $a.len() >= $b.len() {
                if $a.len() % $b.len() != 0 {
                    std::hint::cold_path();
                    return err!(Domain);
                }
                let rank = $a.rank();
                $mac!($b, |it| unsafe {
                    Array::try_alloc_iter(
                        rank,
                        $a.list()
                            .iter()
                            .zip(it.cycle())
                            .map(|($a, $b)| {
                                let t = $b;
                                let $b = *$a;
                                let $a = t;
                                $swap
                            })
                            .take($a.len()),
                    )
                    .map(List::from)
                })
            } else {
                if $b.len() % $a.len() != 0 {
                    std::hint::cold_path();
                    return err!(Domain);
                }
                let rank = $b.rank();
                $mac!($b, |it| unsafe {
                    Array::try_alloc_iter(
                        rank,
                        it.zip($a.list().iter().cycle())
                            .map(|($a, $b)| {
                                let t = *$b;
                                let $b = $a;
                                let $a = t;
                                $swap
                            })
                            .take($b.len()),
                    )
                    .map(List::from)
                })
            }
        }
    };
}

macro_rules! maybe_expand {
    (true, |$a:ident, $b:ident| $alli:expr, $allf:expr, $opi:expr, $opf:expr, $opc:expr) => {
        pub fn int_op($a: i64, $b: i64) -> Status<i64> {
            $opi
        }

        pub fn float_op($a: OrderedFloat<f64>, $b: OrderedFloat<f64>) -> Status<OrderedFloat<f64>> {
            $opf
        }

        pub fn char_op($a: u8, $b: u8) -> Status<u8> {
            $opc
        }

        pub fn typed_op<const A: u8, const B: u8>($a: List, $b: List) -> Status<List> {
            match (A, B) {
                (b'I', b'I') => {
                    let $a = unsafe { $a.arrayi_unchecked() };
                    let $b = unsafe { $b.arrayi_unchecked() };
                    $alli
                }
                (b'F', b'F') => {
                    let $a = unsafe { $a.arrayf_unchecked() };
                    let $b = unsafe { $b.arrayf_unchecked() };
                    $allf
                }
                (b'C', b'C') => unsafe {
                    let $a = $a.arrayc_unchecked();
                    let $b = $b.arrayc_unchecked();

                    if $a.len() > $b.len() {
                        if $a.len() % $b.len() != 0 {
                            std::hint::cold_path();
                            return err!(Domain);
                        }

                        Array::try_alloc_iter(
                            $a.rank(),
                            $a.list().iter().zip($b.list().iter().cycle()).map(|(&$a, &$b)| $opc),
                        )
                        .map(List::from)
                    } else if $a.len() == $b.len() {
                        let rank = if $a.dims() >= $b.dims() { $a.rank() } else { $b.rank() };
                        Array::try_alloc_iter(
                            rank,
                            $a.list().iter().zip($b.list().iter().cycle()).map(|(&$a, &$b)| $opc),
                        )
                        .map(List::from)
                    } else {
                        if $b.len() % $a.len() != 0 {
                            std::hint::cold_path();
                            return err!(Domain);
                        }

                        Array::try_alloc_iter(
                            $b.rank(),
                            $b.list().iter().zip($a.list().iter().cycle()).map(|(&$a, &$b)| $opc),
                        )
                        .map(List::from)
                    }
                },
                _ => unreachable!(),
            }
        }
    };

    (false, |$a:ident, $b:ident| $alli:expr, $allf:expr, $opi:expr, $opf:expr, $opc:expr) => {};
}

macro_rules! create_op_generic {
    ($name:ident, |$a:ident, $b:ident| $alli:expr, $allf:expr, $opi:expr, $opf:expr, $opc:expr, $cond:ident) => {
        pub(crate) mod $name {
            #![allow(unused_imports)]
            #![allow(dead_code)]
            use List::{Booleans as B, Boxes as X, Doubles as F, Integers as I, String as C};
            use ordered_float::{Float, OrderedFloat};
            use std::ops::*;
            use $crate::array::arith::WrappingAdd;
            use $crate::array::cast::Castable;
            use $crate::types::list::{Array, List};
            use $crate::types::scalar::Scalar;
            use $crate::utils::error::{Status, err};
            use $crate::utils::list::*;
            type Fl = OrderedFloat<f64>;

            create_op_typed!(
                Array<Fl>,
                List,
                Status<List>,
                float_op_partial,
                float_op_partial_swap,
                iter_floats,
                |$a, $b| { $opf },
                {
                    let t = $b;
                    let $b = $a;
                    let $a = t;
                    $opf
                }
            );
            create_op_typed!(
                Array<u8>,
                List,
                Status<List>,
                char_op_partial,
                char_op_partial_swap,
                iter_ints,
                |$a, $b| { $opc },
                {
                    let t = $b;
                    let $b = $a;
                    let $a = t;
                    $opc
                }
            );

            maybe_expand!($cond, |$a, $b| $alli, $allf, $opi, $opf, $opc);

            #[inline]
            pub fn generic_op($a: List, $b: List) -> Status<List> {
                if $a.is_empty() {
                    std::hint::cold_path();
                    return Ok($b);
                } else if $b.is_empty() {
                    std::hint::cold_path();
                    return Ok($a);
                }

                match ($a, $b) {
                    (X(_), _) | (_, X(_)) => {
                        std::hint::cold_path();
                        err!(Box)
                    }
                    (I($a), I($b)) => $alli,
                    (F($a), F($b)) => $allf,
                    (F($a), $b) => float_op_partial($a, $b),
                    ($a, F($b)) => float_op_partial_swap($b, $a),
                    (C($a), $b) => char_op_partial($a, $b),
                    ($a, C($b)) => char_op_partial_swap($b, $a),
                    ($a, $b) => {
                        let count = usize::max($a.len(), $b.len());
                        let rank = if $a.len() > $b.len() { $a.rank() } else { $b.rank() };
                        iter_ints!($a, |$a| iter_ints!($b, |$b| unsafe {
                            Array::try_alloc_iter(
                                rank,
                                $a.cycle().zip($b.cycle()).map(|($a, $b)| $opi).take(count),
                            )
                            .map(List::from)
                        }))
                    }
                }
            }
        }
    };
}

create_op_generic!(
    add,
    |a, b| Ok(I(a.add(b))),
    Ok(F(a.add(b))),
    Ok(a + b),
    Ok(a + b),
    {
        let hi = a as i64 + b as i64;
        if hi < 0 || hi > 255 {
            std::hint::cold_path();
            return err!(Value);
        }
        Ok(hi as u8)
    },
    true
);

create_op_generic!(
    wrapping_add,
    |a, b| Ok(I(a.wrapping_add(b))),
    Ok(F(a.wrapping_add(b))),
    Ok(a.wrapping_add(b)),
    Ok(a.wrapping_add(b)),
    {
        let a = (a & 255) as u8;
        let b = (b & 255) as u8;
        Ok(a.wrapping_add(b))
    },
    true
);

create_op_generic!(
    sub,
    |a, b| Ok(I(a.sub(b))),
    Ok(F(a.sub(b))),
    Ok(a - b),
    Ok(a - b),
    {
        let hi = a as i64 - b as i64;
        if hi < 0 || hi > 255 {
            std::hint::cold_path();
            return err!(Value);
        }
        Ok(hi as u8)
    },
    true
);

create_op_generic!(
    mul,
    |a, b| Ok(I(a.mul(b))),
    Ok(F(a.mul(b))),
    Ok(a * b),
    Ok(a * b),
    {
        let hi = a as i64 * b as i64;
        if hi < 0 || hi > 255 {
            std::hint::cold_path();
            return err!(Value);
        }
        Ok(hi as u8)
    },
    true
);

create_op_generic!(
    pow,
    |a, b| {
        let mut a = a.cast_f64().unwrap();
        let mut b = b.cast_f64().unwrap();
        crate::utils::symbols::scalar_op!(a, b,
            [l, r] = { l.powf(*r) }; { r.powf(l) }; { l.powf(r) };
        )
        .map(List::Doubles)
    },
    {
        let mut a = a;
        let mut b = b;
        crate::utils::symbols::scalar_op!(a, b,
            [l, r] = { l.powf(*r) }; { r.powf(l) }; { l.powf(r) };
        )
        .map(List::Doubles)
    },
    Ok(a.to_float()?.powf(b.to_float()?)),
    Ok(a.powf(b)),
    {
        let hi = (a as i64).pow(b as u32);
        if hi < 0 || hi > 255 {
            std::hint::cold_path();
            return err!(Value);
        }
        Ok(hi as u8)
    },
    false
);

create_op_generic!(
    div,
    |a, b| Ok(I(a.div(b))),
    Ok(F(a.div(b))),
    {
        if b == 0 {
            std::hint::cold_path();
            return err!(NaN);
        }
        Ok(a / b)
    },
    {
        if b == 0. {
            std::hint::cold_path();
            return err!(NaN);
        }
        Ok(a / b)
    },
    {
        if b == 0 {
            std::hint::cold_path();
            return err!(NaN);
        }

        let hi = a as i64 / b as i64;
        if hi < 0 || hi > 255 {
            std::hint::cold_path();
            return err!(Value);
        }
        Ok(hi as u8)
    },
    true
);

create_op_generic!(
    rem,
    |a, b| { Ok(I(a.rem(b))) },
    { Ok(F(a.rem(b))) },
    {
        if b == 0 {
            std::hint::cold_path();
            return err!(NaN);
        }
        Ok(a % b)
    },
    {
        if b == 0. {
            std::hint::cold_path();
            return err!(NaN);
        }
        Ok(a % b)
    },
    {
        if b == 0 {
            std::hint::cold_path();
            return err!(NaN);
        }

        let hi = a as i64 % b as i64;
        if hi < 0 || hi > 255 {
            std::hint::cold_path();
            return err!(Value);
        }
        Ok(hi as u8)
    },
    true
);

trait WrappingAdd {
    type Output;
    fn wrapping_add(self, rhs: Self) -> Self::Output;
}

impl WrappingAdd for OrderedFloat<f64> {
    type Output = Self;
    #[inline]
    fn wrapping_add(self, rhs: Self) -> Self {
        let sum = self + rhs;
        if sum.is_infinite() && self.is_finite() && rhs.is_finite() {
            if sum > OrderedFloat::zero() {
                (self - f64::MAX) + (rhs - f64::MAX)
            } else {
                (self + f64::MAX) + (rhs + f64::MAX)
            }
        } else {
            sum
        }
    }
}

macro_rules! impl_numeric_traits {
    ($($t:ty),*) => {$(
        impl WrappingAdd for $t {
            type Output = Self;
            #[inline(always)]
            fn wrapping_add(self, rhs: Self) -> Self {
                wrapping::WrappingAdd::wrapping_add(&self, &rhs)
            }
        }
    )*};
}

impl_numeric_traits![i64, u8];

macro_rules! impl_self_ops {
    ($($t:ty)|*) => {$(
        impl_self_ops! {
            $t, Add, add;
            $t, WrappingAdd, wrapping_add;
            $t, Sub, sub;
            $t, Mul, mul;
            $t, Div, div;
            $t, Rem, rem;
        }
    )*};

    ($($t:ty, $tr:ty, $name:ident);*;) => {$(
        impl $tr for Array<$t> {
            type Output = Array<$t>;
            fn $name(mut self, mut rhs: Self) -> Self::Output {
                scalar_op!(self, rhs, $name).unwrap()
            }
        }
    )*};
}

impl_self_ops![i64 | OrderedFloat<f64> | u8];

/*
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
}*/
