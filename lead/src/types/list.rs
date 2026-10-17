// NOTE: Lists should reject incompatible concatenations/contents
// i.e., all internal scalars should be the same matching types.
// That is, numeric / character / boxed
// Maybe booleans as well?

use std::alloc::{Layout, alloc};
use std::fmt::{self, Display};
use std::ptr::slice_from_raw_parts_mut;

use smallvec::{SmallVec, smallvec};

use super::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Status, err};
use crate::utils::{compatible, display_boxed_lines};

const INLINE_SIZE: usize = 4;

#[derive(Clone, Debug, Hash)]
pub struct List {
    dt: Box<[Scalar]>,
    ct: usize,
    pub(crate) ds: usize,
    pub(crate) rk: SmallVec<[usize; INLINE_SIZE]>,
}

impl List {
    #[inline(always)]
    pub fn new(
        dt: Box<[Scalar]>,
        ct: usize,
        ds: usize,
        rk: SmallVec<[usize; INLINE_SIZE]>,
    ) -> Self {
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
    pub fn mat(data: Vec<Scalar>, ds: usize, rk: SmallVec<[usize; INLINE_SIZE]>) -> Status<Self> {
        if !compatible(&data) {
            return err!(Domain);
        }
        let size = data.len();
        Ok(Self {
            dt: data.into(),
            ct: size,
            ds,
            rk,
        })
    }

    #[inline]
    pub fn mat_ref(data: &[Scalar], ds: usize, rk: &[usize]) -> Status<Self> {
        if !compatible(data) {
            return err!(Domain);
        }
        let size = data.len();
        Ok(Self {
            dt: data.into(),
            ct: size,
            ds,
            rk: rk.into(),
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

    #[inline]
    pub fn range(start: Scalar, end: Scalar) -> Status<Self> {
        if start.is_char() && end.is_char() {
            let _start = unsafe { start.to_char_unchecked() };
            let _end = unsafe { end.to_char_unchecked() };

            return err!(Internal, "TODO: Alphabetical range @ <list.rs> range");
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
    pub fn ranked(self, dims: usize) -> Status<Vec<Self>> {
        if dims == 0 {
            Ok(self.list().iter().copied().map(List::scalar).collect())
        } else if dims >= self.dims() {
            Ok(vec![self])
        } else {
            let rank: SmallVec<[usize; INLINE_SIZE]> =
                self.rank().iter().copied().take(dims).collect();
            let ct: usize = rank.iter().product();

            unsafe { std::hint::assert_unchecked(self.ct % ct == 0) }

            let mut b = Vec::with_capacity(self.ct / ct);
            for i in 0..self.ct / ct {
                let l = self.list()[i * ct..i * ct + ct].to_vec();
                b.push(List::mat(l, dims, rank.clone())?);
            }

            Ok(b)
        }
    }

    #[inline]
    pub fn unranked(
        arena: &Arena,
        original: usize,
        trailing_rank: &[usize],
        mut lists: Vec<List>,
    ) -> Status<Self> {
        if lists.is_empty() {
            return Ok(List::empty());
        } else if lists.len() == 1 {
            return Ok(std::mem::take(&mut lists[0]));
        }

        let mut box_it = false;
        let rank = lists[0].rank();
        for list in lists.iter() {
            if !eq_rank(rank, list.rank()) {
                box_it = true;
                break;
            }
        }

        if box_it {
            List::mat(
                lists
                    .into_iter()
                    .map(|n| Scalar::boxed(&arena, n))
                    .collect::<Status<Vec<Scalar>>>()?,
                trailing_rank.len(),
                trailing_rank.into(),
            )
        } else {
            let mut build: Vec<Scalar> =
                Vec::with_capacity(rank.iter().copied().product::<usize>() * lists.len());
            for list in lists.iter() {
                build.extend(list.list());
            }

            let dims = usize::max(rank.len() + trailing_rank.len(), original);
            let mut rank: SmallVec<[usize; 4]> = if eq_rank(rank, &[1]) {
                smallvec![]
            } else {
                rank.into()
            };
            rank.extend(trailing_rank.iter().copied());

            let diff = build.len() / rank.iter().product::<usize>();
            if diff != 1 {
                rank.push(diff);
            }
            List::mat(build, dims, rank)
        }
    }
}

#[inline]
fn eq_rank(a: &[usize], b: &[usize]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(l, r)| l == r)
}

impl PartialOrd for List {
    fn partial_cmp(&self, o: &List) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&o))
    }
}

impl PartialEq for List {
    fn eq(&self, o: &List) -> bool {
        eq_rank(self.rank(), o.rank()) && self.list().iter().zip(o.list()).all(|(l, r)| *l == *r)
    }
}

impl Ord for List {
    fn cmp(&self, o: &List) -> std::cmp::Ordering {
        self.dims()
            .cmp(&o.dims())
            .then(self.len().cmp(&o.len()))
            .then_with(|| {
                let l = self.list();
                let o = o.list();

                for (l, r) in l.iter().zip(o) {
                    if l != r {
                        return l.cmp(&r);
                    }
                }
                std::cmp::Ordering::Equal
            })
    }
}

impl Eq for List {}

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
            write!(f, "{}", self.list()[0])?;
        } else if self.is_empty() {
            std::hint::cold_path();
            return Ok(());
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
                    // Oh rust strs, how you fuck with me
                    x = usize::max(
                        x,
                        fm.chars()
                            .zip(0..)
                            .find(|(c, _)| *c == '\n')
                            .map_or_else(|| fm.chars().count(), |(_, i)| i),
                    );
                } else {
                    fm = push_and_get(&mut formatted, format!("{num}"));
                    x = usize::max(x, fm.chars().count());
                }
                strings.push(fm.trim_end_matches('\n').split('\n').collect());
            }
            for ys in strings.iter() {
                y = usize::max(y, ys.len());
            }

            let cols = self.rank()[0];
            unsafe {
                std::hint::assert_unchecked(self.len() % cols == 0);
            }

            let rows = self.len() / cols;
            if boxed {
                // Collect to list of words, each list of lines
                // Converto to list of constructed lines
                // Write to formatter
                let mut v = 0;
                let mut m = 0;
                let strings = (0..)
                    .zip(strings.into_iter())
                    .map(|(i, elements)| {
                        if i % cols == 0 {
                            v = vmask(i / cols, rows);
                        }
                        m = v | hmask(i % cols, cols);
                        Ok((m, display_boxed_lines(elements, Some(x), Some(y), m)?))
                    })
                    .collect::<Result<Vec<(u8, Vec<String>)>, fmt::Error>>()?;
                for row in 0..rows {
                    let mut line = String::with_capacity(((x + 4) * cols + 1) * (y + 2));
                    for j in usize::from(row != 0)..y + 2 {
                        for i in 0..cols {
                            let string = &strings[row * cols + i];
                            line.push_str(&string.1[j][usize::from(i != 0) * 3..]);
                        }
                        if row + 1 != rows || j != y + 1 {
                            line.push('\n');
                        }
                    }
                    write!(f, "{line}")?;
                }
            } else {
                // Currently we have a list of scalar values
                // x and y are the max size of those scalars.
                for row in 0..rows {
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
                    if row + 1 != rows {
                        write!(f, "\n")?;
                    }
                }
            }
        } else {
            let dims = self.dims();
            let ranked = self.clone().ranked(dims - 1).expect("Couldn't disp. list");
            for i in 0..ranked.len() {
                write!(f, "{}", ranked[i])?;
                if i + 1 != ranked.len() {
                    write!(f, "{}", "\n".repeat(dims - 1))?;
                }
            }
        }

        Ok(())
    }
}

impl Default for List {
    #[inline(always)]
    fn default() -> Self {
        Self::empty()
    }
}
