use std::cmp::Ordering;

use crate::types::list::Array;
use crate::types::scalar::{Boxed, Scalar};

use ordered_float::OrderedFloat;

impl<T: Scalar> PartialOrd for Array<T>
where
    T: OrderedScalar<T>,
{
    fn partial_cmp(&self, o: &Array<T>) -> Option<Ordering> {
        Some(self.cmp(&o))
    }
}

impl<T: Scalar> PartialEq for Array<T>
where
    T: OrderedScalar<T>,
{
    fn eq(&self, o: &Array<T>) -> bool {
        self.rank() == o.rank() && self.list().iter().zip(o.list()).all(|(l, r)| l.eq_scalar(r))
    }
}

impl<T: Scalar> Ord for Array<T>
where
    T: OrderedScalar<T>,
{
    fn cmp(&self, o: &Array<T>) -> Ordering {
        self.dims().cmp(&o.dims()).then(self.rank().cmp(&o.rank())).then_with(|| {
            let l = self.list();
            let o = o.list();

            for (l, r) in l.iter().zip(o) {
                if l != r {
                    return l.cmp_scalar(&r);
                }
            }
            Ordering::Equal
        })
    }
}

impl<T: Scalar + OrderedScalar<T>> Eq for Array<T> {}

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
    fn cmp_scalar(&self, _: &Boxed) -> Ordering {
        Ordering::Equal
    }

    #[inline]
    fn eq_scalar(&self, _: &Boxed) -> bool {
        true
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

macro_rules! impl_float_ordering {
    ($($t:ty),*) => {$(
        impl OrderedScalar<OrderedFloat<f64>> for $t {
            #[inline]
            fn cmp_scalar(&self, o: &OrderedFloat<f64>) -> Ordering {
                OrderedFloat(*self as i32 as f64).cmp(&o)
            }

            #[inline]
            fn eq_scalar(&self, o: &OrderedFloat<f64>) -> bool {
                OrderedFloat(*self as i32 as f64) == *o
            }
        }

        impl OrderedScalar<$t> for OrderedFloat<f64> {
            #[inline]
            fn cmp_scalar(&self, o: &$t) -> Ordering {
                self.cmp(&OrderedFloat(*o as i32 as f64))
            }

            #[inline]
            fn eq_scalar(&self, o: &$t) -> bool {
                *self == OrderedFloat(*o as i32 as f64)
            }
        }
    )*};
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

impl_float_ordering![bool, u8];
impl_numeric_ordering![u8, i64];
impl_numeric_ordering!([bool] u8, i64);
impl_box_ordering![bool, u8, i64, OrderedFloat<f64>];

// impl PartialEq for Scalar {
//     #[inline]
//     fn eq(&self, o: &Self) -> bool {
//         use Scalar::*;
//         match (&self, &o) {
//             (Number(l), Number(r)) => *l == *r,
//             (Number(l), Float(r)) => OrderedFloat(*l as f64) == *r,
//             (Number(l), Boolean(b)) if *b => *l == 1,
//             (Number(l), Boolean(_)) => *l == 0,
//             (Number(l), Char(c)) => *l == *c as u32 as i64,
//             (Float(l), Float(r)) => *l == *r,
//             (Float(l), Boolean(b)) if *b => *l == 1.,
//             (Float(l), Boolean(_)) => *l == 0.,
//             (Float(l), Char(c)) => *l == *c as u32 as f64,
//             (Char(c), Boolean(b)) if *b => false,
//             (Char(c), Boolean(_)) => *c == '\0',
//             (Char(c), Char(r)) => *c == *r,
//             (Box(l), Box(r)) => l.eq(&r),
//             (Box(_), _) => false,
//             _ => o.eq(&self),
//         }
//     }
// }
//
// impl PartialOrd for Scalar {
//     #[inline]
//     fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
//         Some(self.cmp(&o))
//     }
// }
//
// impl Ord for Scalar {
//     #[inline]
//     fn cmp(&self, o: &Self) -> std::cmp::Ordering {
//         if self.is_box() && !o.is_box() || o.is_box() && !self.is_box() {
//             if self.is_box() { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
//         } else if self.is_float() || o.is_float() {
//             self.to_float().unwrap().cmp(&o.to_float().unwrap())
//         } else if self.is_box() && o.is_box() {
//             unsafe { self.unbox_unchecked().cmp(&o.unbox_unchecked()) }
//         } else {
//             self.to_num().unwrap().cmp(&o.to_num().unwrap())
//         }
//     }
// }
//
// impl Eq for Scalar {}
