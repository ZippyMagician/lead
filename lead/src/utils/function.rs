use std::hint;
use std::mem::ManuallyDrop;

use crate::function::Opts;
use crate::types::list::{List, ListWindow};
use crate::types::scalar::Scalar;

use super::Arena;
use super::error::{Status, err};

use smallvec::{SmallVec, smallvec};

// Returns a vector of `ListView`s, i.e. mutable slices pointing to adjacent values of rank `n`.
// Assumes the result is rank equivalent to the input
#[inline]
pub fn applym<'a>(l: &'a mut List, r: u8) -> (Vec<&'a mut [Scalar]>, SmallVec<[usize; 4]>) {
    if l.is_empty() {
        hint::cold_path();
        return (vec![], smallvec![0]);
    }

    let r = usize::min(r as usize, l.rank().len());
    let rank: SmallVec<[usize; 4]> = l.rank()[..r].into();
    let skip: usize = l.rank()[r..].iter().product();

    let l = l.list_mut();
    let mut v = Vec::with_capacity(l.len() / skip);
    for chunk in l.chunks_exact_mut(skip) {
        v.push(chunk);
    }
    (v, rank)
}

// Applies optionally monadic or dyadic
#[inline]
pub fn apply_loose<T>(o: &Opts, a: List, b: Option<List>, f: T) -> Status<List>
where
    T: Fn(ManuallyDrop<ListWindow>, Option<ManuallyDrop<ListWindow>>) -> Status<List>,
{
    if let Some(b) = b {
        apply_loosed(&o.a, a, b, o.rd, |a, b| f(a, Some(b)))
    } else {
        apply_loosem(&o.a, a, o.rm, |a| f(a, None))
    }
}

// Applies function generally over monadic input. Does not make any assumptions
#[inline]
pub fn apply_loosem<T>(arena: &Arena, l: List, r: u8, f: T) -> Status<List>
where
    T: Fn(ManuallyDrop<ListWindow>) -> Status<List>,
{
    let l = ManuallyDrop::new(l);
    let dims = l.dims();
    let rank = l.rank().to_vec();
    let lists = l.ranked(r as usize)?;
    let single = lists.len() == 1;
    let result =
        lists.into_iter().map(|a| f(ManuallyDrop::new(a))).collect::<Status<Vec<List>>>()?;
    let mut res = List::unranked(arena, dims, &rank[r as usize..], result);
    // Avoid issues with single length windows being dropped
    if single {
        std::hint::cold_path();
        res = res.clone();
    }
    // Drop after windows are collected and memcpy'd
    drop(ManuallyDrop::into_inner(l));
    res
}

// Applies function generally over dyadic input. Does not make assumptions
#[inline]
pub fn apply_loosed<T>(arena: &Arena, a: List, b: List, r: [u8; 2], f: T) -> Status<List>
where
    T: Fn(ManuallyDrop<ListWindow>, ManuallyDrop<ListWindow>) -> Status<List>,
{
    let a = ManuallyDrop::new(a);
    let b = ManuallyDrop::new(b);
    let left = a.ranked(r[0] as usize)?;
    let right = b.ranked(r[1] as usize)?;
    let no_swap = left.len() >= right.len();
    let (mapper, value) = if no_swap { (left, right) } else { (right, left) };

    if mapper.len() % value.len() != 0 {
        return err!(Domain);
    }

    let mut values = mapper
        .iter()
        .zip(value.iter().cycle())
        .map(|(&l, &r)| {
            let l = ManuallyDrop::new(l);
            let r = ManuallyDrop::new(r);
            if no_swap { f(l, r) } else { f(r, l) }
        })
        .collect::<Status<Vec<List>>>()?;
    if values.len() == 1 {
        // See apply_loosem for explanation
        std::hint::cold_path();
        values = values.clone();
    }

    let r = values[0].rank().to_owned();
    let mut boxit = false;
    for v in values.iter().skip(1) {
        if v.rank() != r {
            boxit = true;
            break;
        }
    }
    let res = if boxit {
        let mut boxes = Vec::with_capacity(values.len());
        for v in values {
            boxes.push(Scalar::boxed(arena, v)?);
        }
        List::from_vec(boxes)
    } else {
        List::unranked(arena, values[0].dims() + 1, &[values.len()], values)
    };
    // See above
    drop(ManuallyDrop::into_inner(a));
    drop(ManuallyDrop::into_inner(b));
    res
}
