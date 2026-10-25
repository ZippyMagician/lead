// NOTE: Lists should reject incompatible concatenations/contents
// i.e., all internal scalars should be the same matching types.
// That is, numeric / character / boxed
// Maybe booleans as well?

use std::fmt::{self, Display};

use ecow::{EcoVec, eco_vec};
use smallvec::{SmallVec, smallvec};

use super::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Status, err};
use crate::utils::{compatible, display_boxed_lines};

const INLINE_SIZE: usize = 4;

#[derive(Clone, Debug, Hash)]
pub struct List {
    // dt: Box<[Scalar]>,
    dt: Scalars,
    pub(crate) ds: usize,
    pub(crate) rk: SmallVec<[usize; INLINE_SIZE]>,
}

impl List {
    #[inline(always)]
    pub fn new(dt: impl Into<Scalars>, ds: usize, rk: SmallVec<[usize; INLINE_SIZE]>) -> Self {
        Self { dt: dt.into(), ds, rk }
    }

    #[inline]
    pub fn empty() -> Self {
        Self { dt: Default::default(), ds: 0, rk: smallvec![] }
    }

    #[inline]
    pub fn from_vec(data: EcoVec<Scalar>) -> Status<Self> {
        if !compatible(&data) {
            return err!(Domain);
        }
        let size = data.len();
        Ok(Self { dt: data.into(), ds: 1, rk: smallvec![size] })
    }

    #[inline]
    pub fn mat(
        data: EcoVec<Scalar>, ds: usize, rk: SmallVec<[usize; INLINE_SIZE]>,
    ) -> Status<Self> {
        if !compatible(&data) {
            return err!(Domain);
        }
        Ok(Self { dt: data.into(), ds, rk })
    }

    #[inline]
    pub fn mat_ref(data: &[Scalar], ds: usize, rk: &[usize]) -> Status<Self> {
        if !compatible(data) {
            return err!(Domain);
        }
        Ok(Self { dt: data.into(), ds, rk: rk.into() })
    }

    #[inline]
    pub fn scalar(data: Scalar) -> Self {
        Self { dt: data.into(), ds: 1, rk: smallvec![1] }
    }

    #[inline]
    pub fn string(data: &str) -> Self {
        let size = data.chars().count();
        Self {
            dt: data.chars().map(Scalar::Char).collect::<EcoVec<Scalar>>().into(),
            ds: 1,
            rk: smallvec![size],
        }
    }

    #[inline(always)]
    pub fn list(&self) -> &[Scalar] {
        self.dt.list()
    }

    #[inline(always)]
    pub fn take_list(self) -> EcoVec<Scalar> {
        self.dt.take_list()
    }

    #[inline(always)]
    pub fn list_mut(&mut self) -> &mut [Scalar] {
        self.dt.list_mut()
    }

    #[inline(always)]
    pub fn dims(&self) -> usize {
        unsafe { std::hint::assert_unchecked(self.rk.len() == self.ds) };
        self.ds
    }

