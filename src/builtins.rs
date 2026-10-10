pub mod symbols;

use self::symbols::Symbol;

macro_rules! function {
    ($rm:literal, $am:ident, $monad:expr; $rd:expr, $ad:ident, $dyad:expr; $a:ident, $s:ident) => {{
        crate::function::Function::new_applied(
            ::std::rc::Rc::new($monad),
            ::std::rc::Rc::new($dyad),
            $rm,
            $rd,
            $am,
            $ad,
            $a,
            $s,
        )
    }};
}

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
