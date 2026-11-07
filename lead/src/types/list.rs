// NOTE: Arrays should reject incompatible concatenations/contents
// i.e., all internal scalars should be the same matching types.
// That is, numeric / character / boxed
// Maybe booleans as well?

use std::fmt::{self, Display};
use std::hash::Hash;
use std::hint;
use std::rc::Rc;

use super::scalar::{self, Boxed, Scalar};
use crate::utils::Arena;
use crate::utils::display_boxed_lines;
use crate::utils::error::{Status, err};
use crate::utils::list::with_list;

use ordered_float::OrderedFloat;
use smallvec::{SmallVec, smallvec};

pub(crate) const INLINE_SIZE: usize = 2;

#[derive(Clone, Debug, Hash)]
#[repr(u8, C)]
pub enum List {
    Integers(Array<i64>),
    Doubles(Array<OrderedFloat<f64>>),
    Booleans(Array<bool>),
    String(Array<u8>),
    Boxes(Array<Boxed>),
}

#[repr(C)]
struct Repr {
    pub tag: u8,
    pub ds: usize,
    pub rk: SmallVec<[usize; INLINE_SIZE]>,
    pub s: usize,
    pub e: usize,
}

impl List {
    #[inline(always)]
    pub fn empty() -> Self {
        Self::Integers(Array::empty())
    }

    #[inline]
    pub fn string(data: &str) -> Self {
        let size = data.len();
        unsafe { Array::<u8>::alloc_iter(&[size], data.bytes()).unwrap() }
    }

    #[inline(always)]
    pub fn get_type(&self) -> u8 {
        match &self {
            Self::Integers(_) => b'I',
            Self::Doubles(_) => b'F',
            Self::Booleans(_) => b'B',
            Self::String(_) => b'C',
            Self::Boxes(_) => b'X',
        }
    }

    #[inline(always)]
    pub fn to_cast(self, t: u8) -> Status<Self> {
        with_list!(self, |l| l.to_cast(t))
    }

    #[inline(always)]
    pub fn is_truthy(&self) -> bool {
        if self.is_empty() {
            return true;
        }

        match &self {
            Self::Integers(l) => l.list()[0].to_bool().unwrap(),
            Self::Doubles(l) => l.list()[0].to_bool().unwrap(),
            Self::Booleans(l) => l.list()[0],
            Self::String(l) => l.list()[0].to_bool().unwrap(),
            Self::Boxes(_) => false,
        }
    }

