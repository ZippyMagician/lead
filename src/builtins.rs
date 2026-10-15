pub mod operators;
pub mod symbols;

use super::function::function;

use operators::Operator;
use symbols::Symbol;

macro_rules! _len {
    ($obj:literal) => { 1 };
    () => { 0 };
    ($obj:literal, $($objs:literal),+) => { 1 + _len!($($objs),+) }
}

macro_rules! define_syms {
    ($($rep:ident => ($tok:literal, $rm:literal, $am:ident, $vm:ident, $monad:expr; $rd:expr, $ad:ident, $vd:ident, $dyad:expr));*;) => {
        impl Symbol {
            pub fn get_symbol(s: &str) -> Option<Self> {
                match s.as_bytes() {
                    $($tok => Some(Self::$rep)),*,
                    _ => None
                }
            }

            pub fn getfn(&self, a: crate::utils::Arena, s: crate::scope::Scope) -> crate::function::Function {
                match self {
                    $(Self::$rep => {
                        function!(
                            $rm, $am, $vm, $monad; $rd, $ad, $vd, $dyad; a, s, Some(crate::builtins::symbols::Symbol::$rep)
                        )
                    }),*
                }
            }
        }

        pub const SYMBOLS: [&[u8]; _len!($($tok),*)] = [$($tok),*];
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

    ($($rep:ident => ($tok:literal, $flag:ident, $func:expr));*;) => {
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
                crate::utils::error::err!(Internal, "TODO: <builtins.rs> @ Operator::getop")
            }
        }

        pub const OPERATORS: [&[u8]; _len!($($tok),*)] = [$($tok),*];
    }
}

define_syms! {
    Plus => (b"+", 0, false, false, symbols::abs; [0, 0], false, false, symbols::add);
    Dash => (b"-", 0, false, false, symbols::negate; [0, 0], false, false, symbols::sub);
    Star => (b"*", 0, false, false, symbols::signum; [0, 0], false, false, symbols::mul);
    Percent => (b"%", 0, false, false, symbols::reciprocal; [0, 0], false, false, symbols::div);
    Pipe => (b"|", 0, false, false, symbols::not; [0, 0], false, false, symbols::residue);
    Exclamation => (b"!", 0, false, false, symbols::factorial; [0, 0], false, false, symbols::bitor);
    LeftArrow => (b"<", 99, false, true, symbols::boxed; [0, 0], false, false, symbols::lt);
    RightArrow => (b">", 99, false, true, symbols::unbox; [0, 0], false, false, symbols::gt);
    Caret => (b"^", 0, false, false, symbols::exp; [0, 0], false, false, symbols::bitand);
    Dollar => (b"$", 99, false, false, symbols::shape; [1, 99], true, false, symbols::reshape);
    LeftBracket => (b"[", 99, false, true, symbols::identity; [99, 99], false, true, symbols::left);
    RightBracket => (b"]", 99, false, true, symbols::identity; [99, 99], false, true, symbols::right);
    Question => (b"?", 99, true, false, symbols::dedup_sieve; [99, 1], true, false, symbols::group);
    Tilde => (b"~", 0, false, false, symbols::range; [99, 0], true, false, symbols::index);
}

define_op! {
    AtopBind => (b"&", false, operators::atop_bind);
    HookDefault => (b"&:", false, operators::hook_default);
    OverRank => (b"@", false, operators::over_rank);
    Fold => (b"/", true, operators::fold);
}
