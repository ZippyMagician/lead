pub mod internal;

use std::alloc::{Layout, alloc, dealloc};
use std::mem::ManuallyDrop;

use crate::function::Function;
use crate::types::Type;
use crate::types::list::{Array, List};
use crate::types::map::Map;
use crate::types::scalar::{self, Boxed, Scalar};
use crate::utils::Arena;
use crate::utils::error::{Status, err};
use crate::utils::list::with_list;

use ordered_float::OrderedFloat;
use smallvec::SmallVec;

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
    // tag
    pub t: ScalarType,
    // list
    pub i: *mut u8,
    // count
    pub c: usize,
    // dimensions
    pub d: usize,
    // rank
    pub r: *mut usize,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ScalarType {
    Number = b'I',
    Float = b'F',
    Boolean = b'B',
    Char = b'C',
    Box = b'X',
}

#[repr(C)]
union TypeInner {
    f: ManuallyDrop<Function>,
    l: ManuallyDrop<List>,
    m: ManuallyDrop<Map>,
}

#[repr(C)]
struct ReprType {
    tag: u8,
    t: TypeInner,
}

impl Window {
    #[inline]
    pub fn create(a: &List) -> Status<Window> {
        let ptr: *mut u8;
        with_list!(a, |a| unsafe {
            if a.get_type() != b'X' {
                let size = std::mem::size_of_val(&a.list()[0]);
                let dst = alloc(Layout::array::<u8>(a.len() * size).unwrap());
                std::ptr::copy_nonoverlapping(a.list().as_ptr() as *const u8, dst, a.len() * size);
                ptr = dst;
            } else {
                const SIZE: usize = std::mem::size_of::<Boxed>();
                let dst = alloc(Layout::array::<u8>(SIZE * a.len()).unwrap());
                for (i, b) in a.list().iter().enumerate() {
                    // unwrap boxed value
                    let t: &Type = b.unbox();
                    let r: &ReprType = std::mem::transmute(t);
                    if r.tag != 1 {
                        return err!(Domain);
                    }
                    let l = &r.t.l;

                    let bx = alloc(Layout::new::<Window>());
                    *(bx as *mut Window) = Window::create(l)?;
                    std::ptr::copy_nonoverlapping(bx as *const u8, dst.add(i * SIZE), SIZE);
                }

                ptr = dst;
            }
        });

        Ok(Window {
            t: unsafe { std::mem::transmute(a.get_type()) },
            i: ptr,
            c: a.len(),
            d: a.dims(),
            r: a.rank().as_ptr() as *mut usize,
        })
    }

    // SAFETY: Assumes FFI did not tamper with it
    #[inline]
    pub unsafe fn free(&self) {
        let size = match self.t {
            ScalarType::Number => std::mem::size_of::<i64>(),
            ScalarType::Float => std::mem::size_of::<f64>(),
            ScalarType::Boolean => std::mem::size_of::<bool>(),
            ScalarType::Char => std::mem::size_of::<char>(),
            ScalarType::Box => std::mem::size_of::<*mut Window>(),
        };
        unsafe {
            if self.t == ScalarType::Box {
                let ptr = self.i as *mut *mut Window;
                for i in 0..self.c {
                    let wptr = ptr.add(i).read();
                    dealloc(wptr as *mut u8, Layout::new::<Window>());
                }
            }
            dealloc(self.i, Layout::array::<u8>(self.c * size).unwrap());
            // Rank comes from pointer to list's rank, doesn't need to be dealloc'd
        }
    }

    #[inline]
    pub fn list(&self, a: &Arena) -> Status<List> {
        match self.t {
            ScalarType::Number => unsafe {
                let ptr = self.i as *mut i64;
                let rank =
                    (0..self.d).map(|i| self.r.add(i).read()).collect::<SmallVec<[usize; 2]>>();
                Array::alloc_iter(&rank, (0..self.c).map(|i| ptr.add(i).read()))
            },
            ScalarType::Float => unsafe {
                let ptr = self.i as *mut OrderedFloat<f64>;
                let rank =
                    (0..self.d).map(|i| self.r.add(i).read()).collect::<SmallVec<[usize; 2]>>();
                Array::alloc_iter(&rank, (0..self.c).map(|i| ptr.add(i).read()))
            },
            ScalarType::Boolean => unsafe {
                let ptr = self.i as *mut bool;
                let rank =
                    (0..self.d).map(|i| self.r.add(i).read()).collect::<SmallVec<[usize; 2]>>();
                Array::alloc_iter(&rank, (0..self.c).map(|i| ptr.add(i).read()))
            },
            ScalarType::Char => unsafe {
                let ptr = self.i as *mut u8;
                let rank =
                    (0..self.d).map(|i| self.r.add(i).read()).collect::<SmallVec<[usize; 2]>>();
                Array::alloc_iter(&rank, (0..self.c).map(|i| ptr.add(i).read()))
            },
            ScalarType::Box => unsafe {
                let ptr = self.i as *mut *mut Window;
                let rank =
                    (0..self.d).map(|i| self.r.add(i).read()).collect::<SmallVec<[usize; 2]>>();
                Array::try_alloc_iter(
                    &rank,
                    (0..self.c).map(|i| {
                        let window = ptr.add(i).read().read();
                        scalar::boxed(a, window.list(a)?)
                    }),
                )
                .map(List::Boxes)
            },
        }
    }
}
