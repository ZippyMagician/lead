// NOTE: Lists should reject incompatible concatenations/contents
// i.e., all internal scalars should be the same matching types.
// That is, numeric / character / boxed
// Maybe booleans as well?

use std::alloc::{Layout, alloc};
use std::fmt::{self, Display};
use std::ptr::slice_from_raw_parts_mut;

use smallvec::{SmallVec, smallvec};

use crate::scalar::Scalar;
use crate::utils::error::{Status, err};
use crate::utils::{compatible, display_boxed, display_boxed_lines};

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

impl Display for Scalar {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Float(n) => write!(f, "{n}"),
            Self::Boolean(b) => write!(f, "{}", *b as u8),
            Self::Char(c) => write!(f, "{c}"),
            Self::Box(l) => {
                if l.is_null() {
                    err!(MemoryFault, "Lost Box"; !);
                }
                // SAFETY: Pointer checked. Taking reference, so other pointers are still valid
                let list: &List = unsafe { &**l };
                write!(f, "{}", display_boxed(format!("{list}"), None, None)?)
            }
        }
    }
}

// SAFETY: Determined by called
fn push_and_get<'a>(v: &mut Vec<String>, s: String) -> &'a str {
    v.push(s);
    let r: &str = v.last().unwrap();
    unsafe { &*(r as *const str) }
}

#[inline(always)]
fn vmask(y: usize, my: usize) -> u8 {
    (y != 0) as u8 | ((y + 1 != my) as u8) << 2
}

#[inline(always)]
fn hmask(x: usize, mx: usize) -> u8 {
    ((x + 1 != mx) as u8) << 1 | ((x != 0) as u8) << 3
}

// TODO: impl Display for List
impl Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_scalar() {
            write!(f, "{}", self.list()[0])
        } else if self.is_empty() {
            Ok(())
        } else if self.dims() < 3 {
            let boxed = self.list()[0].is_box();
            let mut formatted = Vec::with_capacity(self.len());
            let mut strings: Vec<Vec<&str>> = Vec::with_capacity(self.len());

            let mut x = 0;
            let mut y = 1;
            let is_str = self.list()[0].is_char();
            for num in self.list().iter() {
                // SAFETY: formatted is not dropped until strings
                let fm;
                if boxed {
                    fm = push_and_get(
                        &mut formatted,
                        format!("{}", unsafe { num.as_unboxed_unchecked() }),
                    );
                    x = usize::max(x, fm.find('\n').unwrap());
                } else {
                    fm = push_and_get(&mut formatted, format!("{num}"));
                    x = usize::max(x, fm.len());
                }
                strings.push(fm.trim_end().split('\n').collect());
            }
            for ys in strings.iter() {
                y = usize::max(y, ys.len());
            }

            let cols = self.rank()[0];
            unsafe {
                std::hint::assert_unchecked(self.len() % cols == 0);
            }

            if boxed {
                // Collect to list of words, each list of lines
                // Converto to list of constructed lines
                // Write to formatter
                let mut v = 0;
                let mut m = 0;
                let rows = self.len() / cols;
                let strings = (0..)
                    .zip(strings.into_iter())
                    .map(|(i, elements)| {
                        if i % rows == 0 {
                            v = vmask(i / rows, rows);
                        }
                        m = v | hmask(i % rows, cols);
                        Ok((m, display_boxed_lines(elements, Some(x), Some(y), m)?))
                    })
                    .collect::<Result<Vec<(u8, Vec<String>)>, fmt::Error>>()?;
                for row in 0..(self.len() / cols) {
                    let mut line = String::with_capacity(((x + 4) * cols + 1) * (y + 2));
                    for j in usize::from(row != 0)..y + 2 {
                        for i in 0..cols {
                            let string = &strings[row * cols + i];
                            line.push_str(&string.1[j][usize::from(i != 0) * 3..]);
                        }
                        line.push('\n');
                    }
                    write!(f, "{line}")?;
                }
            } else {
                // Currently we have a list of scalar values
                // x and y are the max size of those scalars.
                for row in 0..(self.len() / cols) {
                    for col in 0..cols {
                        let word = &strings[row * cols + col];
                        assert!(word.len() == 1, "Internal error");
                        let word = word[0];
                        if is_str || self.dims() == 1 {
                            write!(f, "{}", word)?;
                        } else {
                            write!(f, "{}{}", " ".repeat(x - word.len()), word)?;
                        }
                        if !is_str && col < cols - 1 {
                            write!(f, " ")?;
                        }
                    }
                    write!(f, "\n")?;
                }
            }
            Ok(())
        } else {
            Ok(())
        }
    }
}
