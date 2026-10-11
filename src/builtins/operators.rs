use crate::function::{Function, function};
use crate::scope::{MaybeInstant, Scope};
use crate::utils::Arena;
use crate::utils::error::{Status, err};

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
pub fn fold(f: MaybeInstant, _a: Arena, _s: Scope) -> Status<Function> {
    println!("FOLD: {f:?}");
    todo!()
}
