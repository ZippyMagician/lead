use crate::array::arith;
use crate::builtins::symbols::Symbol;
use crate::function::{Function, Opts};
use crate::types::Type;
use crate::types::list::{Array, List};
use crate::types::scalar::{self, Boxed, Scalar};

use crate::utils::error::{Status, err};
use crate::utils::list::iter_ints;
use crate::utils::symbols::*;

use ordered_float::OrderedFloat;
use smallvec::{SmallVec, smallvec};

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
                (_, Symbol::LeftBracket) => return unsafe { Ok(a.sliced(0, 1, &[1])) },
                (_, Symbol::RightBracket) => return unsafe { Ok(a.sliced(a.len() - 1, a.len(), &[1])) },
                _ => {}
            }
        }

        if let Some(Symbol::RightBracket) = &f.s {
            let rank = &a.rank()[0..a.dims() - 1];
            let trailing = a.rank()[a.dims() - 1];
            let size = a.len() / trailing;
            return Ok(unsafe { a.sliced(size * (trailing - 1), size * trailing, rank) });
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

        if let Some(Symbol::LeftBracket) = &f.s {
            return Ok(acc);
        }

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
    while i < bools.len() && !c(acc) {
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

#[inline(always)]
pub(crate) fn scan(f: &Function, a: Type, o: &Opts) -> Status<Type> {
    if !a.is_list() {
        std::hint::cold_path();
        return err!(Domain);
    } else if a.is_empty() || a.is_scalar() {
        std::hint::cold_path();
        return Ok(a);
    }

    let a = unsafe { a.to_list_unchecked() };
    maybe_apply!([within] o, a, {
        if a.is_scalar() {
            std::hint::cold_path();
            return Ok(a);
        }

        let runs = a.ranked(a.dims() - 1)?;
        if runs.len() == 1 {
            return Ok(runs.into_iter().next().unwrap());
        }

        let mut b = Vec::with_capacity(runs.len());
        let mut runs = runs.into_iter();
        b.push(runs.next().unwrap());
        let mut boxit = false;
        let mut r: SmallVec<[usize; 2]> = SmallVec::from_slice(b[0].rank());
        for (i, run) in runs.enumerate() {
            let res = f.dyadic_call(b[i].clone().into(), run.into())?.to_list()?;
            if !boxit && (res.rank() != &*r || res.get_type() != b[0].get_type()) {
                std::hint::cold_path();
                boxit = true;
            }
            b.push(res);
        }

        if boxit {
            unsafe {
                Array::alloc_iter(
                    &[b.len()],
                    b.into_iter().map(|n| scalar::boxed(&o.a, n).unwrap()),
                )
            }
        } else {
            if b[0].is_scalar() {
                r.clear();
            }
            r.push(b.len());
            unsafe {
                match b[0].get_type() {
                    b'I' => Array::<i64>::alloc_iter(&r, b.iter().flat_map(|n| n.treat_as_ref().list().iter()).copied()),
                    b'F' => Array::<OrderedFloat<f64>>::alloc_iter(&r, b.iter().flat_map(|n| n.treat_as_ref().list().iter()).copied()),
                    b'C' => Array::<u8>::alloc_iter(&r, b.iter().flat_map(|n| n.treat_as_ref().list().iter()).copied()),
                    b'B' => Array::<bool>::alloc_iter(&r, b.iter().flat_map(|n| n.treat_as_ref().list().iter()).copied()),
                    b'X' => Array::<Boxed>::alloc_iter(&r, b.iter().flat_map(|n| n.treat_as_ref().list().iter()).copied()),
                    _ => std::hint::unreachable_unchecked(),
                }
            }
        }
    }).map(Type::List)
}

#[inline(always)]
pub(crate) fn table(f: &Function, a: Type, b: Type, o: &Opts) -> Status<Type> {
    if !b.is_list() || !a.is_list() {
        std::hint::cold_path();
        return err!(Domain);
    }

    let a = unsafe { a.to_list_unchecked() };
    let b = unsafe { b.to_list_unchecked() };
    let rda = u8::max(u8::min(a.dims() as u8, o.rd[0]), 1) - 1;
    let rdb = u8::max(u8::min(b.dims() as u8, o.rd[0]), 1) - 1;
    let a = a.ranked(rda as usize)?;
    let b = b.ranked(rdb as usize)?;

    let size_a = a.len();
    let size_b = b.len();

    let mut boxit = false;
    let mut v: Vec<Type> = Vec::with_capacity(a.len() * b.len());
    let mut r: Option<SmallVec<[usize; 2]>> = None;
    for a in a {
        for b in b.iter() {
            let res = f.dyadic_call(a.clone().into(), b.clone().into())?;
            if !boxit && !res.is_list() {
                std::hint::cold_path();
                boxit = true;
            } else if res.is_scalar() {
                if r.is_none() {
                    std::hint::cold_path();
                    r = Some(smallvec![1]);
                } else if !boxit && r.as_ref().unwrap().as_slice() != &[1] {
                    std::hint::cold_path();
                    boxit = true;
                } else if !boxit
                    && (unsafe { res.as_list_unchecked() }.get_type() == b'X'
                        || unsafe { res.as_list_unchecked() }.get_type()
                            != unsafe { v.last().unwrap().as_list_unchecked() }.get_type())
                {
                    std::hint::cold_path();
                    boxit = true;
                }
            } else {
                let l = unsafe { res.as_list_unchecked() };
                if r.is_none() {
                    std::hint::cold_path();
                    r = Some(SmallVec::from_slice(l.rank()));
                } else if !boxit && r.as_ref().unwrap().as_slice() != l.rank() {
                    boxit = true;
                }
            }
            v.push(res);
        }
    }

    if boxit {
        unsafe {
            Array::alloc_iter(
                &[size_a, size_b],
                v.into_iter().map(|n| scalar::boxed_t(&o.a, n).unwrap()),
            )
            .map(Type::List)
        }
    } else {
        let mut r = r.take().unwrap();
        if r.len() == 1 && r[0] == 1 {
            r.clear();
        }
        r.push(size_a);
        r.push(size_b);
        unsafe {
            match v[0].as_list_unchecked().get_type() {
                b'I' => Array::<i64>::alloc_iter(
                    &r,
                    v.iter()
                        .flat_map(|n| n.as_list_unchecked().treat_as_ref().list().iter())
                        .copied(),
                ),
                b'F' => Array::<OrderedFloat<f64>>::alloc_iter(
                    &r,
                    v.iter()
                        .flat_map(|n| n.as_list_unchecked().treat_as_ref().list().iter())
                        .copied(),
                ),
                b'C' => Array::<u8>::alloc_iter(
                    &r,
                    v.iter()
                        .flat_map(|n| n.as_list_unchecked().treat_as_ref().list().iter())
                        .copied(),
                ),
                b'B' => Array::<bool>::alloc_iter(
                    &r,
                    v.iter()
                        .flat_map(|n| n.as_list_unchecked().treat_as_ref().list().iter())
                        .copied(),
                ),
                b'X' => Array::<Boxed>::alloc_iter(
                    &r,
                    v.iter()
                        .flat_map(|n| n.as_list_unchecked().treat_as_ref().list().iter())
                        .copied(),
                ),
                _ => std::hint::unreachable_unchecked(),
            }
        }
        .map(Type::List)
    }
}

#[inline(always)]
pub fn each_atom_monad(f: &Function, a: Type, o: &Opts) -> Status<Type> {
    if let Type::Map(m) = &a {
        std::hint::cold_path();
        let key_values = m.key_values(&o.a)?;
        let mut f = f.clone();
        f.apply_rm(1);
        return f.monadic_call(Type::List(key_values));
    }

    let Ok(a) = a.to_list() else {
        return err!(Domain);
    };

    if a.get_type() == b'X' {
        maybe_apply!([scalar] o, a, {
            let a = unsafe { a.treat_as::<Boxed>() };
            let trailing = a.rank();
            if f.s.is_some() {
                let mut v = Vec::with_capacity(a.len());
                for_0!(a; [a] {
                    v.push(f.monadic_call(unsafe { a.read() })?.to_list().unwrap());
                });

                // all primitive symbols will yield the same type across a list
                unsafe { List::unranked_unchecked(trailing, v) }
            } else {
                let mut v = Vec::with_capacity(a.len());
                for_0!(a; [a] {
                    v.push(f.monadic_call(unsafe { a.read() })?);
                });

                if v.iter().any(|n| !n.is_list()) {
                    unsafe {
                        Array::try_alloc_iter(
                            trailing,
                            v.into_iter().map(|t| scalar::boxed_t(&o.a, t)),
                        ).map(List::Boxes)
                    }
                } else {
                    let v = v.into_iter().map(|n| unsafe { n.to_list_unchecked() }).collect();
                    List::unranked(&o.a, a.dims(), trailing, v)
                }
            }
        })
        .map(Type::List)
    } else {
        let mut f = f.clone();
        f.apply_rm(0);
        f.monadic_call(Type::List(a))
    }
}

#[inline(always)]
pub fn each_atom_dyad(f: &Function, a: Type, b: Type, o: &Opts) -> Status<Type> {
    let Ok(a) = a.to_list() else {
        return err!(Domain);
    };
    let Ok(b) = b.to_list() else {
        return err!(Domain);
    };

    // SAFETY: Types are checked when necessary
    maybe_apply!([scalar] o, a, b, unsafe {
        if a.is_empty() {
            return Ok(b);
        } else if b.is_empty() {
            return Ok(a);
        }

        let a_maps = a.len() > b.len();
        if a_maps && a.len() % b.len() != 0
            || !a_maps && b.len() % a.len() != 0
        {
            return err!(Domain);
        }

        let trailing;
        let size;
        if a_maps {
            trailing = a.rank();
            size = a.len();
        } else {
            trailing = b.rank();
            size = b.len();
        }

        let mut v = Vec::with_capacity(size);
        if a.get_type() == b'X' && b.get_type() == b'X' {
            for (a, b) in a.listx_unchecked().iter().cycle().zip(b.listx_unchecked().iter().cycle()).take(size) {
                let a = a.read();
                let b = b.read();
                v.push(f.dyadic_call(a, b)?.to_list()?);
            }
        } else if a.get_type() == b'X' {
            for (a, b) in a.listx_unchecked().iter().cycle().zip(b.ranked(0)?.into_iter().cycle()).take(size) {
                let a = a.read();
                let b = Type::List(b);
                v.push(f.dyadic_call(a, b)?.to_list()?);
            }
        } else if b.get_type() == b'X' {
            for (a, b) in a.ranked(0)?.into_iter().cycle().zip(b.listx_unchecked().iter().cycle()).take(size) {
                let a = Type::List(a);
                let b = b.read();
                v.push(f.dyadic_call(a, b)?.to_list()?);
            }
        } else {
            let mut f = f.clone();
            f.apply_rd([0, 0]);
            return f.dyadic_call(Type::List(a), Type::List(b))?.to_list();
        }
        List::unranked(&o.a, trailing.len(), trailing, v)
    })
    .map(Type::List)
}
