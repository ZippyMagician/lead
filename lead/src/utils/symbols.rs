use super::error::Status;
use crate::types::scalar::Scalar;

use smallvec::{SmallVec, smallvec};

macro_rules! for_0 {
    // NOTE: For some reason beyond my understanding, there are cases where
    // this is slower than writing it out manually. One such instance is when
    // the expression, `$b`, reuses a scalar value over and over again.. but
    // only in some such cases.. For instance, in a mathematical operation,
    // the scalar value being used on the LHS is much slower than if you manually
    // zip a cycled iter over said value to the main iteration component. I have
    // no fucking clue why. It's pretty significant too, scalar operations become
    // ~50% slower when a scalar number is on the LHS. Fucking programming, I guess.
    ($a:ident; [$v:ident] = $b:expr) => {{
        for $v in $a.list_mut() {
            *$v = $b;
        }
    }};

    ($a:ident; [$v:ident] $b:expr) => {{
        for $v in $a.list().iter() {
            $b;
        }
    }};
}

macro_rules! for_1 {
    ($a:ident; [$v:ident, $r:ident] $b:expr) => {
        '_for_1: {
            if $a.is_empty() {
                break '_for_1;
            }

            let lists = $a.list();
            let size = $a.rank()[0];
            unsafe { ::std::hint::assert_unchecked(lists.len() % size == 0) };
            for i in 0..lists.len() / size {
                let $v = &lists[i * size..i * size + size];
                let $r = size;
                {
                    $b
                };
            }
        }
    };
}

macro_rules! for_n {
    ($a:ident, $n:expr; [($v:ident, $r:ident)] $b:expr) => {
        '_for_n: {
            if $a.is_empty() {
                break '_for_n;
            }

            let lists = $a.list();
            let axis = usize::min($a.dims(), $n);
            let $r = &$a.rank()[0..axis];
            let size: usize = $r.iter().product();
            unsafe { ::std::hint::assert_unchecked(lists.len() % size == 0) };
            for $v in lists.chunks_exact(size) {
                {
                    $b
                };
            }
        }
    };
}

macro_rules! for_n2 {
    ($n1:expr, $a:ident, $n2:expr, $b:ident; [($l:ident, $lr:ident), ($r:ident, $rr:ident)] $bl:expr) => {
        '_for_n2: {
            if $a.is_empty() || $b.is_empty() {
                break '_for_n2;
            }

            let size_a = $a.rank()[0..$n1].iter().product();
            let tail_a = $a.rank()[$n1..].iter().product();
            let size_b = $b.rank()[0..$n2].iter().product();
            let tail_b = $b.rank()[$n2..].iter().product();

            if tail_a == tail_b {
                let $lr = &$a.rank()[0..$n1];
                let $rr = &$b.rank()[0..$n2];
                for i in 0..tail_a {
                    let $l = &$a.list()[i * size_a..i * size_a + size_a];
                    let $r = &$b.list()[i * size_b..i * size_b + size_b];
                    {
                        $bl
                    };
                }
            } else if tail_a == 0 {
                let $l = $a.list();
                let $lr = $a.rank();
                let $rr = &$b.rank()[0..$n2];
                for i in 0..tail_b {
                    let $r = &$b.list()[i * size_b..i * size_b + size_b];
                    {
                        $bl
                    };
                }
            } else if tail_b == 0 {
                let $r = $b.list();
                let $lr = &$a.rank()[0..$n1];
                let $rr = $b.rank();
                for i in 0..tail_a {
                    let $l = &$a.list()[i * size_a..i * size_a + size_a];
                    {
                        $bl
                    };
                }
            } else if tail_a % tail_b == 0 {
                let mut bucket = $b.list().chunks(size_b).cycle();
                let $lr = &$a.rank()[0..$n1];
                let $rr = &$b.rank()[0..$n2];
                for i in 0..tail_a {
                    let $l = &$a.list()[i * size_a..i * size_a + size_a];
                    let $r = bucket.next().unwrap();
                    {
                        $bl
                    };
                }
            } else if tail_b % tail_a == 0 {
                let mut bucket = $a.list().chunks(size_a).cycle();
                let $lr = &$a.rank()[0..$n1];
                let $rr = &$b.rank()[0..$n2];
                for i in 0..tail_b {
                    let $r = &$b.list()[i * size_b..i * size_b + size_b];
                    let $l = bucket.next().unwrap();
                    {
                        $bl
                    };
                }
            } else {
                return $crate::err!(Domain);
            }
        }
    };
}

