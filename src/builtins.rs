pub mod symbols;

use self::symbols::Symbol;

macro_rules! function {
    ($rm:literal, $monad:expr; $rd:expr, $dyad:expr, $a:ident, $s:ident) => {{
        crate::function::Function::new(
            ::std::rc::Rc::new($monad),
            ::std::rc::Rc::new($dyad),
            $rm,
            $rd,
            $a,
            $s,
        )
    }};
}

macro_rules! define_syms {
    ($($rep:ident => ($tok:pat, $rm:literal, $monad:expr; $rd:expr, $dyad:expr));*;) => {
        impl Symbol {
            pub fn get_symbol(s: &str) -> Option<Self> {
                match s.as_bytes() {
                    $($tok => Some(Self::$rep)),*,
                    _ => None
                }
            }

            pub fn getfn(&self, a: crate::utils::Arena, s: crate::scope::Scope) -> crate::function::Function {
                match self {
                    $(Self::$rep => { function!($rm, $monad; $rd, $dyad, a, s) }),*
                }
            }
        }
    }
}

define_syms! {
    Plus => (b"+", 0, symbols::abs; [0, 0], symbols::add);
    Dash => (b"-", 0, symbols::negate; [0, 0], symbols::sub);
    Star => (b"*", 0, symbols::signum; [0, 0], symbols::mul);
    Percent => (b"%", 0, symbols::reciprocal; [0, 0], symbols::div);
    Pipe => (b"|", 0, symbols::not; [0, 0], symbols::residue);
    Tilde => (b"~", 0, symbols::range; [99, 0], symbols::index);
    LeftArrow => (b"<", 99, symbols::boxed; [0, 0], symbols::lt);
    Dollar => (b"$", 99, symbols::shape; [1, 99], symbols::reshape);
}
