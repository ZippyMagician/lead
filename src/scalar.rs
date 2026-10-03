#![allow(dead_code)]
use std::ops::*;

use crate::list::List;
use crate::utils::error::{Status, err};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(C)]
pub enum Scalar {
    Number(i32),
    Float(f32),
    Boolean(bool),
    Box(*const List),
}

impl Scalar {
    #[inline(always)]
    fn is_num(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    #[inline(always)]
    fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    #[inline(always)]
    fn is_bool(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    #[inline(always)]
    fn is_box(&self) -> bool {
        matches!(self, Self::Box(_))
    }

    #[inline]
    unsafe fn unbox_unchecked(self) -> List {
        match self {
            Self::Box(p) => unsafe { (*p).clone() },
            _ => unreachable!(),
        }
    }

    #[inline]
    unsafe fn to_num_unchecked(&self) -> i32 {
        match self {
            Self::Number(n) => *n,
            _ => unreachable!(),
        }
    }

    #[inline]
    unsafe fn to_float_unchecked(&self) -> f32 {
        match self {
            Self::Float(f) => *f,
            _ => unreachable!(),
        }
    }

    #[inline]
    unsafe fn to_bool_unchecked(&self) -> bool {
        match self {
            Self::Boolean(b) => *b,
            _ => unreachable!(),
        }
    }
}

use Scalar::*;

#[inline(always)]
fn n(i: i32) -> Status<Scalar> {
    Ok(Scalar::Number(i))
}

#[inline(always)]
fn f(i: f32) -> Status<Scalar> {
    Ok(Scalar::Float(i))
}

#[inline(always)]
fn b(i: bool) -> Status<Scalar> {
    Ok(Scalar::Boolean(i))
}

macro_rules! impl_arith_ops {
    ($($name:ident, $f:ident, [$a1:ident, $a2:ident] $operation:block);*;) => {
        $(
            impl $name for Scalar {
                type Output = Status<Self>;

                #[inline]
                fn $f(self, rhs: Self) -> Self::Output {
                    match (self, rhs) {
                        (Number(l), Number(r)) => n(l + r),
                        (Number(l), Float(r)) => f(l as f32 + r),
                        (Number(l), Boolean(r)) => n(l + r as i32),
                        (Float(l), Number(r)) => f(l + r as f32),
                        (Float(l), Float(r)) => f(l + r),
                        (Float(l), Boolean(r)) => f(l + r as u8 as f32),
                        (Boolean(l), Number(r)) => n(l as i32 + r),
                        (Boolean(l), Float(r)) => f(l as u8 as f32 + r),
                        (Boolean(l), Boolean(r)) => n(l as i32 + r as i32),
                        _ => err!(NaN),
                    }
                }
            }

            impl $name<i32> for Scalar {
                type Output = Status<Self>;

                fn $f(self, rhs: i32) -> Self::Output {
                    match self {
                        Number(l) => n(l + rhs),
                        Float(l) => f(l + rhs as f32),
                        Boolean(l) => n(l as i32 + rhs),
                        _ => err!(NaN),
                    }
                }
            }
        )*
    }
}

impl_arith_ops! {
    Add, add, [l, r] {l + r};
    Sub, sub, [l, r] {l - r};
    Mul, mul, [l, r] {l * r};
    Div, div, [l, r] {l / r};
    Rem, rem, [l, r] {l % r};
}

macro_rules! scalar_op {
    ($a:ident, $b:ident, $d:ident) => {
        if $a.len() == $b.len() {
            let values = $a.list_mut();
            for (l, r) in values.iter_mut().zip($b.list()) {
                *l = l.$d(*r)?;
            }
            Ok($a)
        } else if $a.is_empty() {
            Ok($b)
        } else if $b.is_empty() {
            Ok($a)
        } else if $a.is_scalar() {
            let a = $a.list()[0];
            for r in $b.list_mut() {
                *r = r.$d(a)?;
            }
            Ok($b)
        } else if $b.is_scalar() {
            let b = $b.list()[0];
            for l in $a.list_mut() {
                *l = l.$d(b)?;
            }
            Ok($a)
        } else if $a.len() % $b.len() == 0 || $b.len() % $a.len() == 0 {
            let m;
            let o;
            if $a.len() > $b.len() {
                m = $a.list_mut();
                o = $b.list();
            } else {
                m = $b.list_mut();
                o = $a.list();
            }

            for (l, r) in m.iter_mut().zip(o.iter().cycle()) {
                *l = l.$d(*r)?;
            }

            Ok(if $a.len() > $b.len() { $a } else { $b })
        } else {
            err!(Domain)
        }
    };
}

pub(crate) use scalar_op;
