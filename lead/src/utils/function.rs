use std::collections::VecDeque;
use std::hint;

use crate::function::Opts;
use crate::types::list::{Array, List};
use crate::types::scalar::{self, Boxed, Scalar};

use super::Arena;
use super::error::{Status, err};

use smallvec::{SmallVec, smallvec};

// Returns a vector of `ListView`s, i.e. mutable slices pointing to adjacent values of rank `n`.
// Assumes the result is rank equivalent to the input
#[inline]
pub fn applym<'a, T>(l: &'a mut Array<T>, r: u8) -> (VecDeque<&'a mut [T]>, SmallVec<[usize; 4]>)
where
    T: Scalar,
{
    if l.is_empty() {
        hint::cold_path();
        return (vec![].into(), smallvec![0]);
    }

    let r = usize::min(r as usize, l.dims());
    let mut rank: SmallVec<[usize; 4]> = l.rank()[..r].into();
    if r == 0 {
        rank.push(1);
    }
    let skip: usize = rank.iter().product();

    let l = l.list_mut();
    let mut v = VecDeque::with_capacity(l.len() / skip);
    for chunk in l.chunks_exact_mut(skip) {
        v.push_back(chunk);
    }
    (v, rank)
}

// Applies optionally monadic or dyadic
#[inline]
pub fn apply_loose<U, K, T>(o: &Opts, a: Array<U>, b: Option<Array<K>>, f: T) -> Status<List>
where
    T: Fn(List, Option<List>) -> Status<List>,
    U: Scalar,
    K: Scalar,
{
    if let Some(b) = b {
        apply_loosed(&o.a, a, b, o.rd, |a, b| f(a, Some(b)))
    } else {
        apply_loosem(&o.a, a, o.rm, |a| f(a, None))
    }
}

// Applies function generally over monadic input. Does not make any assumptions
#[inline]
pub fn apply_loosem<U, T>(arena: &Arena, l: Array<U>, r: u8, f: T) -> Status<List>
where
    T: Fn(List) -> Status<List>,
    U: Scalar,
{
    let dims = l.dims();
    let rank = l.rank().to_vec();
    let lists = l.ranked(r as usize)?;
    let result = lists.into_iter().map(|a| f(U::wrap(a))).collect::<Status<Vec<List>>>()?;
    let res = List::unranked(arena, dims, &rank[r as usize..], result);
    // Drop after windows are collected and memcpy'd
    drop(l);
    res
}

// Applies function generally over dyadic input. Does not make assumptions
#[inline]
pub fn apply_loosed<U, K, T>(
    arena: &Arena, a: Array<U>, b: Array<K>, r: [u8; 2], f: T,
) -> Status<List>
where
    T: Fn(List, List) -> Status<List>,
    U: Scalar,
    K: Scalar,
{
    let left = a.ranked(r[0] as usize)?;
    let right = b.ranked(r[1] as usize)?;
    let no_swap = left.len() >= right.len();
    let values;
    let from;
    if no_swap {
        from = (a.dims(), a.rank(), usize::min(r[0] as usize, a.dims()));
        if a.len() % b.len() != 0 {
            return err!(Domain);
        }

        values = left
            .iter()
            .zip(right.iter().cycle())
            .map(|(l, r)| {
                let l = U::wrap(l.clone());
                let r = K::wrap(r.clone());
                f(l, r)
            })
            .collect::<Status<Vec<List>>>()?;
    } else {
        from = (b.dims(), b.rank(), usize::min(r[1] as usize, b.dims()));
        if b.len() % a.len() != 0 {
            return err!(Domain);
        }

        values = right
            .iter()
            .zip(left.iter().cycle())
            .map(|(l, r)| {
                let l = K::wrap(l.clone());
                let r = U::wrap(r.clone());
                f(r, l)
            })
            .collect::<Status<Vec<List>>>()?;
    }

    let r = values[0].rank().to_owned();
    let mut boxit = false;
    for v in values.iter().skip(1) {
        if v.rank() != r || values[0].get_type() != v.get_type() {
            boxit = true;
            break;
        }
    }
    let res = if boxit {
        List::Boxes(unsafe {
            Array::<Boxed>::try_alloc_iter(
                from.1,
                values.into_iter().map(|v| scalar::boxed(arena, v)),
            )?
        })
    } else {
        List::unranked_unchecked(&from.1[from.2..], values)?
    };
    drop(a);
    drop(b);
    Ok(res)
}
