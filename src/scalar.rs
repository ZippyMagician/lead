#![allow(dead_code)]
use std::hint::unreachable_unchecked;
use std::ops::*;

use crate::list::List;
use crate::utils::error::err;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(C)]
pub enum Scalar {
    Number(i32),
    Float(f32),
    Boolean(bool),
    Char(char),
    Box(*const List),
}

impl Scalar {
    #[inline(always)]
    pub fn is_num(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    #[inline(always)]
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    #[inline(always)]
    pub fn is_box(&self) -> bool {
        matches!(self, Self::Box(_))
    }

    #[inline]
    pub fn to_num(&self) -> i32 {
        match self {
            Self::Number(n) => *n,
            Self::Float(f) => *f as i32,
            Self::Char(c) => *c as i32,
            Self::Boolean(b) => *b as i32,
            _ => err!(NaN; !),
        }
    }

    #[inline]
    pub fn to_float(&self) -> f32 {
        match self {
            Self::Number(n) => *n as f32,
            Self::Float(f) => *f,
            Self::Char(c) => *c as u32 as f32,
            Self::Boolean(b) => *b as u8 as f32,
            _ => err!(NaN; !),
        }
    }

    #[inline]
    unsafe fn unbox_unchecked(self) -> List {
        match self {
            Self::Box(p) => unsafe { (*p).clone() },
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    unsafe fn to_num_unchecked(&self) -> i32 {
        match self {
            Self::Number(n) => *n,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    unsafe fn to_float_unchecked(&self) -> f32 {
        match self {
            Self::Float(f) => *f,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    unsafe fn to_bool_unchecked(&self) -> bool {
        match self {
            Self::Boolean(b) => *b,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

use Scalar::*;

#[inline(always)]
fn n(i: i32) -> Scalar {
    Scalar::Number(i)
}

#[inline(always)]
fn f(i: f32) -> Scalar {
    Scalar::Float(i)
}

#[inline(always)]
fn b(i: bool) -> Scalar {
    Scalar::Boolean(i)
}

macro_rules! impl_arith_ops {
    ($($name:ident, $f:ident, [$a1:ident, $a2:ident] $operation:block);*;) => {
        $(
            impl $name for Scalar {
                type Output = Self;

                #[inline]
                fn $f(self, rhs: Self) -> Self::Output {
                    // If this first branch is removed, this for some fucking reason
                    // becomes literally 10x more expensive... ???? despite it being
                    // checked in to_float and to_num as well.....
                    // Note: ::std::hint::cold_path makes this slightly slower
                    if self.is_box() || rhs.is_box() {
                        err!(NaN; !);
                    } else if self.is_float() || rhs.is_float() {
                        let $a1 = self.to_float();
                        let $a2 = rhs.to_float();
                        f($operation)
                    } else {
                        let $a1 = self.to_num();
                        let $a2 = rhs.to_num();
                        n($operation)
                    }
                }
            }

            impl $name<i32> for Scalar {
                type Output = Self;

                fn $f(self, rhs: i32) -> Self::Output {
                    if self.is_box() {
                        err!(NaN; !)
                    } else if let Float(fl) = self {
                        let $a1 = fl;
                        let $a2 = rhs as f32;
                        f($operation)
                    } else {
                        let $a1 = self.to_num();
                        let $a2 = rhs;
                        n($operation)
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