    #[inline(always)]
    pub fn rank(&self) -> &[usize] {
        &self.rk
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.dt.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn is_scalar(&self) -> bool {
        self.len() == 1
    }

    #[inline]
    pub fn range(start: Scalar, end: Scalar) -> Status<Self> {
        let chars = start.is_char() && end.is_char();
        let start = start.to_num()?;
        let end = end.to_num()?;
        if end < start || end < 0 || start < 0 {
            return err!(Domain);
        }
        let count: usize = (end - start + 1) as usize;
        unsafe {
            if chars {
                Self::alloc_iter(&[count], (start..=end).map(|i| Scalar::Char(i as u8 as char)))
            } else {
                Self::alloc_iter(&[count], (start..=end).map(Scalar::Number))
            }
        }
    }

    // Swaps out rank. It is in place if possible
    #[inline]
    pub fn with_rank(mut self, r: &[usize]) -> Status<Self> {
        let count: usize = r.iter().product();
        if count == self.len() {
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
            let bucket = self.list().into_iter().copied().cycle();
            unsafe { Self::alloc_iter(r, bucket) }
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn sliced(&self, start: usize, end: usize, r: &[usize]) -> Self {
        Self { dt: self.dt.sliced(start, end), ds: r.len(), rk: SmallVec::from_slice(r) }
    }

    // SAFETY: Assumes called has provided valid sizes
    #[inline(always)]
    pub(crate) unsafe fn alloc_iter<T>(r: &[usize], bucket: T) -> Status<List>
    where
        T: Iterator<Item = Scalar>,
    {
        let count: usize = r.iter().product();
        let dims = r.len();
        if dims == 0 {
            return Ok(List::empty());
        }
        let rank = SmallVec::from_slice(r);

        let bucket = SizedScalars::new(bucket, count);
        Ok(unsafe { Self::new(Scalars::from_iter(bucket, count), dims, rank) })
    }

    // SAFETY: Assumes all sizes within the struct are valid in respect to the data
    #[inline]
    pub fn ranked(&self, dims: usize) -> Status<Vec<List>> {
        if dims == 0 {
            let mut v = Vec::with_capacity(self.len());
            for i in 0..self.len() {
                v.push(unsafe { self.sliced(i, i + 1, &[1]) });
            }
            Ok(v)
        } else if dims >= self.dims() {
            Ok(vec![self.clone()])
        } else {
            let ct: usize = self.rank().iter().product();
            unsafe { std::hint::assert_unchecked(self.len() % ct == 0) }

            let size = self.len() / ct;
            let mut v = Vec::with_capacity(size);
            for i in 0..size {
                v.push(unsafe { self.sliced(i * size, i * size + size, &self.rank()[0..dims]) });
            }
            return Ok(v);
        }
    }

    #[inline]
    pub fn unranked(
        arena: &Arena, _original: usize, trailing_rank: &[usize], mut lists: Vec<List>,
    ) -> Status<Self> {
        if lists.is_empty() {
            return Ok(List::empty());
        } else if lists.len() == 1 {
            return Ok(std::mem::take(&mut lists[0]));
        }

        let mut box_it = false;
        let rank = lists[0].rank();
        for list in lists.iter().skip(1) {
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
                    .collect::<Status<EcoVec<Scalar>>>()?,
                trailing_rank.len(),
                trailing_rank.into(),
            )
        } else {
            let mut build: EcoVec<Scalar> =
                EcoVec::with_capacity(rank.iter().product::<usize>() * lists.len());
            for list in lists.iter() {
                build.extend(list.list().iter().copied());
            }

            let mut rank: SmallVec<[usize; 4]> =
                if eq_rank(rank, &[1]) { smallvec![] } else { rank.into() };
            rank.extend(trailing_rank.iter().copied());

            let diff = build.len() / rank.iter().product::<usize>();
            if diff != 1 {
                rank.push(diff);
            }
            List::mat(build, rank.len(), rank)
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
        self.dims().cmp(&o.dims()).then(self.len().cmp(&o.len())).then_with(|| {
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
                            let l = &string.1[j];
                            let off = l.chars().next().unwrap().len_utf8();
                            line.push_str(&l[usize::from(i != 0) * off..]);
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
            let ranked = self.ranked(dims - 1).expect("Couldn't disp. list");
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

pub struct SizedScalars<T: Iterator<Item = Scalar>> {
    iter: T,
    size: usize,
    c: usize,
}

impl<T: Iterator<Item = Scalar>> SizedScalars<T> {
    pub fn new(iter: T, size: usize) -> Self {
        Self { iter, size, c: 0 }
    }
}

impl<T: Iterator<Item = Scalar>> Iterator for SizedScalars<T> {
    type Item = Scalar;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.c += 1;
        self.iter.next()
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe { std::hint::assert_unchecked(self.c <= self.size) }
        (self.size - self.c, Some(self.size - self.c))
    }
}

#[derive(Clone, Debug, Default, Hash)]
pub struct Scalars {
    v: EcoVec<Scalar>,
    s: usize,
    e: usize,
}

impl Scalars {
    #[inline(always)]
    pub(crate) fn list(&self) -> &[Scalar] {
        &self.v
    }

    #[inline(always)]
    pub(crate) fn take_list(self) -> EcoVec<Scalar> {
        self.v
    }

    #[inline(always)]
    pub(crate) fn list_mut(&mut self) -> &mut [Scalar] {
        self.v.make_mut()
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        unsafe { std::hint::assert_unchecked(self.e >= self.s) }
        self.e - self.s
    }

    #[inline(always)]
    pub(crate) fn sliced(&self, start: usize, end: usize) -> Self {
        Self { v: self.v.clone(), s: start, e: end }
    }

    // SAFETY: What an incredible function
    #[inline(always)]
    pub(crate) unsafe fn from_iter<T>(it: SizedScalars<T>, end: usize) -> Self
    where
        T: Iterator<Item = Scalar>,
    {
        let mut v = EcoVec::with_capacity(it.size_hint().0);
        // SAFETY: Ptr is checked for null
        // as for the casting, well lol
        unsafe {
            let ptr = v.as_ptr() as *mut Scalar;
            for (i, s) in it.enumerate() {
                ptr.add(i).write(s);
            }
            // SAFETY: lmao
            let mut temp: EcoVecInternal<Scalar> = std::mem::transmute(v);
            temp.len = end;
            v = std::mem::transmute(temp);
        }
        Self { v, s: 0, e: end }
    }
}

#[repr(C)]
struct EcoVecInternal<T> {
    ptr: std::ptr::NonNull<T>,
    pub(crate) len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl From<EcoVec<Scalar>> for Scalars {
    #[inline(always)]
    fn from(e: EcoVec<Scalar>) -> Self {
        let size = e.len();
        Self { v: e, s: 0, e: size }
    }
}

impl From<Scalar> for Scalars {
    #[inline(always)]
    fn from(s: Scalar) -> Self {
        Self { v: eco_vec![s], s: 0, e: 1 }
    }
}

impl From<&[Scalar]> for Scalars {
    #[inline(always)]
    fn from(s: &[Scalar]) -> Self {
        let size = s.len();
        Self { v: s.into(), s: 0, e: size }
    }
}
