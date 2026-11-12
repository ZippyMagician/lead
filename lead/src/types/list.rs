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
use crate::utils::error::{Error, Status, err};
use crate::utils::list::with_list;

use ordered_float::OrderedFloat;
use smallvec::{SmallVec, smallvec};

pub(crate) const INLINE_SIZE: usize = 2;

#[derive(Clone, Debug, Hash)]
#[repr(u8, C)]
pub enum List {
    Integers(Array<i64>) = b'I',
    Doubles(Array<OrderedFloat<f64>>) = b'F',
    Booleans(Array<bool>) = b'B',
    String(Array<u8>) = b'C',
    Boxes(Array<Boxed>) = b'X',
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
    pub unsafe fn treat_as<U: Scalar>(self) -> Array<U> {
        match U::get_type() {
            b'I' => unsafe { self.arrayi_unchecked().treat_as() },
            b'F' => unsafe { self.arrayf_unchecked().treat_as() },
            b'B' => unsafe { self.arrayb_unchecked().treat_as() },
            b'C' => unsafe { self.arrayc_unchecked().treat_as() },
            b'X' => unsafe { self.arrayx_unchecked().treat_as() },
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline(always)]
    pub unsafe fn treat_as_ref<U: Scalar>(&self) -> &Array<U> {
        match U::get_type() {
            b'I' => unsafe { self.as_arrayi_unchecked().treat_as_ref() },
            b'F' => unsafe { self.as_arrayf_unchecked().treat_as_ref() },
            b'B' => unsafe { self.as_arrayb_unchecked().treat_as_ref() },
            b'C' => unsafe { self.as_arrayc_unchecked().treat_as_ref() },
            b'X' => unsafe { self.as_arrayx_unchecked().treat_as_ref() },
            _ => unsafe { hint::unreachable_unchecked() },
        }
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

    // TODO: Add on to these as I need them
    #[inline]
    pub unsafe fn arrayi_unchecked(self) -> Array<i64> {
        match self {
            Self::Integers(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn as_arrayi_unchecked(&self) -> &Array<i64> {
        match self {
            Self::Integers(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn arrayf_unchecked(self) -> Array<OrderedFloat<f64>> {
        match self {
            Self::Doubles(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn as_arrayf_unchecked(&self) -> &Array<OrderedFloat<f64>> {
        match self {
            Self::Doubles(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn arrayc_unchecked(self) -> Array<u8> {
        match self {
            Self::String(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn as_arrayc_unchecked(&self) -> &Array<u8> {
        match self {
            Self::String(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn arrayb_unchecked(self) -> Array<bool> {
        match self {
            Self::Booleans(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn as_arrayb_unchecked(&self) -> &Array<bool> {
        match self {
            Self::Booleans(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn arrayx_unchecked(self) -> Array<Boxed> {
        match self {
            Self::Boxes(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub unsafe fn as_arrayx_unchecked(&self) -> &Array<Boxed> {
        match self {
            Self::Boxes(a) => a,
            _ => unsafe {
                hint::unreachable_unchecked();
            },
        }
    }

    #[inline]
    pub fn listi(&self) -> Status<&[i64]> {
        if let Self::Integers(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline]
    pub fn listi_mut(&mut self) -> Status<&mut [i64]> {
        if let Self::Integers(l) = self { Ok(l.list_mut()) } else { err!(Value) }
    }

    #[inline]
    pub fn listf(&self) -> Status<&[OrderedFloat<f64>]> {
        if let Self::Doubles(l) = &self { Ok(l.list()) } else { err!(Value) }
    }

    #[inline]
    pub fn listf_mut(&mut self) -> Status<&mut [OrderedFloat<f64>]> {
        if let Self::Doubles(l) = self { Ok(l.list_mut()) } else { err!(Value) }
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

    // Swaps out rank. It is in place if possible
    // Generic version of `Array::<T>::with_rank`
    #[inline]
    pub fn with_rank(mut self, r: &[usize]) -> Status<List> {
        let count: usize = r.iter().product();
        if count == self.len() {
            let hdr: &mut Repr = unsafe { std::mem::transmute(&mut self) };
            if INLINE_SIZE >= hdr.ds || INLINE_SIZE < r.len() {
                hdr.rk = SmallVec::from_slice(r);
            } else {
                hdr.rk.clear();
                hdr.rk.extend_from_slice(r);
            }
            hdr.ds = hdr.rk.len();
            Ok(self)
        } else if self.is_empty() {
            Ok(self)
        } else {
            with_list!(self, |s| {
                let bucket = s.list().into_iter().copied().cycle();
                unsafe { Array::alloc_iter(r, bucket) }
            })
        }
    }

    #[inline]
    pub fn nth(&mut self, i: usize) -> Status<List> {
        with_list!(self, |s| {
            Ok(Array::scalar(s.list().iter().copied().nth(i).ok_or(Error::Domain)?))
        })
    }

    // Unranks generic amount of Lists based on some data
    #[inline]
    pub fn unranked(
        arena: &Arena, _original: usize, trailing_rank: &[usize], mut lists: Vec<List>,
    ) -> Status<List> {
        if lists.is_empty() {
            return Ok(List::Integers(Array::empty()));
        } else if lists.len() == 1 {
            let f = lists.remove(0);
            return Ok(f);
        }

        let mut box_it = false;
        let rank = lists[0].rank();
        let t = lists[0].get_type();
        for list in lists.iter().skip(1) {
            if rank != list.rank() || t != list.get_type() {
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
    // Generic version of `Array::<T>::unranked_unchecked`
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

    // Generic version of `Array::<T>::ranked`
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

    // Generic version of `Array::<T>::sliced`
    #[inline(always)]
    pub(crate) unsafe fn sliced(&self, start: usize, end: usize, r: &[usize]) -> Self {
        let hdr: &Repr = unsafe { std::mem::transmute(self) };
        let mut l = self.clone();
        let n: &mut Repr = unsafe { std::mem::transmute(&mut l) };
        n.ds = r.len();
        n.rk = SmallVec::from_slice(r);
        n.s = hdr.s + start; // if we are taking from an already sliced value
        n.e = hdr.s + end;
        l
    }
}

impl<T: Scalar> From<Array<T>> for List {
    #[inline(always)]
    fn from(f: Array<T>) -> Self {
        T::wrap(f)
    }
}

// TODO: Various functions implemented for `List`

#[derive(Clone, Debug)]
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
    pub fn get_fill() -> Status<T> {
        T::get_fill()
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
    pub fn get_type(&self) -> u8 {
        T::get_type()
    }

    #[inline(always)]
    pub fn list(&self) -> &[T] {
        self.dt.list()
    }

    #[inline(always)]
    pub fn take_list(self) -> Scalars<T> {
        self.dt
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
        unsafe { Self::alloc_iter_inner(r, bucket).map(T::wrap) }
    }

    // SAFETY: See above
    #[inline(always)]
    pub(crate) unsafe fn alloc_iter_inner<U>(r: &[usize], bucket: U) -> Status<Array<T>>
    where
        U: Iterator<Item = T>,
    {
        let count: usize = r.iter().product();
        let dims = r.len();
        if dims == 0 {
            return Ok(Self::empty());
        }
        let rank = SmallVec::from_slice(r);
        Ok(unsafe { Self::new(Scalars::from_iter(bucket, count), dims, rank) })
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
                // SAFETY: See `<Boxed as Display>::fmt`
                if b.is_null() {
                    err!(MemoryFault, "Lost Box"; !);
                }
                // SAFETY: formatted is not dropped until strings
                let fm;
                fm = push_and_get(&mut formatted, format!("{}", unsafe { b.read() }));
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

macro_rules! impl_display_inner {
    ([u8], $self:ident, $cols:ident, $formatted:ident, $x:ident, $strings:ident) => {
        for bytes in $self.list().chunks_exact($cols) {
            let fm = push_and_get(&mut $formatted, ::std::str::from_utf8(bytes).unwrap());
            $x = usize::max($x, fm.chars().count());
            $strings.push(vec![fm]);
        }
        $cols = 1;
    };

    ([$($t:tt)*], $self:ident, $cols:ident, $formatted:ident, $x:ident, $strings:ident) => {
        for num in $self.list().iter() {
            let fm = push_and_get(&mut $formatted, num.fmt_lead());
            $x = usize::max($x, fm.chars().count());
            $strings.push(vec![fm]);
        }
    };

    ([u8], $self:ident) => {
        $self.list()[0] as char
    };

    ([$($t:tt)*], $self:ident) => {
        $self.list()[0].fmt_lead()
    };
}

macro_rules! impl_display {
    ($([$($t:tt)*]),*) => {$(
        impl Display for Array<$($t)*> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.is_scalar() {
                    write!(f, "{}", impl_display_inner!([$($t)*], self))?;
                } else if self.is_empty() {
                    hint::cold_path();
                    return Ok(());
                } else if self.dims() < 3 {
                    let mut formatted = Vec::with_capacity(self.len());
                    let mut strings: Vec<Vec<&str>> = Vec::with_capacity(self.len());

                    let mut x = 0;
                    let mut y = 1;
                    #[allow(unused_mut)]
                    let mut cols = self.rank()[0];
                    unsafe {
                        std::hint::assert_unchecked(self.len() % cols == 0);
                    }
                    let rows = self.len() / cols;

                    impl_display_inner!([$($t)*], self, cols, formatted, x, strings);

                    for ys in strings.iter() {
                        y = usize::max(y, ys.len());
                    }
                    // Currently we have a list of scalar values
                    // x and y are the max size of those scalars.
                    for row in 0..rows {
                        for col in 0..cols {
                            let word = &strings[row * cols + col];
                            assert!(word.len() == 1, "Internal error");
                            let word = word[0];
                            if self.dims() == 1 {
                                write!(f, "{}", word)?;
                            } else {
                                write!(f, "{}{}", " ".repeat(x - word.len()), word)?;
                            }
                            if col < cols - 1 {
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

impl_display!([i64], [OrderedFloat<f64>], [bool], [u8]);

impl Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        with_list!(self, |v| write!(f, "{v}"))
    }
}

#[derive(Clone, Debug, Default)]
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
    pub(crate) unsafe fn from_iter<U>(it: U, end: usize) -> Scalars<T>
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

// just in case
#[test]
fn is_it_fucked() {
    use std::mem::{align_of, offset_of, size_of};
    let base = 8; // offset after tag
    assert_eq!(align_of::<Array<u8>>(), 8);
    assert_eq!(offset_of!(Repr, ds), base + offset_of!(Array<u8>, ds));
    assert_eq!(offset_of!(Repr, rk), base + offset_of!(Array<u8>, rk));
    assert_eq!(offset_of!(Repr, s), base + offset_of!(Array<u8>, dt) + offset_of!(Scalars<u8>, s));
    assert_eq!(offset_of!(Repr, e), base + offset_of!(Array<u8>, dt) + offset_of!(Scalars<u8>, e));
    assert!(size_of::<Repr>() <= size_of::<List>());
}
