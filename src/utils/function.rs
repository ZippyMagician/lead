use std::hint;

use crate::list::List;
use crate::scalar::Scalar;

use super::error::Status;

use smallvec::SmallVec;

// Mutates in place, expects result to have same shape as input
#[inline]
pub fn applym<'a, T>(l: &'a mut List, r: u8, f: T) -> Status
where
    T: Fn(&'a mut [Scalar], &SmallVec<[usize; 4]>) -> Status,
{
    if l.is_empty() {
        hint::cold_path();
        return Ok(());
    }

    let r = usize::min(r as usize, l.rank().len());
    let rank: SmallVec<[usize; 4]> = l.rank()[..r].into();
    let skip: usize = l.rank()[r..].into_iter().product();

    let l = l.list_mut();
    for chunk in l.chunks_exact_mut(skip) {
        f(chunk, &rank)?;
    }

    Ok(())
}

#[inline]
pub fn apply_loosem<T>(l: List, r: u8, f: T) -> Status<List>
where
    T: Fn(List) -> Status<List>,
{
    let dims = l.dims();
    let rank = l.rank().to_vec();
    let lists = l.ranked(r as usize);
    List::unranked(dims, &rank[r as usize..], lists.into_iter().map(f))
}
