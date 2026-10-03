pub mod symbols;

use self::symbols::Symbol;

macro_rules! function {
    ($rm:literal, $monad:expr; $rd:expr, $dyad:expr) => {{
        crate::function::Function::new(
            ::std::rc::Rc::new($monad),
            ::std::rc::Rc::new($dyad),
            $rm,
            $rd,
        )
    }};
}

macro_rules! define_syms {
    ($($rep:ident => ($tok:pat, $def:expr));*;) => {
        impl Symbol {
            pub fn get_symbol(s: &str) -> Option<Self> {
                match s.as_bytes() {
                    $($tok => Some(Self::$rep)),*,
                    _ => None
                }
            }

            pub fn getfn(&self) -> crate::function::Function {
                match self {
                    $(Self::$rep => { $def }),*
                }
            }
        }
    }
}


define_syms! {
    Plus => (b"+", function!(0, symbols::abs; [0, 0], symbols::add));
    Dash => (b"-", function!(0, symbols::negate; [0, 0], symbols::sub));
    Star => (b"*", function!(0, symbols::signum; [0, 0], symbols::mul));
    Percent => (b"%", function!(0, symbols::reciprocal; [0, 0], symbols::div));
    Pipe => (b"|", function!(0, symbols::not; [0, 0], symbols::residue));
}

