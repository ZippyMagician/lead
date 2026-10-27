use std::ops::*;

use crate::builtins::symbols::Symbol;
use crate::function::{Function, Opts, function};
use crate::scope::Scope;
use crate::types::Type;
use crate::types::list::List;
use crate::types::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Status, err};
use crate::utils::function::*;
use crate::utils::symbols::{maybe_apply, with_eco};

use smallvec::SmallVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(C)]
pub enum Operator {
    #[default]
    Null,
    /// `&`
    AtopBind,
    /// `&:`
    HookDefault,
    /// `@`
    OverRank,
    /// `/`
    Fold,
    /// `\\`
    ScanTable,
    /// `\``
    TieCommute,
    /// `"`
    Each,
    /// `":`
    EachAtom,
    /// `".`
    RepeatUntil,
}

#[inline]
pub(crate) fn atop_bind(l: Type, r: Type, arena: Arena, scope: Scope) -> Status<Function> {
    if l.is_instant() && r.is_instant() {
        return err!(Domain);
    }

    if l.is_instant() {
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| r.dyadic_call(l.clone(), b.unwrap_or(a));
            arena.clone(), scope.clone()
        ))
    } else if r.is_instant() {
        let l = l.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, _, _| l.dyadic_call(a, r.clone());
            arena.clone(), scope.clone()
        ))
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| l.monadic_call(r.call_fn(a, b)?);
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub(crate) fn hook_default(l: Type, r: Type, arena: Arena, scope: Scope) -> Status<Function> {
    if l.is_instant() && r.is_instant() {
        return err!(Domain);
    }

    if l.is_instant() {
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| if let Some(b) = b {
                r.dyadic_call(a, b)
            } else { r.dyadic_call(l.clone(), a) };
            arena.clone(), scope.clone()
        ))
    } else if r.is_instant() {
        let l = l.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _|
                l.dyadic_call(a, b.unwrap_or_else(|| r.clone()));
            arena.clone(), scope.clone()
        ))
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| if let Some(b) = b {
                l.dyadic_call(a, r.monadic_call(b)?)
            } else { l.dyadic_call(a.clone(), r.monadic_call(a)?) };
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub(crate) fn over_rank(l: Type, r: Type, arena: Arena, scope: Scope) -> Status<Function> {
    if l.is_instant() {
        return err!(Domain);
    }

    if r.is_instant() {
        let mut l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_list()?;

        if r.is_empty() || r.len() > 2 {
            return err!(Domain);
        } else if r.is_scalar() {
            let mut n = r.list()[0].to_num()?;
            if n > 99 {
                n = 99
            }
            l.apply_rm(n as u8);
            l.apply_rd([n as u8, n as u8]);
        } else {
            let mut a = r.list()[0].to_num()?;
            let mut b = r.list()[1].to_num()?;
            if a > 99 {
                a = 99
            }
            if b > 99 {
                b = 99
            }
            l.apply_rd([a as u8, b as u8]);
        }
        Ok(l)
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| l.call_fn(
                r.monadic_call(a)?,
                b.map(|b| r.monadic_call(b)).transpose()?,
            );
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub fn fold(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    let fr = f.clone();
    let monad = move |a: Type, o: &Opts| {
        let a = a.to_list()?;
        if a.is_empty() {
            std::hint::cold_path();
            return Ok(a.into());
        }

        apply_loosem(&o.a, a, o.rm, |a| {
            if a.is_scalar() {
                std::hint::cold_path();
                return Ok(a);
            }

            if o.rm == 1 && a.dims() == 1 {
                if let Some(s) = &f.s {
                    let list = a.list();
                    let mut acc = list[0];
                    match s {
                        Symbol::Plus => {
                            for n in list.iter().skip(1) {
                                acc = acc.add(*n)?;
                            }
                            return Ok(List::scalar(acc));
                        }
                        Symbol::Dash => {
                            let mut acc = list[0];
                            for n in list.iter().skip(1) {
                                acc = acc.sub(*n)?;
                            }
                            return Ok(List::scalar(acc));
                        }
                        Symbol::Star => {
                            let mut acc = list[0];
                            if acc.is_bool() {
                                for n in list.iter().skip(1) {
                                    if !unsafe { acc.to_bool_unchecked() } {
                                        break;
                                    }
                                    acc = Scalar::Boolean(
                                        unsafe { acc.to_bool_unchecked() }
                                            && unsafe { n.to_bool_unchecked() },
                                    );
                                }
                            } else {
                                for n in list.iter().skip(1) {
                                    acc = acc.mul(*n)?;
                                }
                            }
                            return Ok(List::scalar(acc));
                        }
                        Symbol::Percent => {
                            let mut acc = list[0];
                            for n in list.iter().skip(1) {
                                acc = acc.div(*n)?;
                            }
                            return Ok(List::scalar(acc));
                        }
                        Symbol::Pipe => {
                            let mut acc = list[0];
                            if acc.is_bool() {
                                for n in list.iter().skip(1) {
                                    if unsafe { acc.to_bool_unchecked() } {
                                        break;
                                    }
                                    acc = Scalar::Boolean(
                                        unsafe { acc.to_bool_unchecked() }
                                            || unsafe { n.to_bool_unchecked() },
                                    );
                                }
                            } else {
                                for n in list.iter().skip(1) {
                                    acc = acc.rem(*n)?;
                                }
                            }
                            return Ok(List::scalar(acc));
                        }
                        _ => {}
                    }
                }
                let items = a.list();
                let mut acc = Type::Scalar(items[0]);
                for item in items.iter().skip(1) {
                    acc = f.dyadic_call(acc, Type::Scalar(*item))?;
                }
                return acc.to_list();
            }

            let d = a.dims();
            let mut items = a.ranked(d - 1)?.into_iter();
            let mut acc = Type::from(items.next().unwrap());
            for item in items {
                acc = f.dyadic_call(acc, Type::from(item))?;
            }
            acc.to_list()
        })
    };
    let f = fr;

    let dyad = move |a: Type, b: Type, o: &Opts| {
        let a = a.to_list()?;
        let b = b.to_list()?;
        apply_loosed(&o.a, a, b, o.rd, |a, b| {
            if b.is_scalar() {
                return Ok(List::scalar(b.list()[0]));
            }
            if !a.is_scalar() {
                return err!(Domain);
            }

            let window = a.list()[0].to_num()?;
            let mut unique = false;
            let window = if window < 0 {
                unique = true;
                (-window) as usize
            } else {
                window as usize
            };

            let dims = b.dims();
            let mut arr = b.ranked(dims - 1)?;
            let jump = if unique { window } else { 1 };

            let mut cells = vec![List::empty(); 1 + (arr.len() - window) / jump];
            let mut j = 0;
            while j <= arr.len() - window {
                let i = j / jump;
                cells[i] = std::mem::take(&mut arr[j]);
                for k in j + 1..j + window {
                    let previous = std::mem::take(&mut cells[i]);
                    cells[i] = f
                        .dyadic_call(Type::from(previous), Type::from(arr[k].clone()))?
                        .to_list()?;
                }
                j += jump;
            }
            List::unranked(&o.a, cells[0].dims() + 1, &[], cells)
        })
    };

    Ok(function!(1, [0, 99], true, true, move |a, b, o| {
        if let Some(b) = b {
            dyad(a, b, o).map(Type::from)
        } else {
            monad(a, o).map(Type::from)
        }
    }; a, s))
}

#[inline]
pub(crate) fn scan_table(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    let fr = f.clone();
    let monad = move |a: Type, o: &Opts| {
        if !a.is_list() {
            return err!(Domain);
        }
        if a.is_scalar() || o.rd[1] == 0 {
            return Ok(a);
        }

        let a = unsafe { a.to_list_unchecked() };
        maybe_apply!(a.dims() <= o.rm as usize, &o.a, a, o.rm, {
            if a.len() == 1 {
                return Ok(a);
            }
            let mut runs = a.ranked(a.dims() - 1)?;
            if runs.len() == 1 {
                return Ok(runs.into_iter().next().unwrap());
            }

            let mut b = Vec::with_capacity(runs.len());
            b.push(std::mem::take(&mut runs[0]));
            let mut boxit = false;
            let mut r: SmallVec<[usize; 3]> = SmallVec::from_slice(b[0].rank());
            for i in 1..runs.len() {
                let res = f
                    .dyadic_call(b[i - 1].clone().into(), std::mem::take(&mut runs[i]).into())?
                    .to_list()?;
                if res.rank() != &*r {
                    boxit = true;
                }
                b.push(res);
            }

            if boxit {
                unsafe {
                    List::alloc_iter(
                        &[runs.len()],
                        b.into_iter().map(|n| Scalar::boxed(&o.a, n).unwrap()),
                    )
                }
            } else {
                if b[0].is_scalar() {
                    r.clear();
                }
                r.push(runs.len());
                unsafe { List::alloc_iter(&r, b.iter().flat_map(|n| n.list().iter()).copied()) }
            }
        })
        .map(Type::List)
    };
    let f = fr;
    let dyad = move |a: Type, b: Type, o: &Opts| {
        if !b.is_list() || !a.is_list() {
            return err!(Domain);
        }

        let a = a.to_list()?;
        let b = b.to_list()?;
        let rda = u8::max(u8::min(a.dims() as u8, o.rd[0]), 1) - 1;
        let rdb = u8::max(u8::min(b.dims() as u8, o.rd[0]), 1) - 1;
        let a = a.ranked(rda as usize)?;
        let b = b.ranked(rdb as usize)?;

        let size_a = a.len();
        let size_b = b.len();

        let mut boxit = false;
        let mut v = Vec::with_capacity(a.len() * b.len());
        let mut r = None;
        for a in a {
            for b in b.iter() {
                let res = f.dyadic_call(a.clone().into(), b.clone().into())?;
                if !res.is_list() && !boxit {
                    boxit = true;
                } else if res.is_scalar() {
                    if r.is_none() {
                        std::hint::cold_path();
                        r = Some(vec![1]);
                    } else if !boxit && r.as_ref().unwrap() != &[1] {
                        boxit = true;
                    }
                } else {
                    let l = res.as_list()?;
                    if r.is_none() {
                        std::hint::cold_path();
                        r = Some(l.rank().to_vec());
                    } else if !boxit && r.as_ref().unwrap() != l.rank() {
                        boxit = true;
                    }
                }
                v.push(res);
            }
        }

        if boxit {
            unsafe {
                List::alloc_iter(
                    &[size_a, size_b],
                    v.into_iter().map(|n| Scalar::boxed_t(&o.a, n).unwrap()),
                )
            }
        } else {
            let mut r = r.take().unwrap();
            if r.len() == 1 && r[0] == 1 {
                r.clear();
            }
            r.push(size_a);
            r.push(size_b);
            unsafe {
                List::alloc_iter(
                    &r,
                    v.iter().flat_map(|l| l.as_list().unwrap_unchecked().list().iter().copied()),
                )
            }
        }
    };
    Ok(function!(99, [99, 99], true, true, move |a, b, o| {
        if let Some(b) = b {
            dyad(a, b, o).map(Type::List)
        } else {
            monad(a, o)
        }
    }; a, s))
}

#[inline]
pub(crate) fn tie_commute(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    Ok(function!(99, [99, 99], false, true, move |a, b, _| {
        if let Some(b) = b {
            f.dyadic_call(b, a)
        } else {
            f.dyadic_call(a.clone(), a)
        }
    }; a, s))
}

#[inline]
pub(crate) fn each(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    Ok(function!(99, [99, 99], true, true, move |a, b, o| {
        let mut f = f.clone();
        let keys_a = if a.is_map() { a.to_map()?.key_values(&o.a) } else { a.to_list() }?;
        if let Some(b) = b {
            let keys_b = if b.is_map() { b.to_map()?.key_values(&o.a) } else { b.to_list() }?;
            let da = usize::max(1, usize::min(o.rd[0] as usize, keys_a.dims()));
            let db = usize::max(1, usize::min(o.rd[1] as usize, keys_b.dims()));
            f.apply_rd([da as u8 - 1, db as u8 - 1]);
            f.dyadic_call(Type::List(keys_a), Type::List(keys_b))
        } else {
            let da = usize::max(1, usize::min(o.rm as usize, keys_a.dims()));
            f.apply_rm(da as u8 - 1);
            f.monadic_call(Type::List(keys_a))
        }
    }; a, s))
}

#[inline]
pub(crate) fn eachatom(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let mut f = f.to_function(a.clone(), s.clone())?;
    f.apply_rm(0);
    f.apply_rd([0, 0]);
    Ok(f)
}

#[inline]
pub(crate) fn repeat_until(l: Type, r: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = l.to_function(a.clone(), s.clone())?;

    if r.is_scalar() {
        let count = if r.is_scalar_strict() {
            unsafe { r.to_scalar_unchecked() }.to_num()?
        } else {
            unsafe { r.to_list_unchecked() }.list()[0].to_num()?
        };
        Ok(function!(99, [99, 99], true, true, move |mut a, b, o| {
            let mut f = f.clone();
            if let Some(mut b) = b {
                if o.rd != [99, 99] {
                    f.apply_rd(o.rd);
                }
                for _ in 0..count {
                    b = f.dyadic_call(a.clone(), b)?;
                }
                Ok(b)
            } else {
                if o.rm != 99 { 
                    f.apply_rm(o.rm);
                }
                for _ in 0..count {
                    a = f.monadic_call(a)?;
                }
                Ok(a)
            }
        }; a, s))
    } else if r.is_list() {
        let counts = unsafe { r.to_list_unchecked() }
            .list()
            .into_iter()
            .map(|n| n.to_num())
            .collect::<Status<Vec<i64>>>()?;
        Ok(function!(99, [99, 99], true, true, move |a, b, o| {
            let mut f = f.clone();
            if let Some(b) = b {
                if o.rd != [99, 99] {
                    f.apply_rd(o.rd);
                }
                let v = with_eco(counts.len(), |v| {
                    for &count in counts.iter() {
                        let mut b = b.clone();
                        for _ in 0..count {
                            b = f.dyadic_call(a.clone(), b)?;
                        }
                        v.push(Scalar::boxed_t(&o.a, b)?);
                    }
                    Ok(())
                })?;
                List::from_vec(v).map(Type::List)
            } else {
                if o.rm != 99 { 
                    f.apply_rm(o.rm);
                }
                let v = with_eco(counts.len(), |v| {
                    for &count in counts.iter() {
                        let mut a = a.clone();
                        for _ in 0..count {
                            a = f.monadic_call(a)?;
                        }
                        v.push(Scalar::boxed_t(&o.a, a)?);
                    }
                    Ok(())
                })?;
                List::from_vec(v).map(Type::List)
            }
        }; a, s))
    } else if r.is_function() {
        let cond = unsafe { r.as_function_unchecked().clone() };
        Ok(function!(99, [99, 99], true, true, move |mut a, b, o| {
            let mut f = f.clone();
            if let Some(b) = b {
                let mut p = f.dyadic_call(a, b.clone())?;
                if o.rd != [99, 99] {
                    f.apply_rd(o.rd);
                }
                loop {
                    let list = cond.monadic_call(p.clone())?.to_list()?;
                    if list.is_empty() || list.list()[0].to_bool()? {
                        break;
                    }
                    p = f.dyadic_call(p, b.clone())?;
                }
                Ok(p)
            } else {
                let mut p;
                if o.rm != 99 {
                    f.apply_rm(o.rm);
                }
                loop {
                    p = a.clone();
                    a = f.monadic_call(a)?;
                    let list = cond.dyadic_call(p, a.clone())?.to_list()?;
                    if list.is_empty() || list.list()[0].to_bool()? {
                        break;
                    }
                }
                Ok(a)
            }
        }; a, s))
    } else {
        err!(Domain)
    }
}
