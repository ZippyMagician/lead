#![allow(unused)]
use std::borrow::Cow;
use std::boxed::Box;
use std::cmp::PartialEq;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::hint::{/*cold_path,*/ unreachable_unchecked};
use std::ops::*;
use std::rc::Rc;

use super::Type;
use super::list::{Array, List};
use crate::utils::error::{Error, Status, err};
use crate::utils::{Arena, display_boxed, fmt_float, fmt_num};

use num_traits::{AsPrimitive, Zero};
use ordered_float::OrderedFloat;
use typed_arena;

pub trait Scalar:
    Display + PartialEq + PartialOrd + Eq + Ord + Clone + Copy + Hash + Default
{
    fn size(&self) -> usize;
    fn get_type() -> u8;
    fn wrap(s: Array<Self>) -> List;

    fn fmt_lead(&self) -> Cow<'static, str>;

    fn cast_to<T: Scalar>(v: Rc<Self>, l: usize) -> Status<Rc<T>>;
    fn to_num(&self) -> Status<i64>;
    fn to_float(&self) -> Status<OrderedFloat<f64>>;
    fn to_bool(&self) -> Status<bool>;
    fn to_char(&self) -> Status<u8>;
}

impl Scalar for i64 {
    #[inline(always)]
    fn size(&self) -> usize {
        std::mem::size_of::<i64>()
    }

    #[inline(always)]
    fn get_type() -> u8 {
        b'I'
    }

    fn cast_to<T: Scalar>(v: Rc<i64>, l: usize) -> Status<Rc<T>> {
        todo!("i64::cast_to")
    }

    #[inline(always)]
    fn wrap(s: Array<i64>) -> List {
        List::Integers(s)
    }

    #[inline(always)]
    fn fmt_lead(&self) -> Cow<'static, str> {
        fmt_num(*self).into()
    }

    #[inline(always)]
    fn to_num(&self) -> Status<i64> {
        Ok(*self)
    }

    #[inline(always)]
    fn to_float(&self) -> Status<OrderedFloat<f64>> {
        Ok(OrderedFloat(self.as_()))
    }

    #[inline(always)]
    fn to_bool(&self) -> Status<bool> {
        Ok(*self != 0)
    }

    #[inline(always)]
    fn to_char(&self) -> Status<u8> {
        if *self >= 0 && *self < 256 {
            Ok(*self as u8)
        } else {
            err!(ValueS, "Invalid character.")
        }
    }
}

impl Scalar for OrderedFloat<f64> {
    #[inline(always)]
    fn size(&self) -> usize {
        std::mem::size_of::<f64>()
    }

    #[inline(always)]
    fn get_type() -> u8 {
        b'F'
    }

    fn cast_to<T: Scalar>(v: Rc<OrderedFloat<f64>>, l: usize) -> Status<Rc<T>> {
        todo!("f64::cast_to")
    }

    #[inline(always)]
    fn wrap(s: Array<OrderedFloat<f64>>) -> List {
        List::Doubles(s)
    }

    #[inline(always)]
    fn fmt_lead(&self) -> Cow<'static, str> {
        fmt_float(*self)
    }

    #[inline(always)]
    fn to_num(&self) -> Status<i64> {
        Ok(self.as_())
    }

    #[inline(always)]
    fn to_float(&self) -> Status<OrderedFloat<f64>> {
        Ok(*self)
    }

    #[inline(always)]
    fn to_bool(&self) -> Status<bool> {
        Ok(*self != 0.)
    }

    #[inline(always)]
    fn to_char(&self) -> Status<u8> {
        if self.fract() == 0. {
            let u: i32 = self.floor().as_();
            if u >= 0 && u < 256 {
                Ok(u as u8)
            } else {
                err!(ValueS, "Invalid character.")
            }
        } else {
            err!(ValueS, "Float is not a whole number, cannot convert to character.")
        }
    }
}

impl Scalar for bool {
    #[inline(always)]
    fn size(&self) -> usize {
        std::mem::size_of::<bool>()
    }

    #[inline(always)]
    fn get_type() -> u8 {
        b'B'
    }

