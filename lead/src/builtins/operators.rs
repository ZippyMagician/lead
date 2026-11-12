use crate::array::each;
use crate::function::{Function, function};
use crate::scope::Scope;
use crate::types::Type;
use crate::types::list::{Array, List};
use crate::types::scalar::{self, Scalar};
use crate::utils::Arena;
use crate::utils::error::{Status, err};
use crate::utils::list::iter_ints;
use crate::utils::symbols::with_eco;

use smallvec::smallvec;

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
            let mut n = iter_ints!(r, |it| it.next().unwrap());
            if n > 99 {
                n = 99
            }
            if n < 0 {
                return err!(Domain);
            }
            l.apply_rm(n as u8);
            l.apply_rd([n as u8, n as u8]);
        } else if r.len() == 2 {
            let mut a;
            let mut b;
            iter_ints!(r, |it| {
                a = it.next().unwrap();
                b = it.next().unwrap();
            });
            if a > 99 {
                a = 99
            }
            if b > 99 {
                b = 99
            }
            if a < 0 || b < 0 {
                return err!(Domain);
            }
            l.apply_rd([a as u8, b as u8]);
        } else {
            return err!(Domain);
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

    Ok(function!(1, [0, 99], true, true, move |a, b, o| {
        if let Some(b) = b {
            each::reduce_groups(&f, a, b, o)
        } else {
            each::reduce(&f, a, o)
        }
    }; a, s))
}

#[inline]
pub(crate) fn scan_table(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    Ok(function!(99, [99, 99], true, true, move |a, b, o| {
        if let Some(b) = b {
            each::table(&f, a, b, o)
        } else {
            each::scan(&f, a, o)
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
    let f = f.to_function(a.clone(), s.clone())?;
    Ok(function!(0, [0, 0], true, true, move |a, b, o| {
        if let Some(b) = b {
            each::each_atom_dyad(&f, a, b, o)
        } else {
            each::each_atom_monad(&f, a, o)
        }
    }; a, s))
}

#[inline]
pub(crate) fn repeat_until(l: Type, r: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = l.to_function(a.clone(), s.clone())?;

    if r.is_scalar() {
        let count = iter_ints!(unsafe { r.to_list_unchecked() }, |it| it.next().unwrap());
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
        let counts = iter_ints!(unsafe { r.to_list_unchecked() }, |it| it.collect::<Vec<i64>>());
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
                        v.push(scalar::boxed_t(&o.a, b)?);
                    }
                    Ok(())
                })?;
                Ok(Type::List(List::Boxes(Array::new(v, 1, smallvec![counts.len()]))))
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
                        v.push(scalar::boxed_t(&o.a, a)?);
                    }
                    Ok(())
                })?;
                Ok(Type::List(List::Boxes(Array::new(v, 1, smallvec![counts.len()]))))
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
                    if list.is_truthy() {
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
                    if list.is_truthy() {
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
