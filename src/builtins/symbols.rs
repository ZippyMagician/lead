#![allow(unused_variables)]

use std::collections::{HashMap, HashSet};
use std::ops::*;

use crate::function::Opts;
use crate::list::List;
use crate::scalar;
use crate::utils::error::{Error, Status, err};
use crate::utils::function::*;
use crate::utils::symbols::{for_0, for_1, scalar_op};
use scalar::Scalar::{self, Boolean, Char, Float, Number};

use num_traits::AsPrimitive;
use ordered_float::{Float as OrderFloat, OrderedFloat};
use smallvec::{SmallVec, smallvec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Symbol {
    /// `+`
    Plus,
    /// `-`
    Dash,
    /// `*`
    Star,
    /// `%`
    Percent,
    /// `|`
    Pipe,
    /// `!`
    Exclamation,
    /// `<`
    LeftArrow,
    /// `>`
    RightArrow,
    /// `^`
    Caret,
    /// `$`
    Dollar,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `?`
    Question,
    /// `~`
    Tilde,
}

// plusm | + | 0 | Absolute value / Cast to int
pub(super) fn abs(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Number(n) => Number(n.abs()),
        Float(f) => Float(f.abs()),
        Boolean(n) => Number(*n as i64),
        Char(c) => Number(*c as u32 as i64),
        Scalar::Box(ptr) => Number(ptr.addr() as i64),
    });
    Ok(a)
}

// plusd | + | 0, 0 | Addition
pub(super) fn add(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(b);
    } else if b.is_empty() {
        return Ok(a);
    } else if matches!(a.list()[0], Scalar::Char(_)) || matches!(b.list()[0], Scalar::Char(_)) {
        scalar_op!(a, b,
            [l, r] = { Scalar::Char(char::from_u32(
                l.to_num()? as u32 + r.to_num()? as u32
            ).ok_or(Error::NaN)?) };
            { Scalar::Char(char::from_u32(
                l.to_num()? as u32 + r.to_num()? as u32
            ).ok_or(Error::NaN)?) };
            { Scalar::Char(char::from_u32(
                l.to_num()? as u32 + r.to_num()? as u32
            ).ok_or(Error::NaN)?) };
        )
    } else {
        scalar_op!(a, b, add)
    }
}

// dashm | - | 0 | Negate / Swap case / Boolean not
pub(super) fn negate(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char(c) if c.is_lowercase() => Char(c.to_ascii_uppercase()),
        Char(c) => Char(c.to_ascii_lowercase()),
        Number(n) => Number(-*n),
        Float(n) => Float(-*n),
        Boolean(b) => Boolean(!*b),
        Scalar::Box(ptr) => return err!(Box),
    });
    Ok(a)
}

// dashd | - | 0, 0 | Subtraction
pub(super) fn sub(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(b);
    } else if b.is_empty() {
        return Ok(a);
    } else if matches!(a.list()[0], Scalar::Char(_)) || matches!(b.list()[0], Scalar::Char(_)) {
        scalar_op!(a, b,
            [l, r] = { Scalar::Char(char::from_u32(
                l.to_num()? as u32 - r.to_num()? as u32
            ).ok_or(Error::NaN)?) };
            { Scalar::Char(char::from_u32(
                l.to_num()? as u32 - r.to_num()? as u32
            ).ok_or(Error::NaN)?) };
            { Scalar::Char(char::from_u32(
                l.to_num()? as u32 - r.to_num()? as u32
            ).ok_or(Error::NaN)?) };
        )
    } else {
        scalar_op!(a, b, sub)
    }
}

// starm | * | 0 | Sign / Case
pub(super) fn signum(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char(c) if c.is_lowercase() => Number(-1),
        Char(c) if c.is_uppercase() => Number(1),
        Char(_) => Number(0),
        Number(n) => Number(n.signum()),
        Float(n) => Number(n.signum().as_()),
        Boolean(b) if *b => Number(1),
        Boolean(_) => Number(0),
        Scalar::Box(ptr) => Number(0),
    });
    Ok(a)
}

// stard | * | 0, 0 | Multiplication / Boolean And
pub(super) fn mul(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(b);
    } else if b.is_empty() {
        return Ok(a);
    } else if matches!(a.list()[0], Scalar::Boolean(_)) && matches!(b.list()[0], Scalar::Boolean(_))
    {
        scalar_op!(a, b,
            [l, r] = { Scalar::Boolean(
                unsafe { l.to_bool_unchecked() && r.to_bool_unchecked() }
            ) };
            { Scalar::Boolean(
                unsafe { l.to_bool_unchecked() && r.to_bool_unchecked() }
            ) };
            { Scalar::Boolean(
                unsafe { l.to_bool_unchecked() && r.to_bool_unchecked() }
            ) };
        )
    } else {
        scalar_op!(a, b, mul)
    }
}

