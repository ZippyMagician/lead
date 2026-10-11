pub mod operators;
pub mod symbols;

use super::function::function;

use operators::Operator;
use symbols::Symbol;

macro_rules! define_syms {
    ($($rep:ident => ($tok:pat, $rm:literal, $am:ident, $monad:expr; $rd:expr, $ad:ident, $dyad:expr));*;) => {
        impl Symbol {
            pub fn get_symbol(s: &str) -> Option<Self> {
                match s.as_bytes() {
                    $($tok => Some(Self::$rep)),*,
                    _ => None
                }
            }

            pub fn getfn(&self, a: crate::utils::Arena, s: crate::scope::Scope) -> crate::function::Function {
                match self {
                    $(Self::$rep => { function!($rm, $am, $monad; $rd, $ad, $dyad; a, s) }),*
                }
            }
        }
    }
}

macro_rules! define_op {
    (@emit, $self:ident, $rep:ident, true, $func:expr, $l:expr, $r:expr, $a:ident, $s:ident) => {
        if *$self == Self::$rep {
            return $func($l, $a, $s);
        }
    };

    (@emit, $self:ident, $rep:ident, false, $func:expr, $l:expr, $r:expr, $a:ident, $s:ident) => {
        if *$self == Self::$rep {
            return $func($l, $r.unwrap(), $a, $s);
        }
    };

    ($($rep:ident => ($tok:pat, $flag:ident, $func:expr));*;) => {
        impl Operator {
            pub fn get_operator(s: &str) -> Option<Self> {
                match s.as_bytes() {
                    $($tok => Some(Self::$rep)),*,
                    _ => None
                }
            }

            pub fn is_monadic(&self) -> bool {
                match &self {
                    $(Self::$rep => $flag),*,
                    _ => todo!("<builtins.rs> @ Operator::is_monadic"),
                }
            }

            pub fn getop(
                &self,
                l: crate::scope::MaybeInstant,
                r: Option<crate::scope::MaybeInstant>,
                a: crate::utils::Arena,
                s: crate::scope::Scope,
            ) -> crate::utils::error::Status<crate::function::Function> {
                $(define_op!(@emit, self, $rep, $flag, $func, l, r, a, s));*;
                todo!("<builtins.rs> @ Operator::getop")
            }
        }
    }
}

define_syms! {
    Plus => (b"+", 0, false, symbols::abs; [0, 0], false, symbols::add);
    Dash => (b"-", 0, false, symbols::negate; [0, 0], false, symbols::sub);
    Star => (b"*", 0, false, symbols::signum; [0, 0], false, symbols::mul);
    Percent => (b"%", 0, false, symbols::reciprocal; [0, 0], false, symbols::div);
    Pipe => (b"|", 0, false, symbols::not; [0, 0], false, symbols::residue);
    Tilde => (b"~", 0, false, symbols::range; [99, 0], true, symbols::index);
    LeftArrow => (b"<", 99, false, symbols::boxed; [0, 0], false, symbols::lt);
    RightArrow => (b">", 99, false, symbols::unbox; [0, 0], false, symbols::gt);
    Dollar => (b"$", 99, false, symbols::shape; [1, 99], true, symbols::reshape);
}

define_op! {
    AtopBind => (b"&", false, operators::atop_bind);
    HookDefault => (b"&:", false, operators::hook_default);
    OverRank => (b"@", false, operators::over_rank);
    Fold => (b"/", true, operators::fold);
}
