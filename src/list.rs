// NOTE: Lists should reject incompatible concatenations/contents
// i.e., all internal scalars should be the same matching types.
// That is, numeric / character / boxed
// Maybe booleans as well?

use std::alloc::{Layout, alloc};
use std::ptr::slice_from_raw_parts_mut;

use smallvec::{SmallVec, smallvec};

use crate::scalar::Scalar;
use crate::utils::compatible;
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
    pub fn from_vec(data: Vec<Scalar>) -> Status<Self> {
        if !compatible(&data) {
            return err!(Domain);
        }
        let size = data.len();
        Ok(Self {
            dt: data.into(),
            ct: size,
            ds: 1,
            rk: smallvec![size],
        })
    }

    #[inline]
    pub fn scalar(data: Scalar) -> Self {
        Self {
            dt: [data].into(),
            ct: 1,
            ds: 1,
            rk: smallvec![1],
        }
    }

    #[inline]
    pub fn string(data: &str) -> Self {
        let size = data.len();
        Self {
            dt: data.chars().map(Scalar::Char).collect(),
            ct: size,
            ds: 1,
            rk: smallvec![size],
        }
    }

    #[inline]
    pub fn range(start: Scalar, end: Scalar) -> Status<Self> {
        if start.is_char() && end.is_char() {
            let _start = unsafe { start.to_char_unchecked() };
            let _end = unsafe { end.to_char_unchecked() };

            todo!("Alphabetical range @ <list.rs> rnage");
        } else {
            let start = start.to_num()?;
            let end = end.to_num()?;
            if end < start || end < 0 || start < 0 {
                return err!(Domain);
            }

            let count: usize = (end - start + 1) as usize;
            let layout = Layout::array::<Scalar>(count).unwrap();
            // SAFETY: Pointer is checked for NULL. Size of allocation is known,
            // slice is transmuted and boxed immediately
            unsafe {
                let ptr = alloc(layout) as *mut Scalar;
                if ptr.is_null() {
                    return err!(MemoryFault, "Could not allocate space.");
                }

                for (j, i) in (start..).zip(0..count) {
                    ptr.add(i).write(Scalar::Number(j));
                }

                let raw: *mut [Scalar] = slice_from_raw_parts_mut(ptr, count);
                Ok(Self::new(Box::from_raw(raw), count, 1, smallvec![count]))
            }
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
        let count: usize = r.iter().product();
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

                let raw: *mut [Scalar] = slice_from_raw_parts_mut(ptr, count);
                Ok(Self::new(Box::from_raw(raw), count, r.len(), r.into()))
            }
        }
    }

    #[inline]
    pub fn ranked(self, dims: usize) -> Vec<Self> {
        if dims == 0 {
            self.list().iter().cloned().map(List::scalar).collect()
        } else if dims >= self.dims() {
            vec![self]
        } else {
            todo!("Non-trivial List::ranked(self, dims)");
        }
    }

    #[inline]
    pub fn unranked<T>(_original: usize, _trailing_rank: &[usize], _list: T) -> Status<Self>
    where
        T: Iterator<Item = Status<Self>>,
    {
        todo!("List::unranked");
    }
}