// percentm | % | 0 | Reciprocal
pub(super) fn reciprocal(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char('\0') | Number(0) | Float(OrderedFloat(0.)) | Boolean(false) => {
            std::hint::cold_path();
            return err!(NaN);
        }
        Char(c) => Float(OrderedFloat(*c as u32 as f64).recip()),
        Number(n) => Float(OrderedFloat(*n as f64).recip()),
        Float(f) => Float(f.recip()),
        Boolean(_) => Float(OrderedFloat(1.0)),
        _ => return err!(NaN),
    });
    Ok(a)
}

// percentd | % | 0, 0 | Division
pub(super) fn div(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, div)
}

// pipem | | | 0 | Boolean not
pub(super) fn not(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = match v {
        Char(c) if *c == '\0' => Boolean(true),
        Number(n) if *n == 0 => Boolean(true),
        Float(f) if *f == 0. => Boolean(true),
        Boolean(b) => Boolean(!*b),
        Scalar::Box(_) => return err!(Box),
        _ => Boolean(false),
    });
    Ok(a)
}

// piped | | | 0, 0 | Residue (Modulus) / Boolean Or
pub(super) fn residue(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(b);
    } else if b.is_empty() {
        return Ok(a);
    } else if matches!(a.list()[0], Scalar::Boolean(_)) && matches!(b.list()[0], Scalar::Boolean(_))
    {
        scalar_op!(a, b,
            [l, r] = { Scalar::Boolean(
                unsafe { l.to_bool_unchecked() || r.to_bool_unchecked() }
            ) };
            { Scalar::Boolean(
                unsafe { l.to_bool_unchecked() || r.to_bool_unchecked() }
            ) };
            { Scalar::Boolean(
                unsafe { l.to_bool_unchecked() || r.to_bool_unchecked() }
            ) };
        )
    } else {
        scalar_op!(b, a, rem)
    }
}

// exclamationm | ! | 0 | Factorial
pub(super) fn factorial(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = {
        let n = v.to_num()?;
        if n < 0 {
            return err!(Value);
        } else if n == 0 {
            Scalar::Number(1)
        } else {
            Scalar::Number((1..n).product())
        }
    });
    Ok(a)
}

// exclamationd | ! | 0, 0 | Bitwise OR
pub(super) fn bitor(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, [l, r] = { Scalar::Number(l.to_num()? | r.to_num()?) })
}

// leftarrowm | < | 99 | Box
pub(crate) fn boxed(a: List, o: &Opts) -> Status<List> {
    Ok(List::scalar(Scalar::boxed(&o.a, a)?))
}

// leftarrowd | < | 0, 0 | Less than
pub(crate) fn lt(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op! {
        a, b, [l, r] =
        { Scalar::Boolean(*l < *r) };
        { Scalar::Boolean(l < *r)  };
        { Scalar::Boolean(*l < r)  };
    }
}

// rightarrowm | > | 99 | Unbox / First
pub(crate) fn unbox(a: List, o: &Opts) -> Status<List> {
    if a.is_scalar() {
        let f = &a.list()[0];
        if f.is_box() {
            unsafe { f.unbox_unchecked() }.to_list()
        } else {
            Ok(a)
        }
    } else if a.is_empty() {
        std::hint::cold_path();
        Ok(a)
    } else {
        Ok(List::scalar(a.list()[0]))
    }
}

// rightarrowd | > | 0, 0 | Greater than
pub(crate) fn gt(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op! {
        a, b, [l, r] =
        { Scalar::Boolean(*l > *r) };
        { Scalar::Boolean(l > *r) };
        { Scalar::Boolean(*l < r) };
    }
}

// caretm | ^ | 0 | Exponential
pub(super) fn exp(mut a: List, _: &Opts) -> Status<List> {
    for_0!(a; [v] = Float(v.to_float()?.exp()));
    Ok(a)
}

// caretd | ^ | 0, 0 | Bitwise AND
pub(super) fn bitand(mut a: List, mut b: List, _: &Opts) -> Status<List> {
    scalar_op!(a, b, [l, r] = { Scalar::Number(l.to_num()? & r.to_num()?) })
}

// dollarm | $ | 99 | Shape
pub(crate) fn shape(a: List, _: &Opts) -> Status<List> {
    List::from_vec(a.rank().iter().map(|n| Scalar::Number(*n as i64)).collect())
}

// dollard | $ | 1, 99 | Reshape
pub(crate) fn reshape(a: List, b: List, o: &Opts) -> Status<List> {
    apply_loosed(&o.a, a, b, o.rd, |a, b| {
        let rank = a
            .list()
            .iter()
            .map(|n| n.to_num().map(|n| n as usize))
            .collect::<Status<SmallVec<[usize; 4]>>>()?;
        b.with_rank(&rank)
    })
}

// lbracketm / rbracketm | [ / ] | 99 | Identity
pub(super) const fn identity(a: List, _: &Opts) -> Status<List> {
    Ok(a)
}

// lbracketd | [ | 99, 99 | Left
#[inline(always)]
pub(super) fn left(a: List, b: List, _: &Opts) -> Status<List> {
    Ok(a)
}

