use std::error;
use std::fmt::{self, Display};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Clone, Debug)]
pub enum Error {
    LexComplete,
    NaN,
    MemoryFault(&'static str),
    Syntax(&'static str),
    Domain,
    Box,
    Value,
    // TODO: whatever else I need
}

macro_rules! err {
    ($type:ident, $msg:expr) => {
        Err(crate::utils::error::Error::$type($msg))
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
            LexComplete => write!(f, "LexCompletion"),
            MemoryFault(s) => write!(f, "MEMORY [-1]: {s}"),
            NaN => write!(f, "NUMERIC [0]"),
            Syntax(s) => write!(f, "SYNTAX [1]: {s}"),
            Domain => write!(f, "DOMAIN [2]"),
            Box => write!(f, "BOX [3]"),
            Value => write!(f, "VALUE [4]"),
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
