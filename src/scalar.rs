#![allow(dead_code)]
use std::fmt::Debug;
use std::hint::{/*cold_path,*/ unreachable_unchecked};
use std::ops::*;

use crate::list::List;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Scalar {
    Number(i32),
    Float(f32),
    Boolean(bool),
    Char(char),
    Box(*const List),
}

// Representation of scalar, for casting
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(crate) struct Repr<T>
where
    T: Clone + Copy + Debug + PartialEq + PartialOrd,
{
    tag: u8,
    value: T,
}

impl Scalar {
    #[inline]
    pub fn boxed(a: Arena, l: List) -> Status<Self> {
        let arena = a.borrow_mut();
        let r = arena.alloc(l);
        Ok(Box(r as *const List))
    }

    #[inline]
    pub fn boxed_leak(l: List) -> Status<Self> {
        use std::boxed::Box;
        Ok(Self::Box(Box::into_raw(Box::new(l))))
    }

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
    pub fn is_char(&self) -> bool {
        matches!(self, Self::Char(_))
    }

    #[inline(always)]
    pub fn is_box(&self) -> bool {
        matches!(self, Self::Box(_))
    }

    #[inline]
    pub fn to_num(&self) -> Status<i32> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Float(f) => Ok(*f as i32),
            Self::Char(c) => Ok(*c as i32),
            Self::Boolean(b) => Ok(*b as i32),
            _ => err!(NaN),
        }
    }

    #[inline]
    pub fn to_float(&self) -> Status<f32> {
        match self {
            Self::Number(n) => Ok(*n as f32),
            Self::Float(f) => Ok(*f),
            Self::Char(c) => Ok(*c as u32 as f32),
            Self::Boolean(b) => Ok(*b as u8 as f32),
            _ => err!(NaN),
        }
    }

    #[inline]
    pub fn to_char(&self) -> Status<char> {
        match self {
            Self::Char(c) => Ok(*c),
            _ => err!(Domain),
        }
    }

    #[inline]
    pub fn repr(&self) -> u8 {
        match self {
            Self::Number(_) | Self::Float(_) => 0,
            Self::Char(_) => 1,
            Self::Boolean(_) => 2,
            Self::Box(_) => 3,
        }
    }

    #[inline]
    pub(crate) unsafe fn as_unboxed_unchecked(&self) -> &List {
        match self {
            Self::Box(p) => unsafe { &**p },
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn unbox_unchecked(self) -> List {
        match self {
            Self::Box(p) => unsafe { (*p).clone() },
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn to_num_unchecked(&self) -> i32 {
        match self {
            Self::Number(n) => *n,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn to_float_unchecked(&self) -> f32 {
        match self {
            Self::Float(f) => *f,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn to_bool_unchecked(&self) -> bool {
        match self {
            Self::Boolean(b) => *b,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn to_char_unchecked(&self) -> char {
        match self {
            Self::Char(c) => *c,
            _ => unsafe { unreachable_unchecked() },
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

#[inline(always)]
fn c(i: i32) -> Status<Scalar> {
    Ok(Scalar::Char(char::from_u32(i as u32).ok_or(Error::Domain)?))
}

macro_rules! impl_arith_ops {
    ($($name:ident, $f:ident, [$a1:ident, $a2:ident] $operation:block);*;) => {
        $(
            impl $name for Scalar {
                type Output = Status<Self>;

                #[inline]
                fn $f(self, rhs: Self) -> Self::Output {
                    // If this first branch is removed, this for some fucking reason
                    // becomes literally 10x more expensive... ???? despite it being
                    // checked in to_float and to_num as well.....
                    // Note: ::std::hint::cold_path makes this slightly slower
                    if self.is_box() || rhs.is_box() {
                        err!(NaN)
                    } else if self.is_float() || rhs.is_float() {
                        let $a1 = self.to_float()?;
                        let $a2 = rhs.to_float()?;
                        f($operation)
                    } else {
                        let $a1 = self.to_num()?;
                        let $a2 = rhs.to_num()?;
                        n($operation)
                    }
                }
            }

            impl $name<i32> for Scalar {
                type Output = Status<Self>;

                fn $f(self, rhs: i32) -> Self::Output {
                    if self.is_box() {
                        err!(NaN)
                    } else if let Float(fl) = self {
                        let $a1 = fl;
                        let $a2 = rhs as f32;
                        f($operation)
                    } else {
                        let $a1 = self.to_num()?;
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
