use std::cmp::Ordering;

use crate::function::Opts;
use crate::types::list::{Array, List};
use crate::types::scalar::{Boxed, Scalar};

use crate::utils::error::{Status, err};
use crate::utils::function::apply_loosed;
use crate::utils::list::*;
use crate::utils::symbols::scalar_map;

use num_traits::Zero;
use ordered_float::OrderedFloat;

impl<T, U> PartialOrd<Array<U>> for Array<T>
where
    T: Scalar + OrderedScalar + OrderedScalar<U>,
    U: Scalar,
{
    fn partial_cmp(&self, o: &Array<U>) -> Option<Ordering> {
        Some(self.dims().cmp(&o.dims()).then(self.rank().cmp(&o.rank())).then_with(|| {
            let l = self.list();
            let o = o.list();

            for (l, r) in l.iter().zip(o) {
                if !l.eq_scalar(r) {
                    return l.cmp_scalar(r);
                }
            }
            Ordering::Equal
        }))
    }
}

impl<T: Scalar, U: Scalar> PartialEq<Array<U>> for Array<T>
where
    T: OrderedScalar + OrderedScalar<U>,
{
    fn eq(&self, o: &Array<U>) -> bool {
        self.rank() == o.rank() && self.list().iter().zip(o.list()).all(|(l, r)| l.eq_scalar(r))
    }
}

impl<T: Scalar + OrderedScalar> Eq for Array<T> {}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        with_list!(self, |a| with_list!(other, |b| a == b))
    }
}
impl Eq for List {}

impl PartialOrd for List {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for List {
    fn cmp(&self, other: &Self) -> Ordering {
        with_list!(self, |a| with_list!(other, |b| a.partial_cmp(b))).unwrap()
    }
}

pub trait OrderedScalar<T: Scalar = Self>: Scalar {
    fn cmp_scalar(&self, o: &T) -> Ordering;

    fn eq_scalar(&self, o: &T) -> bool;
}

/*impl<T: Scalar> OrderedScalar<T> for T {
    #[inline(always)]
    fn cmp_scalar(&self, o: &T) -> Ordering {
        self.cmp(&o)
    }

    #[inline(always)]
    fn eq_scalar(&self, o: &T) -> bool {
        *self == *o
    }
}*/

/* Based on Singeli program by Dzaima
fn cmp_src64(a:i64, b:f64) : i8 = {
  af := int_to_float{f64, a, 'rtn'}
  if (af == b) af = int_to_float{f64, a, 'rtp'}
  cmp2{af, b}
}

fn cmp_tgt64_2(i:i64, f:f64) : i8 = {
  lo := cast_i{i32, i}
  lo &= ((1<<31)-1) | (lo-1)
  hi := i - promote{i64, lo}
  d := f - f64~~(1<<31) * int_to_float{f64, (hi>>31) & ((i>>31) | ((1<<33)-1))}
  cmp2{int_to_float{f64, lo}, d}
}
*/
#[inline(always)]
fn cmp_i64_f64(i: i64, f: f64) -> (f64, f64) {
    let mut lo = i as i32;
    lo &= i32::MAX | lo.wrapping_sub(1);
    let hi = i.wrapping_sub(lo as i64);
    let mask = (i >> 31) | ((1i64 << 33) - 1);
    let k = (hi >> 31) & mask;
    let d = f - 2147483648.0 * (k as f64);
    let a = lo as f64;
    (a, d)
}

impl OrderedScalar<OrderedFloat<f64>> for i64 {
    #[inline]
    fn cmp_scalar(&self, o: &OrderedFloat<f64>) -> Ordering {
        // SAFETY: Ordering is `#[repr(i8)]` with -1, 0, 1
        let (a, b) = cmp_i64_f64(*self, o.into_inner());
        unsafe { std::mem::transmute((a > b) as i8 - (a < b) as i8) }
    }

    #[inline]
    fn eq_scalar(&self, o: &OrderedFloat<f64>) -> bool {
        let (a, b) = cmp_i64_f64(*self, o.into_inner());
        a == b
    }
}

impl OrderedScalar<i64> for OrderedFloat<f64> {
    #[inline]
    fn cmp_scalar(&self, o: &i64) -> Ordering {
        let (a, b) = cmp_i64_f64(*o, self.into_inner());
        unsafe { std::mem::transmute((b > a) as i8 - (b < a) as i8) }
    }

    #[inline]
    fn eq_scalar(&self, o: &i64) -> bool {
        o.eq_scalar(self)
    }
}

impl OrderedScalar for Boxed {
    #[inline]
    fn cmp_scalar(&self, o: &Boxed) -> Ordering {
        self.addr().cmp(&o.addr())
    }

    #[inline]
    fn eq_scalar(&self, o: &Boxed) -> bool {
        self.addr() == o.addr()
    }
}

impl OrderedScalar for OrderedFloat<f64> {
    #[inline(always)]
    fn cmp_scalar(&self, o: &OrderedFloat<f64>) -> Ordering {
        self.cmp(o)
    }

    #[inline(always)]
    fn eq_scalar(&self, o: &OrderedFloat<f64>) -> bool {
        self == o
    }
}

impl OrderedScalar for bool {
    #[inline(always)]
    fn cmp_scalar(&self, o: &bool) -> Ordering {
        self.cmp(o)
    }

