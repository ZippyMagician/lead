use crate::function::Opts;
use crate::types::list::{Array, INLINE_SIZE};
use crate::types::scalar::Scalar;

use crate::utils::error::{Status, err};
use crate::utils::symbols::*;

use smallvec::{SmallVec, smallvec};

pub(crate) fn concat<T: Scalar>(a: Array<T>, b: Array<T>, o: &Opts) -> Status<Array<T>> {
    if o.rd[0] == 1 && o.rd[1] == 1 {
        if a.dims() < 2 && b.dims() < 2 {
            let v = with_eco(a.len() + b.len(), |v| {
                v.extend(a.list());
                v.extend(b.list());
                Ok(())
            })?;
            return Ok(Array::new(v, 1, smallvec![a.len() + b.len()]));
        } else if b.dims() < 2 {
            let mut rank = SmallVec::from(a.rank());
            rank[0] += b.len();
            let build = with_eco(rank.iter().product(), |build| {
                for_1!(a; [r, _rk] {
                    build.extend(r);
                    build.extend(b.list());
                });
                Ok(())
            })?;
            return Ok(Array::new(build, a.dims(), rank));
        } else if a.dims() < 2 {
            let mut rank = SmallVec::from(b.rank());
            rank[0] += a.len();
            let build = with_eco(rank.iter().product(), |build| {
                for_1!(b; [r, _rk] {
                    build.extend(a.list());
                    build.extend(r);
                });
                Ok(())
            })?;
            return Ok(Array::new(build, b.dims(), rank));
        }
    } else if a.dims() == b.dims() {
        if a.dims() <= o.rd[0] as usize && b.dims() <= o.rd[1] as usize {
            let mut r = SmallVec::from(a.rank());
            let last = r.len() - 1;
            if r[0..last].iter().zip(&b.rank()[0..last]).any(|(a, b)| a != b) {
                return err!(Domain);
            }
            r[last] += b.rank()[last];

            let v = with_eco(a.len() + b.len(), |v| {
                v.extend(a.list());
                v.extend(b.list());
                Ok(())
            })?;
            return Ok(Array::new(v, r.len(), r));
        } else if b.dims() <= o.rd[1] as usize && o.rd[0] == 1 {
            let mut r = SmallVec::from(a.rank());
            let last = b.dims() - 1;
            if r[0..last].iter().zip(&b.rank()[0..last]).any(|(a, b)| a != b) {
                return err!(Domain);
            }
            r[last] += b.rank()[last];

            let v = with_eco(r.iter().product(), |v| {
                for_1!(a; [l, _rk] {
                    v.extend(l);
                    v.extend(b.list());
                });
                Ok(())
            })?;
            return Ok(Array::new(v, a.dims(), r));
        } else if a.dims() <= o.rd[0] as usize && o.rd[1] == 1 {
            let mut r = SmallVec::from(b.rank());
            let last = a.dims() - 1;
            if r[0..last].iter().zip(&a.rank()[0..last]).any(|(a, b)| a != b) {
                return err!(Domain);
            }
            r[last] += a.rank()[last];

            let v = with_eco(r.iter().product(), |v| {
                for_1!(b; [l, _rk] {
                    v.extend(a.list());
                    v.extend(l);
                });
                Ok(())
            })?;
            return Ok(Array::new(v, b.dims(), r));
        }
    }

    let ad = usize::min(a.dims(), o.rd[0] as usize);
    let bd = usize::min(b.dims(), o.rd[1] as usize);
    let counter = usize::max(
        a.rank()[ad..].iter().chain(&[1]).product(),
        b.rank()[bd..].iter().chain(&[1]).product(),
    );
    let size_a: usize = a.rank()[0..ad].iter().chain(&[1]).product();
    let size_b: usize = b.rank()[0..bd].iter().chain(&[1]).product();
    // Sanity check
    if ad != bd
        && (ad > bd && (ad - bd > 1 || b.rank()[0..bd].iter().zip(a.rank()).any(|(a, b)| *a != *b))
            || bd > ad
                && (bd - ad > 1 || a.rank()[0..ad].iter().zip(b.rank()).any(|(a, b)| *a != *b)))
    {
        std::hint::cold_path();
        return err!(Domain);
    }

    let v = with_eco(counter * (size_a + size_b), |v| {
        for_n2!(ad, a, bd, b; [(l, lk), (r, rk)] {
            if lk.is_empty() && rk.len() > 1 || rk.is_empty() && lk.len() > 1 {
                std::hint::cold_path();
                return err!(Domain);
            }
            v.extend(l);
            v.extend(r);
        });
        Ok(())
    })?;

    let ldim = usize::max(ad, bd);
    let r = if a.dims() < b.dims() {
        b.rank()
    } else if a.dims() == b.dims() {
        if a.len() > b.len() { a.rank() } else { b.rank() }
    } else {
        a.rank()
    };
    let mut rank: SmallVec<[usize; INLINE_SIZE]> = smallvec![];

    if ad == 0 || bd == 0 {
        rank.push(2);
    }
    if ad == 0 && bd == 0 {
        rank.extend(r.iter().copied());
    } else {
        for i in 0..usize::max(ldim, r.len()) {
            if i + 1 == ldim {
                rank.push(a.rank().get(i).unwrap_or(&1) + b.rank().get(i).unwrap_or(&1));
            } else {
                rank.push(*r.get(i).unwrap_or(&1));
            }
        }
    }

    Ok(Array::new(v, rank.len(), rank))
}
