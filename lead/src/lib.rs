pub mod exec;
pub mod lex;
pub mod parse;
pub mod scope;

pub mod array;
pub mod utils;

pub mod builtins;

pub mod function;
pub mod types;

#[cfg(feature = "ffi")]
pub mod ffi;
