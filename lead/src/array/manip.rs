use crate::function::Opts;
use crate::types::list::{Array, INLINE_SIZE};
use crate::types::scalar::Scalar;

use crate::utils::error::{Status, err};
use crate::utils::symbols::*;

use smallvec::{SmallVec, smallvec};

#[inline(always)]
pub(crate) fn concat<T: Scalar>(a: Array<T>, b: Array<T>, o: &Opts) -> Status<Array<T>> {
    if o.rd[0] == 1 && o.rd[1] == 1 {
        if a.dims() < 2 && b.dims() < 2 {
            let v = build_rc(a.len() + b.len(), |v| {
                v.extend(a.list());
                v.extend(b.list());
                Ok(())
            })?;
            return Ok(Array::new(v, 1, smallvec![a.len() + b.len()]));
        } else if b.dims() < 2 {
            let mut rank = SmallVec::from(a.rank());
            rank[0] += b.len();
            let build = build_rc(rank.iter().product(), |build| {
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
            let build = build_rc(rank.iter().product(), |build| {
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

            let v = build_rc(a.len() + b.len(), |v| {
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

            let v = build_rc(r.iter().product(), |v| {
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

            let v = build_rc(r.iter().product(), |v| {
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

    let (ca, fa) = a.rank().split_at(ad);
    let (cb, fb) = b.rank().split_at(bd);
    let counter = usize::max(fa.iter().chain(&[1]).product(), fb.iter().chain(&[1]).product());
    let size_a: usize = ca.iter().chain(&[1]).product();
    let size_b: usize = cb.iter().chain(&[1]).product();

    // Some sanity checks
    let frame = if fa == fb {
        fa
    } else if fa.len() > fb.len() && fa.starts_with(fb) {
        fa
    } else if fb.len() > fa.len() && fb.starts_with(fa) {
        fb
    } else {
        return err!(Domain);
    };

    if ad.abs_diff(bd) > 1 {
        return err!(Domain);
    }
    let r = ad.max(bd).max(1);
    let at = |c: &[usize], i: usize| c.get(i).copied().unwrap_or(1);

    if (0..r - 1).any(|i| at(ca, i) != at(cb, i)) {
        return err!(Domain);
    }

    let v = build_rc(counter * (size_a + size_b), |v| {
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

    let mut rk: SmallVec<[usize; INLINE_SIZE]> = SmallVec::new();
    rk.extend((0..r - 1).map(|i| at(ca, i)));
    rk.push(at(ca, r - 1) + at(cb, r - 1));
    rk.extend_from_slice(frame);

    Ok(Array::new(v, rk.len(), rk))
}

#[inline(always)]
pub(crate) fn table<T: Scalar>(a: Array<T>, b: Array<T>) -> Status<Array<T>> {
    if a.rank() == b.rank() {
        let v = build_rc(a.len() + b.len(), |v| {
            v.extend(a.list());
            v.extend(b.list());
            Ok(())
        })?;
        let mut r = SmallVec::from(a.rank());
        r.push(2);
        return Ok(Array::new(v, a.dims() + 1, r));
    }

    let dims = usize::max(a.dims(), b.dims());
    let mut rank = SmallVec::with_capacity(dims);
    for i in 0..dims {
        rank.push(usize::max(*a.rank().get(i).unwrap_or(&1), *b.rank().get(i).unwrap_or(&1)));
    }

    let size: usize = rank.iter().product();
    let v = build_rc(size * 2, |v| {
        v.extend_iter(pad_rank((a.list(), a.rank()), &rank).unwrap());
        v.extend_iter(pad_rank((b.list(), b.rank()), &rank).unwrap());
        Ok(())
    })?;
    rank.push(2);
    Ok(Array::new(v, rank.len(), rank))
}
