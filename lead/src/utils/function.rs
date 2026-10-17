use std::hint;

use crate::function::Opts;
use crate::types::list::List;
use crate::types::scalar::Scalar;

use super::Arena;
use super::error::{Status, err};

use smallvec::{SmallVec, smallvec};

// Returns a vector of `ListView`s, i.e. mutable slices pointing to adjacent values of rank `n`.
// Assumes the result is rank equivalent to the input
#[inline]
pub fn applym<'a, T>(l: &'a mut List, r: u8) -> (Vec<&'a mut [Scalar]>, SmallVec<[usize; 4]>) {
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
    T: Fn(List, Option<List>) -> Status<List>,
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
    T: Fn(List) -> Status<List>,
{
    let dims = l.dims();
    let rank = l.rank().to_vec();
    let lists = l.ranked(r as usize)?;
    List::unranked(
        arena,
        dims,
        &rank[r as usize..],
        lists.into_iter().map(f).collect::<Status<Vec<List>>>()?,
    )
}

// Applies function generally over dyadic input. Does not make assumptions
#[inline]
pub fn apply_loosed<T>(arena: &Arena, a: List, b: List, r: [u8; 2], f: T) -> Status<List>
where
    T: Fn(List, List) -> Status<List>,
{
    let left_dims = a.dims();
    let right_dims = b.dims();
    let left_rank: SmallVec<[usize; 4]> = SmallVec::from(a.rank());
    let right_rank: SmallVec<[usize; 4]> = SmallVec::from(b.rank());

    let left = a.ranked(r[0] as usize)?;
    let right = b.ranked(r[1] as usize)?;
    let no_swap = left.len() >= right.len();
    let (mapper, value) = if no_swap {
        (left, right)
    } else {
        (right, left)
    };

    if mapper.len() % value.len() != 0 {
        return err!(Domain);
    }

    let values = mapper
        .into_iter()
        .zip(value.into_iter().cycle())
        .map(|(l, r)| if no_swap { f(l, r) } else { f(r, l) })
        .collect::<Status<Vec<List>>>()?;
    if no_swap {
        List::unranked(arena, left_dims, &left_rank, values)
    } else {
        List::unranked(arena, right_dims, &right_rank, values)
    }
}