// rbracketd | ] | 99, 99 | Right
#[inline(always)]
pub(super) fn right(a: List, b: List, _: &Opts) -> Status<List> {
    Ok(b)
}

// questionm | ? | 99 | Dedup sieve
pub(super) fn dedup_sieve(a: List, o: &Opts) -> Status<List> {
    if o.rm == 0 {
        let mut m = HashSet::new();
        let mut new = Vec::with_capacity(a.len());
        for s in a.list() {
            if m.contains(s) {
                continue;
            }
            m.insert(*s);
            new.push(*s);
        }
        List::from_vec(new)
    } else {
        let rank: SmallVec<[usize; 4]> = SmallVec::from(a.rank());
        let ldims = usize::min(o.rm as usize, a.dims() - 1);
        let runs = a.ranked(ldims)?;
        let mut m = HashSet::new();
        let mut new = Vec::with_capacity(runs.len());
        for run in runs.iter() {
            if m.contains(run) {
                continue;
            }
            m.insert(run);
            new.push(run.clone());
        }

        List::unranked(&o.a, rank.len(), &rank[..ldims], new)
    }
}

// questiond | ? | 99, 1 | Group
pub(super) fn group(a: List, b: List, o: &Opts) -> Status<List> {
    if b.is_empty() {
        return err!(Domain);
    }
    if a.is_empty() {
        return Ok(a);
    }

    let ldims = usize::min(o.rm as usize, a.dims() - 1);
    let rank: SmallVec<[usize; 4]> = SmallVec::from(a.rank());
    let dims = a.dims();
    let runs = a.ranked(ldims)?;

    let mut build: Vec<List> = Vec::with_capacity(b.len() / b.rank()[0]);
    for_1!(b; [b, r] {
        if rank[ldims] != b.len() {
            return err!(Domain);
        }
        let mut map: HashMap<usize, Vec<&[Scalar]>> = HashMap::new();
        let bucket = b.into_iter().map(|n| n.to_num().map(|n| n as usize))
            .collect::<Status<Vec<usize>>>()?;
        for i in 0..=*bucket.iter().max().unwrap() {
            map.insert(i, vec![]);
        }
        for (run, i) in runs.iter().zip(0..) {
            map.get_mut(&bucket[i]).unwrap().push(run.list());
        }
        let mut construct = vec![List::empty(); map.len()];
        for (key, value) in map.into_iter() {
            if !value.is_empty() {
                construct[key] = List::unranked(&o.a, ldims, &rank[..ldims],
                    value.into_iter().map(|v| List::mat_ref(v, ldims, &rank[..ldims]))
                        .collect::<Status<Vec<List>>>()?
                )?
            };
        }
        build.push(List::from_vec(construct.into_iter().map(
            |n| Scalar::boxed(&o.a, n)
        ).collect::<Status<Vec<Scalar>>>()?)?);
    });

    List::unranked(&o.a, dims, &rank, build)
}

// tildem | ~ | 0 | 1-range / Rising alphabet
pub(super) fn range(mut a: List, o: &Opts) -> Status<List> {
    if a.is_scalar() {
        List::range(Scalar::Number(1), a.list()[0])
    } else {
        for_0!(a; [v] = Scalar::boxed(
            &o.a, List::range(Scalar::Number(1), *v)?)?
        );
        Ok(a)
    }
}

// tilded | ~ | 99, 0 | Index
pub(super) fn index(a: List, b: List, o: &Opts) -> Status<List> {
    apply_loosed(&o.a, a, b, o.rd, |a, b| {
        if a.is_empty() {
            std::hint::cold_path();
            return err!(Domain);
        }

        let index = b.list()[0];
        if !index.is_box() {
            let i = index.to_num()? as usize;
            let rank: SmallVec<[usize; 4]> = if a.dims() == 0 {
                smallvec![1]
            } else {
                a.rank().iter().copied().take(a.dims() - 1).collect()
            };
            let r: usize = rank.iter().copied().product();
            if i >= unsafe { *a.rank().last().unwrap_unchecked() } {
                return err!(Domain);
            }
            let values = a.list()[i * r..i * r + r].to_vec();
            List::mat(values, a.dims() - 1, rank)
        } else {
            let index = unsafe { index.unbox_unchecked() }
                .to_list()?
                .list()
                .iter()
                .map(|n| n.to_num().map(|n| n as usize))
                .collect::<Status<Vec<usize>>>()?;

            if a.dims() < index.len() {
                return err!(Domain);
            }
            let rank = a.rank();

            let mut i = 0;
            let mut prefix = 1;
            for j in 0..index.len() {
                i += index[j] * prefix;
                prefix *= rank[rank.len() - j - 1];
            }

            let rank: SmallVec<[usize; 4]> =
                rank.iter().copied().take(a.dims() - index.len()).collect();
            let ct: usize = rank.iter().copied().product();
            if ct == 0 {
                Ok(List::scalar(a.list()[i]))
            } else {
                List::mat(a.list()[i..i + ct].to_vec(), a.dims() - index.len(), rank)
            }
        }
    })
}
