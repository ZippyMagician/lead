use std::ops::*;

use crate::builtins::symbols::Symbol;
use crate::function::{Function, Opts, function};
use crate::list::List;
use crate::scalar::Scalar;
use crate::scope::{MaybeInstant, Scope};
use crate::utils::Arena;
use crate::utils::error::{Status, err};
use crate::utils::function::*;

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
}

#[inline]
pub(crate) fn atop_bind(
    l: MaybeInstant,
    r: MaybeInstant,
    arena: Arena,
    scope: Scope,
) -> Status<Function> {
    if l.is_instant() && r.is_instant() {
        return err!(Domain);
    }

    if l.is_instant() {
        let l = l.to_instant()?;
        let r = r.to_function(arena.clone(), scope.clone())?;

        let lr = l.clone();
        let rr = r.clone();
        Ok(function!(
            move |a, _| r.dyadic_call(l.clone(), a);
            move |_, b, _| rr.dyadic_call(lr.clone(), b);
            arena.clone(), scope.clone()
        ))
    } else if r.is_instant() {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_instant()?;

        let lr = l.clone();
        let rr = r.clone();
        Ok(function!(
            move |a, _| l.dyadic_call(a, r.clone());
            move |a, _, _| lr.dyadic_call(a, rr.clone());
            arena.clone(), scope.clone()
        ))
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;

        let lr = l.clone();
        let rr = r.clone();
        Ok(function!(
            move |a, _| l.monadic_call(r.monadic_call(a)?);
            move |a, b, _| lr.monadic_call(rr.dyadic_call(a, b)?);
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub(crate) fn hook_default(
    l: MaybeInstant,
    r: MaybeInstant,
    arena: Arena,
    scope: Scope,
) -> Status<Function> {
    if l.is_instant() && r.is_instant() {
        return err!(Domain);
    }

    if l.is_instant() {
        let l = l.to_instant()?;
        let r = r.to_function(arena.clone(), scope.clone())?;

        let rr = r.clone();
        Ok(function!(
            move |a, _| r.dyadic_call(l.clone(), a);
            move |a, b, _| rr.dyadic_call(a, b);
            arena.clone(), scope.clone()
        ))
    } else if r.is_instant() {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_instant()?;

        let lr = l.clone();
        Ok(function!(
            move |a, _| l.dyadic_call(a, r.clone());
            move |a, b, _| lr.dyadic_call(a, b);
            arena.clone(), scope.clone()
        ))
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;

        let lr = l.clone();
        let rr = r.clone();
        Ok(function!(
            move |a, _| l.dyadic_call(a.clone(), r.monadic_call(a)?);
            move |a, b, _| lr.dyadic_call(a, rr.monadic_call(b)?);
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub(crate) fn over_rank(
    l: MaybeInstant,
    r: MaybeInstant,
    arena: Arena,
    scope: Scope,
) -> Status<Function> {
    if l.is_instant() {
        return err!(Domain);
    }

    if r.is_instant() {
        let mut l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_instant()?;

        if r.is_empty() || r.len() > 2 {
            return err!(Domain);
        } else if r.is_scalar() {
            let n = r.list()[0].to_num()?;
            l.apply_rm(n as u8);
            l.apply_rd([n as u8, n as u8]);
        } else {
            let a = r.list()[0].to_num()?;
            let b = r.list()[1].to_num()?;
            l.apply_rd([a as u8, b as u8]);
        }

        let lr = l.clone();
        Ok(function!(
            move |a, _| l.monadic_call(a);
            move |a, b, _| lr.dyadic_call(a, b);
            arena.clone(), scope.clone()
        ))
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;

        let lr = l.clone();
        let rr = r.clone();
        Ok(function!(
            move |a, _| l.monadic_call(r.monadic_call(a)?);
            move |a, b, _| lr.dyadic_call(rr.monadic_call(a)?, rr.monadic_call(b)?);
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub fn fold(f: MaybeInstant, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    let fr = f.clone();
    let monad = move |a: List, o: &Opts| {
        if a.is_empty() {
            std::hint::cold_path();
            return Ok(a);
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
                let mut acc = List::scalar(items[0]);
                for item in items.iter().skip(1) {
                    acc = f.dyadic_call(acc, List::scalar(*item))?;
                }
                return Ok(acc);
            }

            let d = a.dims();
            let mut items = a.ranked(d - 1)?.into_iter();
            let mut acc = items.next().unwrap();
            for item in items {
                acc = f.dyadic_call(acc, item)?;
            }
            Ok(acc)
        })
    };
    let f = fr;

    let dyad = move |a: List, b: List, o: &Opts| {
        if b.is_scalar() {
            return Ok(b);
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
                cells[i] = f.dyadic_call(previous, arr[k].clone())?;
            }
            j += jump;
        }
        List::unranked(&o.a, dims, &[], cells)
    };

    Ok(function!(1, true, monad; [0, 99], false, dyad; a, s))
}