    #[inline(always)]
    fn eq_scalar(&self, o: &bool) -> bool {
        *self == *o
    }
}

impl OrderedScalar<bool> for OrderedFloat<f64> {
    #[inline]
    fn cmp_scalar(&self, o: &bool) -> Ordering {
        (*self != OrderedFloat::zero()).cmp(o)
    }

    #[inline]
    fn eq_scalar(&self, o: &bool) -> bool {
        (*self != OrderedFloat::zero()) == *o
    }
}

impl OrderedScalar<OrderedFloat<f64>> for bool {
    #[inline]
    fn cmp_scalar(&self, o: &OrderedFloat<f64>) -> Ordering {
        self.cmp(&(*o != OrderedFloat::zero()))
    }

    #[inline]
    fn eq_scalar(&self, o: &OrderedFloat<f64>) -> bool {
        *self == (*o != OrderedFloat::zero())
    }
}

impl OrderedScalar<OrderedFloat<f64>> for u8 {
    #[inline]
    fn cmp_scalar(&self, o: &OrderedFloat<f64>) -> Ordering {
        OrderedFloat(*self as i32 as f64).cmp(&o)
    }
    #[inline]
    fn eq_scalar(&self, o: &OrderedFloat<f64>) -> bool {
        OrderedFloat(*self as i32 as f64) == *o
    }
}
impl OrderedScalar<u8> for OrderedFloat<f64> {
    #[inline]
    fn cmp_scalar(&self, o: &u8) -> Ordering {
        self.cmp(&OrderedFloat(*o as i32 as f64))
    }
    #[inline]
    fn eq_scalar(&self, o: &u8) -> bool {
        *self == OrderedFloat(*o as i32 as f64)
    }
}

macro_rules! impl_numeric_ordering {
    ($t1:ty; $t2:ty) => {
        impl OrderedScalar<$t1> for $t2 {
            #[inline]
            fn cmp_scalar(&self, o: &$t1) -> Ordering {
                (*self as i64).cmp(&(*o as i64))
            }

            #[inline]
            fn eq_scalar(&self, o: &$t1) -> bool {
                *self as i64 == *o as i64
            }
        }
    };

    ([bool] $($t:ty),*) => {$(
        impl OrderedScalar<bool> for $t {
            #[inline]
            fn cmp_scalar(&self, o: &bool) -> Ordering {
                (*self != 0).cmp(o)
            }

            #[inline]
            fn eq_scalar(&self, o: &bool) -> bool {
                (*self != 0) == *o
            }
        }

        impl OrderedScalar<$t> for bool {
            #[inline]
            fn cmp_scalar(&self, o: &$t) -> Ordering {
                self.cmp(&(*o != 0))
            }

            #[inline]
            fn eq_scalar(&self, o: &$t) -> bool {
                *self == (*o != 0)
            }
        }
        )*};

    ($($t:ty),*) => {$(
        impl_numeric_ordering!(u8; $t);
        impl_numeric_ordering!(i64; $t);
    )*};
}

macro_rules! impl_box_ordering {
    ($($t:ty),*) => {$(
        impl OrderedScalar<Boxed> for $t {
            #[inline]
            fn cmp_scalar(&self, _: &Boxed) -> Ordering {
                Ordering::Less
            }

            #[inline]
            fn eq_scalar(&self, _: &Boxed) -> bool {
                false
            }
        }

        impl OrderedScalar<$t> for Boxed {
            #[inline]
            fn cmp_scalar(&self, _: &$t) -> Ordering {
                Ordering::Greater
            }

            #[inline]
            fn eq_scalar(&self, _: &$t) -> bool {
                false
            }
        }
    )*};
}

impl_numeric_ordering![u8, i64];
impl_numeric_ordering!([bool] u8, i64);
impl_box_ordering![bool, u8, i64, OrderedFloat<f64>];

macro_rules! impl_symbol_ordering {
    ($name:ident, $($t:tt)*) => {
        pub(crate) fn $name(a: List, b: List, o: &Opts) -> Status<List> {
            use List::*;
            if o.rd == [0, 0] {
                match (a, b) {
                    (Integers(a), Integers(b)) => {
                        scalar_map!(a, b,
                            [l, r] = { *l $($t)* *r }; { r $($t)* *l }; { *l $($t)* r };
                        )
                        .map(List::Booleans)
                    }
                    (Doubles(a), Doubles(b)) => {
                        scalar_map!(a, b,
                            [l, r] = { *l $($t)* *r }; { r $($t)* *l }; { *l $($t)* r };
                        )
                        .map(List::Booleans)
                    }
                    (String(a), String(b)) => {
                        scalar_map!(a, b,
                            [l, r] = { *l $($t)* *r }; { r $($t)* *l }; { *l $($t)* r };
                        )
                        .map(List::Booleans)
                    }
                    (a, b) => apply_loosed(&o.a, a, b, o.rd, |a, b| Ok(Array::scalar(a $($t)* b))),
                }
            } else {
                apply_loosed(&o.a, a, b, o.rd, |a, b| Ok(Array::scalar(a $($t)* b)))
            }
        }
    };
}

impl_symbol_ordering!(lt, <);
impl_symbol_ordering!(lte, <=);
impl_symbol_ordering!(gt, >);
impl_symbol_ordering!(gte, >=);
impl_symbol_ordering!(eq, ==);

