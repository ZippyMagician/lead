use crate::array::arith;
use crate::builtins::symbols::{self, Symbol};
use crate::function::{Function, Opts};
use crate::types::Type;
use crate::types::list::{Array, List};
use crate::types::scalar::Scalar;

use crate::utils::error::{Status, err};
use crate::utils::function::apply_loosem;
use crate::utils::list::iter_ints;
use crate::utils::symbols::*;

use smallvec::SmallVec;

// SAFETY: Assumes both lists are booleans
#[inline(always)]
pub unsafe fn scalar_xor(a: List, b: List) -> Status<List> {
    let mut a = unsafe { a.arrayb_unchecked() };
    let mut b = unsafe { b.arrayb_unchecked() };
    scalar_op!(a, b,
        [l, r] = { *l ^ *r }; { *r ^ l }; { *l ^ r };
    )
    .map(List::Booleans)
}

// SAFETY: See above
#[inline(always)]
pub unsafe fn scalar_and(a: List, b: List) -> Status<List> {
    let mut a = unsafe { a.arrayb_unchecked() };
    let mut b = unsafe { b.arrayb_unchecked() };
    scalar_op!(a, b,
        [l, r] = { *l && *r }; { *r && l }; { *l && r };
    )
    .map(List::Booleans)
}

// SAFETY: See above
#[inline(always)]
pub unsafe fn scalar_or(a: List, b: List) -> Status<List> {
    let mut a = unsafe { a.arrayb_unchecked() };
    let mut b = unsafe { b.arrayb_unchecked() };
    scalar_op!(a, b,
        [l, r] = { *l || *r }; { *r || l }; { *l || r };
    )
    .map(List::Booleans)
}

macro_rules! reduce_op_primitive {
    ($a:expr, $name:path) => {{
        let mut items = $a.into_iter();
        let mut acc = *items.next().unwrap();
        for &item in items {
            acc = $name(acc, item)?;
        }
        Ok(Array::scalar(acc))
    }};
}

macro_rules! reduce_op {
    ($a:expr, $($t:tt)*) => {{
        match $a.get_type() {
            b'F' => reduce_op_primitive!(unsafe { $a.listf_unchecked() }, $($t)*::float_op),
            b'I' => reduce_op_primitive!(unsafe { $a.listi_unchecked() }, $($t)*::int_op),
            b'C' => reduce_op_primitive!(unsafe { $a.listc_unchecked() }, $($t)*::char_op),
            _ => err!(Box),
        }
    }};
}

macro_rules! match_symbol_general {
    ($tag:ident, $f:ident, $items:ident, $acc:ident; $($sym:pat, $($seg:ident)::+);*) => {
        if let Some(s) = &$f.s {
            match ($tag, s) {
                (b'B' | b'X', _) => {}
                $(
                    (b'I', $sym) => {
                        for item in $items {
                            $acc = $($seg)::+::<b'I', b'I'>($acc, item)?;
                        }
                        return Ok($acc);
                    }
                    (b'F', $sym) => {
                        for item in $items {
                            $acc = $($seg)::+::<b'F', b'F'>($acc, item)?;
                        }
                        return Ok($acc);
                    }
                    (b'C', $sym) => {
                        for item in $items {
                            $acc = $($seg)::+::<b'C', b'C'>($acc, item)?;
                        }
                        return Ok($acc);
                    }
                )*
                _ => {}
            }
        }
    }
}

#[inline]
pub fn reduce(f: &Function, a: Type, o: &Opts) -> Status<Type> {
    if a.is_empty() || a.is_scalar() {
        std::hint::cold_path();
        return Ok(a);
    }

    let a = a.to_list()?;
    maybe_apply!([within] o, a, {
        if let Some(s) = &f.s
            && a.dims() == 1
        {
            match (a.get_type(), s) {
                // (b'I', Symbol::Plus) => return Ok(reduce_op_ints(a, |a, b| a + b)),
                (_, Symbol::Plus) => return reduce_op!(a, arith::add),
                (b'B', Symbol::PlusColon) => return Ok(reduce_op_bools(a, |a, b| a ^ b, |_| false)),
                (_, Symbol::PlusColon) => return reduce_op!(a, arith::wrapping_add),
                (_, Symbol::Dash) => return reduce_op!(a, arith::sub),
                (b'B', Symbol::Star) => return Ok(reduce_op_bools(a, |a, b| a && b, |n| !n)),
                (_, Symbol::Star) => return reduce_op!(a, arith::mul),
                (_, Symbol::Percent) => return reduce_op!(a, arith::div),
                (b'B', Symbol::Pipe) => return Ok(reduce_op_bools(a, |a, b| a || b, |n| n)),
                (_, Symbol::Pipe) => return reduce_op!(a, arith::rem),
                (_, Symbol::Comma) => return f.monadic_call(Type::List(a))?.to_list(),
                _ => {}
            }
        }

        let tag = a.get_type();
        let mut items = a.ranked(a.dims() - 1)?.into_iter();
        let mut acc = items.next().unwrap();
        
        // match general arithmetic operations
        match_symbol_general! {
            tag, f, items, acc;
            Symbol::Plus, arith::add::typed_op;
            Symbol::PlusColon, arith::wrapping_add::typed_op;
            Symbol::Dash, arith::sub::typed_op;
            Symbol::Star, arith::mul::typed_op;
            Symbol::Percent, arith::div::typed_op;
            Symbol::Pipe, arith::rem::typed_op
        };

        let mut acc = Type::List(acc);
        for item in items {
            acc = f.dyadic_call(acc, Type::List(item))?;
        }
        acc.to_list()
    })
    .map(Type::List)
}

