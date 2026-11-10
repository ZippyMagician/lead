extern crate libloading;

use std::path::Path;
use std::sync::RwLock;

use super::{ExternTypes, ScalarType, Window};
use crate::types::list::List;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};

use libloading::Library;

type InnerLink = unsafe extern "C" fn(...) -> *mut std::ffi::c_void;
#[cfg(unix)]
pub type LinkedFunction = libloading::os::unix::Symbol<InnerLink>;
#[cfg(windows)]
pub type LinkedFunction = libloading::os::windows::Symbol<InnerLink>;

pub static CTX_LIBS: RwLock<Vec<Library>> = RwLock::new(Vec::new());

pub(crate) fn load_library(lib: &Path) -> Status {
    let mut lock = CTX_LIBS
        .write()
        .ok()
        .ok_or(Error::Internal("Failed to acquire shared libraries".into()))?;
    unsafe {
        lock.push(Library::new(lib.to_str().unwrap().trim())?);
    }
    drop(lock);
    Ok(())
}

pub(crate) fn link_symbol(name: &str, _vars: Option<Vec<ExternTypes>>) -> Status<LinkedFunction> {
    let lock =
        CTX_LIBS.read().ok().ok_or(Error::Internal("Failed to acquire shared libraries".into()))?;
    let mut res = Err(Error::Scope);
    'inner: for lib in lock.iter() {
        unsafe {
            if let Ok(sym) = lib.get::<InnerLink>(name.as_bytes()) {
                res = Ok(sym.into_raw());
                break 'inner;
            }
        }
    }

    drop(lock);
    res
}

pub(crate) fn call_symbol(
    symbol: LinkedFunction, arena: &Arena, a: List, b: Option<List>,
) -> Status<List> {
    let mut window2 = Window {
        t: ScalarType::Number,
        i: std::ptr::null_mut(),
        c: 0,
        d: 0,
        r: std::ptr::null_mut(),
    };
    if let Some(b) = &b {
        window2 = Window::create(b)?;
    }
    let mut window1 = Window::create(&a)?;

    unsafe {
        let return_value;
        let res =
            symbol(&mut window1, if b.is_some() { &mut window2 } else { std::ptr::null_mut() })
                as *mut Window;
        if !res.is_null() {
            let window = &mut *res; // SAFETY: Checked for null. Other assumptions up to external
            // function.
            return_value = window.list(arena);
            window1.free();
            window2.free();
            drop(a);
            drop(b);
            return return_value;
        } else {
            err!(MemoryFault, "Got null pointer from C FFI")
        }
    }
}

impl From<libloading::Error> for Error {
    fn from(f: libloading::Error) -> Self {
        Self::Internal(format!("{f}").into())
    }
}