    fn cast_to<T: Scalar>(v: Rc<bool>, l: usize) -> Status<Rc<T>> {
        todo!("bool::cast_to")
    }

    #[inline(always)]
    fn wrap(s: Array<bool>) -> List {
        List::Booleans(s)
    }

    #[inline(always)]
    fn fmt_lead(&self) -> Cow<'static, str> {
        if *self {
            Cow::Borrowed("1")
        } else {
            Cow::Borrowed("0")
        }
    }

    #[inline(always)]
    fn to_num(&self) -> Status<i64> {
        Ok(*self as u8 as i64)
    }

    #[inline(always)]
    fn to_float(&self) -> Status<OrderedFloat<f64>> {
        Ok(OrderedFloat(*self as u8 as f64))
    }

    #[inline(always)]
    fn to_bool(&self) -> Status<bool> {
        Ok(*self)
    }

    #[inline(always)]
    fn to_char(&self) -> Status<u8> {
        Ok(*self as u8)
    }
}

// Strings
impl Scalar for u8 {
    #[inline(always)]
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
    }

    #[inline(always)]
    fn get_type() -> u8 {
        b'C'
    }

    fn cast_to<T: Scalar>(v: Rc<u8>, l: usize) -> Status<Rc<T>> {
        todo!("bool::cast_to")
    }

    #[inline(always)]
    fn wrap(s: Array<u8>) -> List {
        List::String(s)
    }

    // Should be handled separately
    #[inline(always)]
    fn fmt_lead(&self) -> Cow<'static, str> {
        unreachable!()
    }

    #[inline(always)]
    fn to_num(&self) -> Status<i64> {
        Ok(*self as i64)
    }

    #[inline(always)]
    fn to_float(&self) -> Status<OrderedFloat<f64>> {
        Ok(OrderedFloat(*self as f64))
    }

    #[inline(always)]
    fn to_bool(&self) -> Status<bool> {
        Ok(*self != 0)
    }

    #[inline(always)]
    fn to_char(&self) -> Status<u8> {
        Ok(*self)
    }
}

impl Scalar for Boxed {
    #[inline(always)]
    fn size(&self) -> usize {
        std::mem::size_of::<*const Type>()
    }

    #[inline(always)]
    fn get_type() -> u8 {
        b'X'
    }

    #[inline(always)]
    fn cast_to<T: Scalar>(v: Rc<Boxed>, l: usize) -> Status<Rc<T>> {
        err!(Domain)
    }

    #[inline(always)]
    fn wrap(s: Array<Boxed>) -> List {
        List::Boxes(s)
    }

    #[inline(always)]
    fn fmt_lead(&self) -> Cow<'static, str> {
        format!("{}", *self).into()
    }

    #[inline(always)]
    fn to_num(&self) -> Status<i64> {
        err!(ValueS, "Cannot convert from boxed value.")
    }

    #[inline(always)]
    fn to_float(&self) -> Status<OrderedFloat<f64>> {
        err!(ValueS, "Cannot convert from boxed value.")
    }

    #[inline(always)]
    fn to_bool(&self) -> Status<bool> {
        err!(ValueS, "Cannot convert from boxed value.")
    }

    #[inline(always)]
    fn to_char(&self) -> Status<u8> {
        err!(ValueS, "Cannot convert from boxed value.")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Boxed(*mut Type);

impl Boxed {
    #[inline]
    pub(crate) unsafe fn steal_box(&self) -> Type {
        std::mem::take(unsafe { &mut **&self.0 })
    }
}

impl Display for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_null() {
            err!(MemoryFault, "Lost Box"; !);
        }
        // SAFETY: Pointer checked. Taking reference, so other pointers are still valid
        let list: &Type = unsafe { &*self.0 };
        write!(f, "{}", display_boxed(format!("{list}"), None, None)?)
    }
}

#[inline]
pub fn boxed(a: &Arena, l: List) -> Status<Boxed> {
    let arena = a.borrow_mut();
    let l = l.into();
    let r = arena.alloc(l);
    Ok(Boxed(r))
}

#[inline]
pub fn boxed_t(a: &Arena, l: Type) -> Status<Boxed> {
    let arena = a.borrow_mut();
    let r = arena.alloc(l);
    Ok(Boxed(r))
}