#[inline]
pub fn reduce_op(dims: usize, a: List, f: impl Fn(List, List) -> Status<List>) -> Status<Type> {
    if a.get_type() == b'F' {
        let mut items = a.ranked(dims)?.into_iter();
        let mut acc = items.next().unwrap();
        for item in items {
            unsafe {
                std::hint::assert_unchecked(acc.get_type() == b'F');
                std::hint::assert_unchecked(item.get_type() == b'F');
            }
            acc = f(acc, item)?;
        }

        Ok(Type::List(acc))
    } else if a.get_type() == b'I' {
        let mut items = a.ranked(dims)?.into_iter();
        let mut acc = items.next().unwrap();
        for item in items {
            unsafe {
                std::hint::assert_unchecked(acc.get_type() == b'I');
                std::hint::assert_unchecked(item.get_type() == b'I');
            }
            acc = f(acc, item)?;
        }

        Ok(Type::List(acc))
    } else if a.get_type() == b'C' {
        let mut items = a.ranked(dims)?.into_iter();
        let mut acc = items.next().unwrap();
        for item in items {
            unsafe {
                std::hint::assert_unchecked(acc.get_type() == b'C');
                std::hint::assert_unchecked(item.get_type() == b'C');
            }
            acc = f(acc, item)?;
        }

        Ok(Type::List(acc))
    } else {
        err!(Box)
    }
}

#[inline]
pub fn reduce_op_ints(a: List, f: impl Fn(i64, i64) -> i64) -> List {
    let ints = unsafe { a.listi_unchecked() };
    let mut acc = ints[0];

    for &i in ints.iter().skip(1) {
        acc = f(acc, i);
    }
    Array::scalar(acc)
}

#[inline]
pub fn reduce_op_bools(a: List, f: impl Fn(bool, bool) -> bool, c: impl Fn(bool) -> bool) -> List {
    let bools = unsafe { a.listb_unchecked() };
    let mut acc = bools[0];

    let mut i = 1;
    while !c(acc) {
        acc = f(acc, bools[i]);
        i += 1;
    }

    Array::scalar(acc)
}

#[inline(always)]
pub fn reduce_groups(f: &Function, a: Type, b: Type, o: &Opts) -> Status<Type> {
    if a.is_empty() {
        std::hint::cold_path();
        return Ok(b);
    } else if b.is_empty() || b.is_scalar() {
        std::hint::cold_path();
        return Ok(b);
    }

    let a = a.to_list()?;
    let b = b.to_list()?;
    maybe_apply!([within] o, a, b, {
        if b.is_scalar() {
            std::hint::cold_path();
            return Ok(b);
        }
        if !a.is_scalar() {
            std::hint::cold_path();
            return err!(Domain);
        }

        let window = iter_ints!(a, |it| it.next().unwrap());
        let mut unique = false;
        let window = if window < 0 {
            unique = true;
            (-window) as usize
        } else {
            window as usize
        };
        let dims = b.dims();
        let mut rank: SmallVec<[usize; 2]> = SmallVec::from_slice(&b.rank()[..dims - 1]);
        let offset: usize = rank.iter().chain(std::iter::once(&1usize)).product();
        rank.push(window);
        let jump = if unique { window } else { 1 };
        let mut cells = Vec::with_capacity(1 + (b.rank()[dims - 1] - window) / jump);
        let mut j = 0;
        while j <= b.rank()[dims - 1] - window {
            let slice = unsafe { b.sliced(j * offset, (j + window) * offset, &rank) };
            cells.push(unsafe { reduce(&f, Type::List(slice), o)?.to_list_unchecked() });
            j += jump;
        }
        List::unranked(&o.a, 0, &[], cells)
    })
    .map(Type::from)
}
