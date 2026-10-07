pub mod symbols;

use self::symbols::Symbol;

macro_rules! function {
    ($rm:literal, $monad:expr; $rd:expr, $dyad:expr, $s:ident) => {{
        crate::function::Function::new(
            ::std::rc::Rc::new($monad),
            ::std::rc::Rc::new($dyad),
            $rm,
            $rd,
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

            pub fn getfn(&self, s: crate::scope::Scope) -> crate::function::Function {
                match self {
                    $(Self::$rep => { function!($rm, $monad; $rd, $dyad, s) }),*
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
}