#[inline]
pub fn boxed_custom(a: &mut typed_arena::Arena<Type>, l: Type) -> Status<Boxed> {
    let r = a.alloc(l);
    Ok(Boxed(r))
}

#[inline]
pub fn boxed_leak(l: List) -> Status<Boxed> {
    let l = l.into();
    Ok(Boxed(Box::into_raw(Box::new(l))))
}

/*
#[derive(Clone, Copy, Debug, Hash)]
#[repr(C, u8)]
pub enum Scalar {
    Number(i64),
    Float(OrderedFloat<f64>),
    Boolean(bool),
    Char(char),
    Box(*mut Type),
}

impl Scalar {
    #[inline]
    pub fn boxed(a: &Arena, l: List) -> Status<Self> {
        let arena = a.borrow_mut();
        let l = l.into();
        let r = arena.alloc(l);
        Ok(Box(r))
    }

    #[inline]
    pub fn boxed_t(a: &Arena, l: Type) -> Status<Self> {
        let arena = a.borrow_mut();
        let r = arena.alloc(l);
        Ok(Box(r))
    }

    #[inline]
    pub fn boxed_custom(a: &mut typed_arena::Arena<Type>, l: Type) -> Status<Self> {
        let r = a.alloc(l);
        Ok(Box(r))
    }

    #[inline]
    pub fn boxed_leak(l: List) -> Status<Self> {
        use std::boxed::Box;
        let l = l.into();
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
    pub fn to_cast(self, tag: char) -> Status<Self> {
        let res = match tag {
            'N' => match self {
                Self::Box(p) => Self::Number(p as u32 as i64),
                Self::Number(_) | Self::Float(_) => self,
                _ => Self::Number(self.to_num().unwrap()),
            },
            'I' => match self {
                Self::Box(p) => Self::Number(p as u32 as i64),
                Self::Number(_) => self,
                _ => Self::Number(self.to_num().unwrap()),
            },
            'F' => match self {
                Self::Box(p) => Self::Float(OrderedFloat(p as u32 as f64)),
                Self::Float(_) => self,
                _ => Self::Float(self.to_float().unwrap()),
            },
            'C' => match self {
                Self::Box(p) => Self::Char(char::from_u32(p as u32).ok_or(Error::NaN)?),
                Self::Char(_) => self,
                _ => Self::Char(self.to_char()?),
            },
            'B' => match self {
                Self::Box(_) => Self::Boolean(!unsafe { self.as_unboxed_unchecked() }.is_empty()),
                Self::Boolean(_) => self,
                _ => Self::Boolean(self.to_bool().unwrap()),
            },
            _ => {
                std::hint::cold_path();
                return err!(Value);
            }
        };
        Ok(res)
    }

    #[inline]
    pub fn to_num(&self) -> Status<i64> {
        match self {
            Self::Number(n) => Ok(*n),
            Self::Float(f) => Ok(f.as_()),
            Self::Char(c) => Ok(*c as i64),
            Self::Boolean(b) => Ok(*b as i64),
            _ => {
                std::hint::cold_path();
                err!(NaN)
            }
        }
    }

    #[inline]
    pub fn to_float(&self) -> Status<OrderedFloat<f64>> {
        match self {
            Self::Number(n) => Ok(OrderedFloat(*n as f64)),
            Self::Float(f) => Ok(*f),
            Self::Char(c) => Ok(OrderedFloat(*c as u32 as f64)),
            Self::Boolean(b) => Ok(OrderedFloat(*b as u8 as f64)),
            _ => {
                std::hint::cold_path();
                err!(NaN)
            }
        }
    }

    #[inline]
    pub fn to_char(&self) -> Status<char> {
        match self {
            Self::Char(c) => Ok(*c),
            Self::Number(n) if *n >= 0 => char::from_u32(*n as u64 as u32).ok_or(Error::Value),
            Self::Float(f) if f.floor() == f.into_inner() => {
                char::from_u32(f.floor().as_()).ok_or(Error::Value)
            }
            _ => {
                std::hint::cold_path();
                err!(Value)
            }
        }
    }

    #[inline]
    pub fn to_bool(&self) -> Status<bool> {
        match self {
            Self::Char('\0') => Ok(false),
            Self::Number(0) => Ok(false),
            Self::Float(f) if f.is_zero() => Ok(false),
            Self::Boolean(b) => Ok(*b),
            Self::Box(_) => {
                std::hint::cold_path();
                err!(Value)
            }
            _ => Ok(true),
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
    pub fn get_fill(&self) -> Status<Scalar> {
        match self {
            Self::Number(_) | Self::Float(_) => Ok(Self::Number(0)),
            Self::Char(_) => Ok(Self::Char(' ')),
            Self::Boolean(_) => Ok(Self::Boolean(false)),
            Self::Box(_) => {
                std::hint::cold_path();
                err!(ValueS, "No valid fill type for boxes.")
            }
        }
    }

    #[inline]
    pub(crate) unsafe fn as_unboxed_unchecked(&self) -> &Type {
        match self {
            Self::Box(p) => unsafe { &**p },
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn steal_box(&self) -> Type {
        let Self::Box(p) = &self else { unsafe { unreachable_unchecked() } };
        std::mem::take(unsafe { &mut **p })
    }

    #[inline]
    pub(crate) unsafe fn unbox_unchecked(self) -> Type {
        match self {
            Self::Box(p) => unsafe { (*p).clone() },
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn to_num_unchecked(&self) -> i64 {
        match self {
            Self::Number(n) => *n,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    #[inline]
    pub(crate) unsafe fn to_float_unchecked(&self) -> OrderedFloat<f64> {
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

impl Scalar {
    #[inline(always)]
    pub(crate) fn abs(self) -> Self {
        match self {
            Self::Number(n) => Self::Number(n.abs()),
            _ => self,
        }
    }

    #[inline(always)]
    pub(crate) fn abs_diff(self, rhs: Self) -> Status<Self> {
        if self.is_box() || rhs.is_box() {
            err!(NaN)
        } else if self.is_num() && rhs.is_num() {
            unsafe {
                let a = self.to_num_unchecked();
                let b = rhs.to_num_unchecked();
                n(a.abs_diff(b) as i64)
            }
        } else if self.is_float() && rhs.is_float() {
            unsafe {
                let a = self.to_float_unchecked();
                let b = rhs.to_float_unchecked();
                f(OrderedFloat((a - b).abs()))
            }
        } else if self.is_float() || rhs.is_float() {
            let a = self.to_float()?;
            let b = rhs.to_float()?;
            f(OrderedFloat((a - b).abs()))
        } else {
            let a = self.to_num()?;
            let b = rhs.to_num()?;
            n(a.abs_diff(b) as i64)
        }
    }
}

use Scalar::*;

#[inline(always)]
pub(crate) fn n(i: i64) -> Status<Scalar> {
    Ok(Scalar::Number(i))
}

#[inline(always)]
pub(crate) fn f(i: OrderedFloat<f64>) -> Status<Scalar> {
    Ok(Scalar::Float(i))
}

#[inline(always)]
pub(crate) fn b(i: bool) -> Status<Scalar> {
    Ok(Scalar::Boolean(i))
}

#[inline(always)]
pub(crate) fn c(i: u32) -> Status<Scalar> {
    Ok(Scalar::Char(char::from_u32(i).ok_or(Error::Domain)?))
}

macro_rules! impl_arith_ops {
    ($($name:ident, $f:ident, [$a1:ident, $a2:ident] $operation:block);*;) => {
        $(
            impl $name for Scalar {
                type Output = Status<Self>;

                #[inline]
                #[allow(unused_variables)]
                fn $f(self, rhs: Self) -> Self::Output {
                    // If this first branch is removed, this for some fucking reason
                    // becomes literally 10x more expensive... ???? despite it being
                    // checked in to_float and to_num as well.....
                    // Note: ::std::hint::cold_path makes this slightly slower
                    if self.is_box() || rhs.is_box() {
                        err!(NaN)
                    } else if self.is_num() && rhs.is_num() {
                        unsafe {
                            let $a1 = self.to_num_unchecked();
                            let $a2 = rhs.to_num_unchecked();
                            n($operation)
                        }
                    } else if self.is_float() && rhs.is_float() {
                        unsafe {
                            let $a1 = self.to_float_unchecked();
                            let $a2 = rhs.to_float_unchecked();
                            f($operation)
                        }
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
        )*
    }
}

impl_arith_ops! {
    Add, add, [l, r] {l + r};
    Sub, sub, [l, r] {l - r};
    Mul, mul, [l, r] {l * r};
    Div, div, [l, r] {
        use num_traits::Zero;
        // So we don't panic on division by 0. f64 lacks
        // a `checked_div`.
        if r.is_zero() {
            std::hint::cold_path(); return err!(NaN);
        } else { l / r }
    };
    Rem, rem, [l, r] {
        use num_traits::Zero;
        if r.is_zero() {
            std::hint::cold_path(); return err!(NaN);
        } else { l % r }
    };
}

impl Add<u32> for Scalar {
    type Output = Status<Self>;

    fn add(self, rhs: u32) -> Self::Output {
        if self.is_box() {
            return err!(NaN);
        } else if self.is_float() {
            return f(unsafe { self.to_float_unchecked() } + rhs as f64);
        } else if self.is_char() {
            return c(unsafe { self.to_char_unchecked() } as u32 + rhs);
        } else {
            return n(self.to_num()? + rhs as i64);
        }
    }
}

impl Sub<u32> for Scalar {
    type Output = Status<Self>;

    fn sub(self, rhs: u32) -> Self::Output {
        if self.is_box() {
            return err!(NaN);
        } else if self.is_float() {
            return f(unsafe { self.to_float_unchecked() } - rhs as f64);
        } else if self.is_char() {
            return c(unsafe { self.to_char_unchecked() } as u32 - rhs);
        } else {
            return n(self.to_num()? - rhs as i64);
        }
    }
}

impl PartialEq for Scalar {
    #[inline]
    fn eq(&self, o: &Self) -> bool {
        use Scalar::*;
        match (&self, &o) {
            (Number(l), Number(r)) => *l == *r,
            (Number(l), Float(r)) => OrderedFloat(*l as f64) == *r,
            (Number(l), Boolean(b)) if *b => *l == 1,
            (Number(l), Boolean(_)) => *l == 0,
            (Number(l), Char(c)) => *l == *c as u32 as i64,
            (Float(l), Float(r)) => *l == *r,
            (Float(l), Boolean(b)) if *b => *l == 1.,
            (Float(l), Boolean(_)) => *l == 0.,
            (Float(l), Char(c)) => *l == *c as u32 as f64,
            (Char(c), Boolean(b)) if *b => false,
            (Char(c), Boolean(_)) => *c == '\0',
            (Char(c), Char(r)) => *c == *r,
            (Box(l), Box(r)) => l.eq(&r),
            (Box(_), _) => false,
            _ => o.eq(&self),
        }
    }
}

impl PartialOrd for Scalar {
    #[inline]
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&o))
    }
}

impl Ord for Scalar {
    #[inline]
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        if self.is_box() && !o.is_box() || o.is_box() && !self.is_box() {
            if self.is_box() { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }
        } else if self.is_float() || o.is_float() {
            self.to_float().unwrap().cmp(&o.to_float().unwrap())
        } else if self.is_box() && o.is_box() {
            unsafe { self.unbox_unchecked().cmp(&o.unbox_unchecked()) }
        } else {
            self.to_num().unwrap().cmp(&o.to_num().unwrap())
        }
    }
}

impl Eq for Scalar {}

impl Display for Scalar {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", fmt_num(*n)),
            Self::Float(n) => write!(f, "{}", fmt_float(*n)),
            Self::Boolean(b) => write!(f, "{}", *b as u8),
            Self::Char(c) => write!(f, "{c}"),
            Self::Box(l) => {
                if l.is_null() {
                    err!(MemoryFault, "Lost Box"; !);
                }
                // SAFETY: Pointer checked. Taking reference, so other pointers are still valid
                let list: &Type = unsafe { &**l };
                write!(f, "{}", display_boxed(format!("{list}"), None, None)?)
            }
        }
    }
}*/
