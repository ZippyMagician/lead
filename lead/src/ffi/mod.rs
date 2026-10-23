pub mod internal;

use std::mem::ManuallyDrop;

use crate::function::Function;
use crate::types::list::List;
use crate::types::map::Map;
use crate::types::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[repr(u8)]
pub(crate) enum ExternTypes {
    C_INT8,
    C_INT16,
    C_INT32,
    C_INT64,
    C_UINT8,
    C_UINT16,
    C_UINT32,
    C_UINT64,
    C_FLOAT,
    C_DOUBLE,
    C_BOOL,
    C_CHAR,
    C_STR,
}

#[repr(C)]
pub struct Window {
    // list
    pub i: *mut ScalarExtern,
    // count
    pub c: usize,
    // dimensions
    pub d: usize,
    // rank
    pub r: *mut usize,
}

#[derive(Clone, Copy)]
#[repr(C, u8)]
pub enum ScalarExtern {
    Number(i64),
    Float(f64),
    Boolean(bool),
    Char(u32),
    Box(*mut Window),
}

#[repr(C)]
union TypeInner {
    f: ManuallyDrop<Function>,
    l: ManuallyDrop<List>,
    s: Scalar,
    m: ManuallyDrop<Map>,
}

#[repr(C)]
struct ReprType {
    tag: u8,
    t: TypeInner,
}

impl ScalarExtern {
    pub fn from_scalar(s: Scalar) -> Status<Self> {
        match s {
            Scalar::Number(n) => Ok(Self::Number(n)),
            Scalar::Float(f) => Ok(Self::Float(f.into_inner())),
            Scalar::Boolean(b) => Ok(Self::Boolean(b)),
            Scalar::Char(c) => Ok(Self::Char(c as u32)),
            // SAFETY: Good luck
            Scalar::Box(t) => unsafe {
                let r = t as *mut ReprType;
                if (*r).tag != 1 && (*r).tag != 2 {
                    return err!(Domain);
                }

                let l = if (*r).tag == 1 { &mut (*r).t.l } else { &mut List::scalar((*r).t.s) };

                let mut alloc = l
                    .list()
                    .iter()
                    .map(|s| Self::from_scalar(*s))
                    .collect::<Status<Vec<Self>>>()?;
                alloc.shrink_to_fit();
                let window = Window {
                    i: alloc.into_raw_parts().0,
                    d: l.dims(),
                    c: l.len(),
                    r: l.rank().as_ptr() as *mut usize,
                };
                Ok(Self::Box(Box::into_raw(Box::new(window))))
            },
        }
    }

    pub fn to_scalar(self, arena: &Arena) -> Status<Scalar> {
        match self {
            Self::Number(n) => Ok(Scalar::Number(n)),
            Self::Float(f) => Ok(Scalar::Float(f.into())),
            Self::Boolean(b) => Ok(Scalar::Boolean(b)),
            Self::Char(c) => Ok(Scalar::Char(char::from_u32(c).ok_or(Error::NaN)?)),
            // SAFETY: Assumes extern call did not fuck with the values
            // (i.e. they are all correct)
            Self::Box(w) => unsafe {
                let w: &mut Window = &mut *w;
                let mut v = Vec::with_capacity(w.c);
                for i in 0..w.c {
                    v.push((*w.i.add(i)).to_scalar(arena)?);
                }
                let list =
                    List::new(v.into(), w.c, w.d, std::slice::from_raw_parts(w.r, w.d).into());
                drop(Box::from_raw(w)); // SAFETY: Assume allocation wasn't recreated by C
                Ok(Scalar::boxed(arena, list)?)
            },
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn lead_free_list(a: *mut Window) {
    unsafe {
        let a = &mut *a;
        let inner = Box::from_raw(std::slice::from_raw_parts_mut(a.i, a.c) as *mut [ScalarExtern]);
        let rank = Vec::from_raw_parts(a.r, a.d, a.d);

        drop(inner);
        drop(rank);
    }
}
