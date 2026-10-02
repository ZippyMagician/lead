use std::error;
use std::fmt::{self, Display};
use std::num::{ParseFloatError, ParseIntError};

#[derive(Clone, Debug)]
pub enum Error {
    Fail,
    LexComplete,
    NaN,
    MemoryFault(&'static str),
    ContextFail(&'static str),
    Syntax(&'static str),
    UnsupportedType(&'static str),
    UndefinedSymbol(&'static str),
    SymbolRedefinition(&'static str),
    Unimplemented(&'static str),
    Runtime(&'static str),
    Internal(String),
    Warning(String),
    // TODO: whatever else I need
}

macro_rules! err {
    ($type:ident, $msg:expr) => {
        Err(crate::utils::error::Error::$type($msg))
    };

    ($type:ident) => {
        Err(crate::utils::error::Error::$type)
    };
}

pub(crate) use err;

pub type Status<T = ()> = Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;

        match self {
            LexComplete => write!(f, "LexCompletion"),
            Fail => write!(f, "Error: Program failed."),
            ContextFail(s) => write!(f, "Error: {s}"),
            MemoryFault(s) => write!(f, "MemoryFault: {s}"),
            NaN => write!(f, "TypeError: Not a valid number."),
            Syntax(s) => write!(f, "SyntaxError: {s}"),
            UnsupportedType(s) => write!(f, "TypeError: {s}"),
            UndefinedSymbol(s) => write!(f, "UndefinedError: '{s}' is undefined."),
            SymbolRedefinition(s) => write!(f, "StackError: Attempted to redefine symbol '{s}'."),
            Unimplemented(s) => write!(f, "ImplementationError: '{s}' is unimplemented."),
            Runtime(s) => write!(f, "RuntimeError: {s}"),
            Internal(s) => write!(f, "InternalError: {s}"),
            Warning(s) => write!(f, "Warning: {s}"),
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
