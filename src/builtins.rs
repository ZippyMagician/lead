#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Symbol {
    Plus,
    Dash,
    Star,
    Percent,
    Pipe,
}

impl Symbol {
    pub fn get_symbol(s: &str) -> Option<Symbol> {
        match s.as_bytes() {
            b"+" => Some(Self::Plus),
            b"-" => Some(Self::Dash),
            b"*" => Some(Self::Star),
            b"%" => Some(Self::Percent),
            b"|" => Some(Self::Pipe),
            _ => None,
        }
    }
}

