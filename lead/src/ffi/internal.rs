extern crate libloading;

use super::{ExternTypes, ScalarExtern, Window};
use crate::types::list::List;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};

use std::path::Path;
use std::sync::RwLock;

use libloading::Library;

type InnerLink = unsafe extern "C" fn(...) -> *mut Window;
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
    let mut l1 = Vec::with_capacity(a.len());
    for s in a.list() {
        l1.push(ScalarExtern::from_scalar(*s)?);
    }

    let mut window2 = Window { i: std::ptr::null_mut(), c: 0, d: 0, r: std::ptr::null_mut() };
    if let Some(b) = &b {
        let mut l2 = Vec::with_capacity(b.len());
        for s in b.list() {
            l2.push(ScalarExtern::from_scalar(*s)?);
        }
        window2 = Window {
            i: l2.into_raw_parts().0,
            c: b.len(),
            d: b.dims(),
            r: b.rank().as_ptr() as *mut usize,
        };
    }

    let mut window1 = Window {
        i: l1.into_raw_parts().0,
        c: a.len(),
        d: a.dims(),
        r: a.rank().as_ptr() as *mut usize,
    };

    unsafe {
        let res =
            symbol(&mut window1, if b.is_some() { &mut window2 } else { std::ptr::null_mut() });
        if !res.is_null() {
            let window = &mut *res; // SAFETY: Checked for null. Other assumptions up to external
            // function.
            let mut v = Vec::with_capacity(window.c);
            for i in 0..window.c {
                v.push((*window.i.add(i)).to_scalar(arena)?);
            }
            let list = List::new(
                v.into(),
                window.c,
                window.d,
                std::slice::from_raw_parts(window.r, window.d).into(),
            );
            Ok(list)
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

impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        Self::Internal(format!("{f}").into())
    }
}