pub(crate) use for_0;
pub(crate) use for_1;
pub(crate) use for_n;
pub(crate) use for_n2;

macro_rules! scalar_op {
    ($a:ident, $b:ident, $d:ident) => {
        scalar_op!($a, $b,
            [l, r] = { l.$d(*r)? }; { r.$d(l)? }; { l.$d(r)? };
        )
    };

    ($a:ident, $b:ident, [$l:ident, $r:ident] = $bl:block) => {
        scalar_op!($a, $b, [$l, $r] = $bl; $bl; $bl;)
    };

    // e1: L is &mut S, R is &S.
    // e2: L is S, R is &mut S
    // e3: L is &mut S, R is S
    ($a:ident, $b:ident, [$l:ident, $r:ident] = $e1:block; $e2:block; $e3:block;) => {
        if $a.len() == $b.len() {
            let values = $a.list_mut();
            for ($l, $r) in values.iter_mut().zip($b.list()) {
                *$l = $e1;
            }
            Ok($a)
        } else if $a.is_empty() {
            Ok($b)
        } else if $b.is_empty() {
            Ok($a)
        } else if $a.is_scalar() {
            // And now it's fucking slower?? what the fuck
            /*for (l, r) in $b.list_mut().iter_mut().zip(
                $a.list().iter().cycle()
            ) {
                *l = r.$d(*l)?;
            }*/
            let $l = $a.list()[0];
            for_0!($b; [$r] = $e2);
            Ok($b)
        } else if $b.is_scalar() {
            let $r = $b.list()[0];
            for_0!($a; [$l] = $e3);
            Ok($a)
        } else if $a.len() % $b.len() == 0 {
            let m = $a.list_mut();
            let o = $b.list();
            for ($l, $r) in m.iter_mut().zip(o.iter().cycle()) {
                *$l = $e1;
            }
            Ok($a)
        } else if $b.len() % $a.len() == 0 {
            let m = $b.list_mut();
            let o = $a.list();
            for ($r, $l) in m.iter_mut().zip(o.iter().cycle()) {
                *$r = $e1;
            }
            Ok($b)
        } else {
            err!(Domain)
        }

    }
}

pub(crate) use scalar_op;

pub(crate) fn pad_rank((v, vr): (&[Scalar], &[usize]), r: &[usize]) -> Status<Vec<Scalar>> {
    if vr == r {
        return Ok(v.to_vec());
    }

    let rank: SmallVec<[usize; 4]> =
        r.iter().enumerate().map(|(i, r)| usize::max(*r, *vr.get(i).unwrap_or(&1))).collect();

    // Could probably do this with scan, but can't be bothered refactoring
    // took me long enough to get this working when I wrote the typescript
    // interepreter anyways
    let mut off: SmallVec<[usize; 4]> = smallvec![1; rank.len()];
    let mut noff: SmallVec<[usize; 4]> = smallvec![1; rank.len()];
    for i in 1..rank.len() {
        off[i] = off[i - 1] * vr.get(i - 1).unwrap_or(&1);
        noff[i] = noff[i - 1] * rank[i - 1];
    }

    let nlen: usize = r.iter().product();
    let mut b = vec![v[0].get_fill()?; nlen];
    for i in 0..v.len() {
        let mut j = 0;
        for k in 0..rank.len() {
            j += i / off[k] % vr.get(k).unwrap_or(&1) * noff[k];
        }
        b[j] = v[i];
    }

    Ok(b)
}
