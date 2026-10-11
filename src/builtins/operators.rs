use crate::function::{Function, function};
use crate::scope::{Scope, MaybeInstant};
use crate::utils::error::{Status, err};
use crate::utils::Arena;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Operator {
    /// `&`
    AtopBind,
    /// `&:`
    HookDefault,
    /// `@`
    OverRank,
    /// `/`
    Fold,
}

pub(crate) fn atop_bind(l: MaybeInstant, r: MaybeInstant, arena: Arena, scope: Scope) -> Status<Function> {
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

