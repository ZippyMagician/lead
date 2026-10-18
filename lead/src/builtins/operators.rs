use std::ops::*;

use crate::builtins::symbols::Symbol;
use crate::function::{Function, Opts, function};
use crate::scope::Scope;
use crate::types::list::List;
use crate::types::scalar::Scalar;
use crate::types::{Type, Value};
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
    /// `\``
    TieCommute,
}

#[inline]
pub(crate) fn atop_bind(l: Type, r: Type, arena: Arena, scope: Scope) -> Status<Function> {
    if l.is_instant() && r.is_instant() {
        return err!(Domain);
    }

    if l.is_value() {
        let l = l.to_value()?;
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| r.dyadic_call(l.clone(), b.unwrap_or(a));
            arena.clone(), scope.clone()
        ))
    } else if r.is_value() {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_value()?;
        Ok(function!(
            move |a, _, _| l.dyadic_call(a, r.clone());
            arena.clone(), scope.clone()
        ))
    } else {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| l.monadic_call(r.call_fn(a, b)?.to_value()?);
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub(crate) fn hook_default(l: Type, r: Type, arena: Arena, scope: Scope) -> Status<Function> {
    if l.is_instant() && r.is_instant() {
        return err!(Domain);
    }

    if l.is_value() {
        let l = l.to_value()?;
        let r = r.to_function(arena.clone(), scope.clone())?;
        Ok(function!(
            move |a, b, _| if let Some(b) = b {
                r.dyadic_call(a, b)
            } else { r.dyadic_call(l.clone(), a) };
            arena.clone(), scope.clone()
        ))
    } else if r.is_value() {
        let l = l.to_function(arena.clone(), scope.clone())?;
        let r = r.to_value()?;
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
                l.dyadic_call(a, r.monadic_call(b)?.to_value()?)
            } else { l.dyadic_call(a.clone(), r.monadic_call(a)?.to_value()?) };
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
        let r = r.to_instant()?;

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
                r.monadic_call(a)?.to_value()?,
                b.map(|b|
                    r.monadic_call(b).and_then(|v| v.to_value())
                ).transpose()?,
            );
            arena.clone(), scope.clone()
        ))
    }
}

#[inline]
pub fn fold(f: Type, a: Arena, s: Scope) -> Status<Function> {
    let f = f.to_function(a.clone(), s.clone())?;
    let fr = f.clone();
    let monad = move |a: Value, o: &Opts| {
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
                let mut acc = Value::List(List::scalar(items[0]));
                for item in items.iter().skip(1) {
                    acc = f.dyadic_call(acc, Value::List(List::scalar(*item)))?.to_value()?;
                }
                return acc.to_list();
            }

            let d = a.dims();
            let mut items = a.ranked(d - 1)?.into_iter();
            let mut acc = Value::List(items.next().unwrap());
            for item in items {
                acc = f.dyadic_call(acc, Value::List(item))?.to_value()?;
            }
            acc.to_list()
        })
        .map(Value::List)
    };
    let f = fr;

    let dyad = move |a: Value, b: Value, o: &Opts| {
        let a = a.to_list()?;
        let b = b.to_list()?;
        apply_loosed(&o.a, a, b, o.rd, |a, b| {
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
                    cells[i] = f
                        .dyadic_call(Value::List(previous), Value::List(arr[k].clone()))?
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
