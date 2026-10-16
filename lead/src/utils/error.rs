use std::borrow::Cow;
use std::error;
use std::fmt::{self, Display};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Clone, Debug)]
pub enum Error {
    Complete,
    NaN,
    MemoryFault(Cow<'static, str>),
    Syntax(Cow<'static, str>),
    Domain,
    Argument,
    Scope,
    Box,
    Value,
    ValueS(Cow<'static, str>),
    Internal(Cow<'static, str>),
    // TODO: whatever else I need
}

macro_rules! err {
    ($type:ident, $msg:expr) => {
        Err(crate::utils::error::Error::$type($msg.into()))
    };

    ($type:ident, $msg:expr; !) => {
        crate::utils::error::print_error(crate::utils::error::Error::$type($msg.into()))
    };

    ($type:ident) => {
        Err(crate::utils::error::Error::$type)
    };

    ($type:ident; !) => {
        crate::utils::error::print_error(crate::utils::error::Error::$type)
    };
}

pub(crate) use err;

pub type Status<T = ()> = Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        match self {
            Complete => write!(f, "Completion"),
            MemoryFault(s) => write!(f, "MEMORY [-1]: {s}"),
            NaN => write!(f, "NUMERIC [0]"),
            Syntax(s) => write!(f, "SYNTAX [1]: {s}"),
            Domain => write!(f, "DOMAIN [2]"),
            Box => write!(f, "BOX [3]"),
            Value => write!(f, "VALUE [4]"),
            ValueS(s) => write!(f, "VALUE[4]: {s}"),
            Argument => write!(f, "ARG [5]"),
            Scope => write!(f, "SCOPE [6]"),
            Internal(s) => write!(f, "INTERNAL [-1]: {s}"),
        }
    }
}

pub fn print_error(error: Error) -> ! {
    println!("{error}");
    std::process::exit(1);
}

impl error::Error for Error {}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Self {
        Self::NaN
    }
}

impl From<ParseFloatError> for Error {
    fn from(_: ParseFloatError) -> Self {
        Self::NaN
    }
}

impl<T> From<Error> for Status<T>
where
    T: Clone,
{
    fn from(value: Error) -> Self {
        Err(value)
    }
}
