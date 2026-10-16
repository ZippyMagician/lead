extern crate libloading;

use super::Window;
use crate::utils::error::{Error, Status};

use std::path::Path;
use std::sync::RwLock;

use libloading::{Library, Symbol};

pub static CTX_LIBS: RwLock<Vec<Library>> = RwLock::new(Vec::new());
pub static DYN_SYMBOLS: RwLock<
    Vec<Symbol<unsafe extern "C" fn(*mut Window, *mut Window) -> *mut Window>>,
> = RwLock::new(Vec::new());

pub fn load_library(lib: &Path) -> Status {
    let mut lock = CTX_LIBS
        .write()
        .ok()
        .ok_or(Error::Internal("Failed to acquire shared libraries".into()))?;
    unsafe {
        // why tf is there a trailing newline rust?
        lock.push(Library::new(lib.to_str().unwrap().trim())?);
    }
    drop(lock);
    Ok(())
}

impl From<libloading::Error> for Error {
    fn from(f: libloading::Error) -> Self {
        Self::Internal(format!("{f}").into())
    }
}

impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        Self::Internal(format!("{f}").into())
    }
}
