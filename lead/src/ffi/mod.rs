pub mod internal;

use crate::list::List;
use crate::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};

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

struct ReprType {
    tag: u8,
    t: ReprValue,
}

struct ReprValue {
    tag: u8,
    v: List,
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
                if (*r).tag != 0 {
                    return err!(Domain);
                }
                let v = &mut (*r).t as *mut ReprValue;
                if (*v).tag != 0 {
                    return err!(Domain);
                }
                let l = &mut (*v).v;

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
                Ok(Self::Box(Box::leak(Box::new(window))))
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
                let list = List::new(
                    v.into(),
                    w.c,
                    w.d,
                    std::slice::from_raw_parts(w.r, w.d).into(),
                );
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
