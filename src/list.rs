use std::alloc::{Layout, alloc};
use std::slice;

use smallvec::{SmallVec, smallvec};

use crate::scalar::Scalar;
use crate::utils::error::{Status, err};

const INLINE_SIZE: usize = 4;

#[derive(Clone, Debug)]
pub struct List {
    dt: Box<[Scalar]>,
    ct: usize,
    ds: usize,
    rk: SmallVec<[usize; INLINE_SIZE]>,
}

impl List {
    #[inline(always)]
    fn new(dt: Box<[Scalar]>, ct: usize, ds: usize, rk: SmallVec<[usize; INLINE_SIZE]>) -> Self {
        Self { dt, ct, ds, rk }
    }

    #[inline]
    pub fn empty() -> Self {
        Self {
            dt: Box::new([]),
            ct: 0,
            ds: 0,
            rk: smallvec![],
        }
    }

    #[inline]
    pub fn from_vec(data: Vec<Scalar>) -> Self {
        let size = data.len();
        Self {
            dt: data.into(),
            ct: size,
            ds: 1,
            rk: smallvec![size],
        }
    }

    #[inline]
    pub fn scalar(data: Scalar) -> Self {
        Self {
            dt: [data].into(),
            ct: 1,
            ds: 0,
            rk: smallvec![1],
        }
    }

    #[inline(always)]
    pub fn list(&self) -> &[Scalar] {
        &self.dt
    }

    #[inline(always)]
    pub fn list_mut(&mut self) -> &mut [Scalar] {
        &mut self.dt
    }

    #[inline(always)]
    pub fn dims(&self) -> usize {
        self.ds
    }

    #[inline(always)]
    pub fn rank(&self) -> &[usize] {
        &self.rk
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.ct
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.ct == 0
    }

    #[inline(always)]
    pub fn is_scalar(&self) -> bool {
        self.ct == 1
    }

    // Swaps out rank. It is in place if possible
    #[inline]
    pub fn with_rank(mut self, r: &[usize]) -> Status<Self> {
        let count: usize = r.into_iter().product();
        if count == self.ct {
            if INLINE_SIZE >= self.dims() || INLINE_SIZE < r.len() {
                self.rk = SmallVec::from_slice(r);
            } else {
                self.rk.clear();
                self.rk.extend_from_slice(r);
            }
            self.ds = self.rk.len();
            Ok(self)
        } else if self.is_empty() {
            Ok(self)
        } else {
            let mut bucket = self.list().iter().cycle();
            let layout = Layout::array::<Scalar>(count).unwrap();
            // SAFETY: Pointer is checked for NULL. Size of allocation is known,
            // slice is transmuted and boxed immediately
            unsafe {
                let ptr = alloc(layout) as *mut Scalar;
                if ptr.is_null() {
                    return err!(MemoryFault, "Could not allocate space.");
                }

                for i in 0..count {
                    ptr.add(i).write(*bucket.next().unwrap());
                }

                let raw: *mut [Scalar] = slice::from_raw_parts_mut(ptr, count);
                Ok(Self::new(Box::from_raw(raw), count, r.len(), r.into()))
            }
        }
    }
}