    #[inline]
    pub fn listi(&self) -> Status<&[i64]> {
        if let Self::Integers(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline]
    pub fn listf(&self) -> Status<&[OrderedFloat<f64>]> {
        if let Self::Doubles(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline]
    pub fn listb(&self) -> Status<&[bool]> {
        if let Self::Booleans(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline]
    pub fn listc(&self) -> Status<&[u8]> {
        if let Self::String(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline]
    pub fn listx(&self) -> Status<&[Boxed]> {
        if let Self::Boxes(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline(always)]
    pub unsafe fn listi_unchecked(&self) -> &[i64] {
        let Self::Integers(l) = &self else { unsafe { hint::unreachable_unchecked() } };
        l.list()
    }

    #[inline(always)]
    pub unsafe fn listf_unchecked(&self) -> &[OrderedFloat<f64>] {
        let Self::Doubles(l) = &self else { unsafe { hint::unreachable_unchecked() } };
        l.list()
    }

    #[inline(always)]
    pub unsafe fn listb_unchecked(&self) -> &[bool] {
        let Self::Booleans(l) = &self else { unsafe { hint::unreachable_unchecked() } };
        l.list()
    }

    #[inline(always)]
    pub unsafe fn listc_unchecked(&self) -> &[u8] {
        let Self::String(l) = &self else { unsafe { hint::unreachable_unchecked() } };
        l.list()
    }

    #[inline(always)]
    pub unsafe fn listx_unchecked(&self) -> &[Boxed] {
        let Self::Boxes(l) = &self else { unsafe { hint::unreachable_unchecked() } };
        l.list()
    }

    #[inline(always)]
    pub fn rank(&self) -> &[usize] {
        let hdr: &Repr = unsafe { std::mem::transmute(self) };
        &hdr.rk
    }

    pub fn dims(&self) -> usize {
        let hdr: &Repr = unsafe { std::mem::transmute(self) };
        hdr.ds
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        let hdr: &Repr = unsafe { std::mem::transmute(self) };
        hdr.e - hdr.s
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
    pub fn unranked(
        arena: &Arena, _original: usize, trailing_rank: &[usize], lists: Vec<List>,
    ) -> Status<List> {
        if lists.is_empty() {
            return Ok(List::Integers(Array::empty()));
        } else if lists.len() == 1 {
            let f = lists[0].clone();
            return Ok(f);
        }

        let mut box_it = false;
        let rank = lists[0].rank();
        let t = lists[0].get_type();
        for list in lists.iter().skip(1) {
            if !eq_rank(rank, list.rank()) || t != list.get_type() {
                box_it = true;
                break;
            }
        }

        if box_it {
            unsafe {
                Array::<Boxed>::try_alloc_iter(
                    trailing_rank,
                    lists.into_iter().map(|n| scalar::boxed(&arena, n)),
                )
                .map(List::Boxes)
            }
        } else {
            let mut rank: SmallVec<[usize; INLINE_SIZE]> =
                if eq_rank(rank, &[1]) { smallvec![] } else { rank.into() };
            rank.extend_from_slice(trailing_rank);

            let diff = lists[0].rank().iter().product::<usize>() * lists.len()
                / rank.iter().product::<usize>();
            if diff != 1 {
                rank.push(diff);
            }
            // Same type
            unsafe {
                match t {
                    b'I' => Array::<i64>::alloc_iter(
                        &rank,
                        lists.iter().flat_map(|n| n.listi_unchecked()).copied(),
                    ),
                    b'F' => Array::<OrderedFloat<f64>>::alloc_iter(
                        &rank,
                        lists.iter().flat_map(|n| n.listf_unchecked()).copied(),
                    ),
                    b'B' => Array::<bool>::alloc_iter(
                        &rank,
                        lists.iter().flat_map(|n| n.listb_unchecked()).copied(),
                    ),
                    b'C' => Array::<u8>::alloc_iter(
                        &rank,
                        lists.iter().flat_map(|n| n.listc_unchecked()).copied(),
                    ),
                    b'X' => Array::<Boxed>::alloc_iter(
                        &rank,
                        lists.iter().flat_map(|n| n.listx_unchecked()).copied(),
                    ),
                    _ => hint::unreachable_unchecked(),
                }
            }
        }
    }

    // Assumes lists have already been checked to match
    #[inline]
    pub fn unranked_unchecked(trailing_rank: &[usize], lists: Vec<List>) -> Status<List> {
        if lists.is_empty() {
            return Ok(List::Integers(Array::empty()));
        } else if lists.len() == 1 {
            let f = lists[0].clone();
            return Ok(f);
        }

        let rank = lists[0].rank();
        let t = lists[0].get_type();
        let mut rank: SmallVec<[usize; INLINE_SIZE]> =
            if eq_rank(rank, &[1]) { smallvec![] } else { rank.into() };
        rank.extend_from_slice(trailing_rank);

        let diff = lists[0].rank().iter().product::<usize>() * lists.len()
            / rank.iter().product::<usize>();
        if diff != 1 {
            rank.push(diff);
        }

        unsafe {
            match t {
                b'I' => Array::<i64>::alloc_iter(
                    &rank,
                    lists.iter().flat_map(|n| n.listi_unchecked()).copied(),
                ),
                b'F' => Array::<OrderedFloat<f64>>::alloc_iter(
                    &rank,
                    lists.iter().flat_map(|n| n.listf_unchecked()).copied(),
                ),
                b'B' => Array::<bool>::alloc_iter(
                    &rank,
                    lists.iter().flat_map(|n| n.listb_unchecked()).copied(),
                ),
                b'C' => Array::<u8>::alloc_iter(
                    &rank,
                    lists.iter().flat_map(|n| n.listc_unchecked()).copied(),
                ),
                b'X' => Array::<Boxed>::alloc_iter(
                    &rank,
                    lists.iter().flat_map(|n| n.listx_unchecked()).copied(),
                ),
                _ => hint::unreachable_unchecked(),
            }
        }
    }

    // SAFETY: Assumes all sizes within the struct are valid in respect to the data
    #[inline]
    pub fn ranked(&self, dims: usize) -> Status<Vec<Self>> {
        if dims == 0 {
            let mut v = Vec::with_capacity(self.len());
            for i in 0..self.len() {
                v.push(unsafe { self.sliced(i, i + 1, &[1]) });
            }
            Ok(v)
        } else if dims >= self.dims() {
            Ok(vec![self.clone()])
        } else {
            let ct: usize = self.rank().iter().take(dims).product();
            unsafe { std::hint::assert_unchecked(self.len() % ct == 0) }

            let size = self.len() / ct;
            let mut v = Vec::with_capacity(size);
            for i in 0..size {
                v.push(unsafe { self.sliced(i * ct, i * ct + ct, &self.rank()[0..dims]) });
            }
            return Ok(v);
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn sliced(&self, start: usize, end: usize, r: &[usize]) -> Self {
        let mut l = self.clone();
        let n: &mut Repr = unsafe { std::mem::transmute(&mut l) };
        n.ds = r.len();
        n.rk = SmallVec::from_slice(r);
        n.s = start;
        n.e = end;
        drop(n);
        l
    }
}

// TODO: Various functions implemented for `List`

#[derive(Clone, Debug, Hash)]
#[repr(C)]
pub struct Array<T: Scalar> {
    pub(crate) ds: usize,
    pub(crate) rk: SmallVec<[usize; INLINE_SIZE]>,
    pub(crate) dt: Scalars<T>,
}

impl<T: Scalar> Array<T> {
    #[inline(always)]
    pub fn new(dt: impl Into<Scalars<T>>, ds: usize, rk: SmallVec<[usize; INLINE_SIZE]>) -> Self {
        Self { dt: dt.into(), ds, rk }
    }

    #[inline]
    pub fn empty() -> Self {
        Self { dt: Default::default(), ds: 0, rk: smallvec![] }
    }

    #[inline]
    pub fn from_vec(data: Rc<[T]>) -> List {
        let size = data.len();
        T::wrap(Self { dt: data.into(), ds: 1, rk: smallvec![size] })
    }

    #[inline]
    pub fn mat(data: Vec<T>, ds: usize, rk: SmallVec<[usize; INLINE_SIZE]>) -> List {
        T::wrap(Self { dt: data.into(), ds, rk })
    }

    #[inline]
    pub fn mat_ref(data: &[T], ds: usize, rk: &[usize]) -> List {
        T::wrap(Self { dt: data.into(), ds, rk: rk.into() })
    }

    #[inline]
    pub fn scalar(data: T) -> List {
        T::wrap(Self { dt: data.into(), ds: 1, rk: smallvec![1] })
    }

    #[inline(always)]
    pub fn list(&self) -> &[T] {
        self.dt.list()
    }

    #[inline(always)]
    pub fn list_mut(&mut self) -> &mut [T] {
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

    // Swaps out rank. It is in place if possible
    #[inline]
    pub fn with_rank(mut self, r: &[usize]) -> Status<List> {
        let count: usize = r.iter().product();
        if count == self.len() {
            if INLINE_SIZE >= self.dims() || INLINE_SIZE < r.len() {
                self.rk = SmallVec::from_slice(r);
            } else {
                self.rk.clear();
                self.rk.extend_from_slice(r);
            }
            self.ds = self.rk.len();
            Ok(T::wrap(self))
        } else if self.is_empty() {
            Ok(T::wrap(self))
        } else {
            let bucket = self.list().into_iter().copied().cycle();
            unsafe { Self::alloc_iter(r, bucket) }
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn sliced(&self, start: usize, end: usize, r: &[usize]) -> Self {
        Self { dt: self.dt.sliced(start, end), ds: r.len(), rk: SmallVec::from_slice(r) }
    }

    #[inline(always)]
    pub(crate) unsafe fn try_alloc_iter<U>(r: &[usize], bucket: U) -> Status<Self>
    where
        U: Iterator<Item = Status<T>>,
    {
        let count: usize = r.iter().product();
        let dims = r.len();
        if dims == 0 {
            return Ok(Array::empty());
        }
        let rank = SmallVec::from_slice(r);
        Ok(unsafe { Self::new(Scalars::try_from_iter(bucket, count)?, dims, rank) })
    }

    // SAFETY: Assumes called has provided valid sizes
    #[inline(always)]
    pub(crate) unsafe fn alloc_iter<U>(r: &[usize], bucket: U) -> Status<List>
    where
        U: Iterator<Item = T>,
    {
        let count: usize = r.iter().product();
        let dims = r.len();
        if dims == 0 {
            return Ok(T::wrap(Array::empty()));
        }
        let rank = SmallVec::from_slice(r);
        let bucket = SizedScalars::new(bucket, count);
        Ok(T::wrap(unsafe { Self::new(Scalars::from_iter(bucket, count), dims, rank) }))
    }

    // SAFETY: Assumes all sizes within the struct are valid in respect to the data
    #[inline]
    pub fn ranked(&self, dims: usize) -> Status<Vec<Self>> {
        if dims == 0 {
            let mut v = Vec::with_capacity(self.len());
            for i in 0..self.len() {
                v.push(unsafe { self.sliced(i, i + 1, &[1]) });
            }
            Ok(v)
        } else if dims >= self.dims() {
            Ok(vec![self.clone()])
        } else {
            let ct: usize = self.rank().iter().take(dims).product();
            unsafe { std::hint::assert_unchecked(self.len() % ct == 0) }

            let size = self.len() / ct;
            let mut v = Vec::with_capacity(size);
            for i in 0..size {
                v.push(unsafe { self.sliced(i * ct, i * ct + ct, &self.rank()[0..dims]) });
            }
            return Ok(v);
        }
    }

    // Assumes lists have already been checked to match
    #[inline]
    pub fn unranked_unchecked(trailing_rank: &[usize], lists: Vec<Array<T>>) -> Status<List> {
        if lists.is_empty() {
            return Ok(T::wrap(Array::empty()));
        } else if lists.len() == 1 {
            let f = lists[0].clone();
            return Ok(T::wrap(f));
        }

        let rank = lists[0].rank();
        let mut rank: SmallVec<[usize; INLINE_SIZE]> =
            if eq_rank(rank, &[1]) { smallvec![] } else { rank.into() };
        rank.extend_from_slice(trailing_rank);

        let diff = lists[0].rank().iter().product::<usize>() * lists.len()
            / rank.iter().product::<usize>();
        if diff != 1 {
            rank.push(diff);
        }
        unsafe { Self::alloc_iter(&rank, lists.iter().flat_map(|n| n.list()).copied()) }
    }
}

#[inline]
fn eq_rank(a: &[usize], b: &[usize]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(l, r)| l == r)
}

// SAFETY: Determined by called
fn push_and_get<'a, T>(v: &mut Vec<T>, s: T) -> &'a str
where
    T: AsRef<str>,
{
    v.push(s);
    let r: &str = v.last().unwrap().as_ref();
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

impl Display for Array<Boxed> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_scalar() {
            write!(f, "{}", self.list()[0])?;
        } else if self.is_empty() {
            hint::cold_path();
            return Ok(());
        } else if self.dims() < 3 {
            let mut formatted = Vec::with_capacity(self.len());
            let mut strings: Vec<Vec<&str>> = Vec::with_capacity(self.len());

            let mut x = 0;
            let mut y = 1;
            for b in self.list().iter() {
                // SAFETY: formatted is not dropped until strings
                let fm;
                fm = push_and_get(&mut formatted, format!("{}", b));
                // Oh rust strs, how you fuck with me
                x = usize::max(
                    x,
                    fm.chars()
                        .zip(0..)
                        .find(|(c, _)| *c == '\n')
                        .map_or_else(|| fm.chars().count(), |(_, i)| i),
                );
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

macro_rules! impl_display {
    ($($t:ty),*) => {$(
        impl Display for Array<$t> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.is_scalar() {
                    write!(f, "{}", self.list()[0])?;
                } else if self.is_empty() {
                    hint::cold_path();
                    return Ok(());
                } else if self.dims() < 3 {
                    let mut formatted = Vec::with_capacity(self.len());
                    let mut strings: Vec<Vec<&str>> = Vec::with_capacity(self.len());

                    let mut x = 0;
                    let mut y = 1;
                    let is_str = <$t>::get_type() == b'C';
                    for num in self.list().iter() {
                        let fm = push_and_get(&mut formatted, num.fmt_lead());
                        x = usize::max(x, fm.chars().count());
                        strings.push(vec![fm]);
                    }
                    for ys in strings.iter() {
                        y = usize::max(y, ys.len());
                    }

                    let cols = self.rank()[0];
                    unsafe {
                        std::hint::assert_unchecked(self.len() % cols == 0);
                    }

                    let rows = self.len() / cols;
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
    )*};
}

impl_display![i64, OrderedFloat<f64>, bool, u8];

impl Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        with_list!(self, |v| write!(f, "{v}"))
    }
}

pub struct SizedScalars<T: Iterator<Item = K>, K: Scalar> {
    iter: T,
    size: usize,
    c: usize,
}

impl<T: Iterator<Item = K>, K: Scalar> SizedScalars<T, K> {
    pub fn new(iter: T, size: usize) -> Self {
        Self { iter, size, c: 0 }
    }
}

impl<T: Iterator<Item = K>, K: Scalar> Iterator for SizedScalars<T, K> {
    type Item = K;
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
#[repr(C)]
pub struct Scalars<T: Scalar> {
    pub(crate) s: usize,
    pub(crate) e: usize,
    pub(crate) v: Rc<[T]>,
}

impl<T: Scalar> Scalars<T> {
    #[inline(always)]
    pub(crate) fn list(&self) -> &[T] {
        // SAFETY: Creator must guarantee boundaries
        &self.v[self.s..self.e]
    }

    #[inline(always)]
    pub(crate) fn list_mut(&mut self) -> &mut [T] {
        if Rc::get_mut(&mut self.v).is_none() {
            self.v = self.v[self.s..self.e].iter().copied().collect();
            self.e -= self.s;
            self.s = 0;
        }

        let all = Rc::get_mut(&mut self.v).unwrap();
        &mut all[self.s..self.e]
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        unsafe { std::hint::assert_unchecked(self.e >= self.s) }
        self.e - self.s
    }

    #[inline(always)]
    pub(crate) fn sliced(&self, start: usize, end: usize) -> Self {
        Self { v: self.v.clone(), s: self.s + start, e: self.s + end }
    }

    #[inline(always)]
    pub(crate) unsafe fn from_iter<U>(it: SizedScalars<U, T>, end: usize) -> Scalars<T>
    where
        U: Iterator<Item = T>,
    {
        Self { v: it.take(end).collect(), s: 0, e: end }
    }

    #[inline(always)]
    pub(crate) unsafe fn try_from_iter<U>(it: U, end: usize) -> Status<Scalars<T>>
    where
        U: Iterator<Item = Status<T>>,
    {
        Ok(Self { v: it.take(end).collect::<Status<Rc<[T]>>>()?, s: 0, e: end })
    }
}

impl<T: Scalar> From<Vec<T>> for Scalars<T> {
    #[inline(always)]
    fn from(e: Vec<T>) -> Self {
        let size = e.len();
        Self { v: e.into(), s: 0, e: size }
    }
}

impl<T: Scalar> From<Rc<[T]>> for Scalars<T> {
    #[inline(always)]
    fn from(e: Rc<[T]>) -> Self {
        let size = e.len();
        Self { v: e, s: 0, e: size }
    }
}

impl<T: Scalar> From<T> for Scalars<T> {
    #[inline(always)]
    fn from(s: T) -> Self {
        Self { v: Rc::new([s]), s: 0, e: 1 }
    }
}

impl<T: Scalar> From<&[T]> for Scalars<T> {
    #[inline(always)]
    fn from(s: &[T]) -> Self {
        let size = s.len();
        Self { v: s.into(), s: 0, e: size }
    }
}
