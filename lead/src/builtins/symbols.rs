#![allow(unused_variables)]

use std::collections::{HashMap, HashSet};
use std::ops::*;

use crate::function::Opts;
use crate::types::Value;
use crate::types::list::List;
use crate::types::scalar;
use crate::utils::error::{Error, Status, err};
use crate::utils::function::*;
use crate::utils::symbols::{for_0, for_1, for_n2, pad_rank, scalar_op};
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
    /// `=`
    Equal,
    /// `~`
    Tilde,
    /// `,`
    Comma,
    /// `;`
    Semicolon,
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
pub(crate) fn boxed(a: Value, o: &Opts) -> Status<Value> {
    Ok(List::scalar(Scalar::boxed_l(&o.a, a)?).into())
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
pub(crate) fn unbox(a: Value, o: &Opts) -> Status<Value> {
    let a = a.to_list()?;
    if a.is_scalar() {
        let f = &a.list()[0];
        if f.is_box() { unsafe { f.unbox_unchecked() }.to_value() } else { Ok(a.into()) }
    } else if a.is_empty() {
        std::hint::cold_path();
        Ok(a.into())
    } else {
        Ok(List::scalar(a.list()[0]).into())
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
pub(super) const fn identity(a: Value, _: &Opts) -> Status<Value> {
    Ok(a)
}

// lbracketd | [ | 99, 99 | Left
#[inline(always)]
pub(super) fn left(a: Value, b: Value, _: &Opts) -> Status<Value> {
    Ok(a)
}

// rbracketd | ] | 99, 99 | Right
#[inline(always)]
pub(super) fn right(a: Value, b: Value, _: &Opts) -> Status<Value> {
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

// equalm | = | 2 / 99 | Transpose / Keys
pub(super) fn transpose_keys(a: Value, o: &Opts) -> Status<Value> {
    if a.is_empty() {
        return Ok(a);
    }

    if let Value::Map(a) = a {
        return a.keys(&o.a).map(Value::List);
    }

    let a = unsafe { a.to_list_unchecked() };
    let dims = a.dims();
    let leading = usize::min(dims, o.rm as usize);
    if leading < 1 {
        return err!(Domain);
    }

    let rank = a.rank().to_vec();
    if dims == 1 {
        return a.with_rank(&[1, rank[0]]).map(Value::List);
    } else if leading == 1 {
        return List::unranked(
            &o.a,
            dims,
            &[],
            a.ranked(leading)?
                .into_iter()
                .map(|v| v.with_rank(&[1, rank[0]]))
                .collect::<Status<Vec<List>>>()?,
        )
        .map(Value::List);
    } else if leading == 0 {
        return Ok(Value::List(a));
    }

    apply_loosem(&o.a, a, leading as u8, |a| {
        assert!(a.dims() >= 2, "INTERNAL: Something went wrong.");
        let leading = a.dims();
        let mut rank = SmallVec::from(a.rank());
        rank.swap(leading - 2, leading - 1);
        if (rank[leading - 2] == 1) != (rank[leading - 1] == 1) {
            return a.with_rank(&rank);
        }

        let trailing = a.ranked(leading - 1)?;
        List::mat(
            trailing[0]
                .list()
                .iter()
                .zip(0..)
                .flat_map(|(_, i)| trailing.iter().map(move |v| v.list()[i]))
                .collect::<Vec<Scalar>>(),
            leading,
            rank.into(),
        )
    })
    .map(Value::List)
}

// equald | = | 0 | Equals
#[inline(always)]
pub(super) fn equal(mut a: List, mut b: List, o: &Opts) -> Status<List> {
    if o.rd[0] == 0 && o.rd[1] == 0 {
        scalar_op! {
            a, b, [l, r] =
            { Scalar::Boolean(*l == *r) };
            { Scalar::Boolean(l == *r)  };
            { Scalar::Boolean(*l == r)  };
        }
    } else {
        std::hint::cold_path();
        err!(Internal, "TODO: Non rank [0, 0] for dyadic '='.")
    }
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
                .to_instant()?
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

// commam | , | 99 | Ravel / Values
pub(crate) fn ravel(a: Value, o: &Opts) -> Status<Value> {
    if a.is_empty() {
        return Ok(a);
    }
    if let Value::Map(m) = a {
        return m.values().map(Value::List);
    }

    let mut a = unsafe { a.to_list_unchecked() };
    if a.is_scalar() {
        return Ok(Value::List(a));
    }
    a.ds = 1;
    a.rk = smallvec![a.len()];
    Ok(Value::List(a))
}

// commad | , | 1, 1 | Concatenate
pub(crate) fn concat(mut a: List, mut b: List, o: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(b);
    } else if b.is_empty() {
        return Ok(a);
    }

    let boxit = a.list()[0].repr() != b.list()[0].repr();
    let box_a = boxit && b.list()[0].repr() == 3 && a.list()[0].repr() != 3;
    let box_b = boxit && a.list()[0].repr() == 3 && b.list()[0].repr() != 3;

    if box_a {
        for_0!(a; [l] = Scalar::boxed(&o.a, List::scalar(*l))?);
    }
    if box_b {
        for_0!(b; [l] = Scalar::boxed(&o.a, List::scalar(*l))?);
    }

    if o.rd[0] == 1 && o.rd[1] == 1 {
        if a.dims() < 2 && b.dims() < 2 {
            let mut v = Vec::with_capacity(a.len() + b.len());
            v.extend(a.list());
            v.extend(b.list());
            return List::from_vec(v);
        } else if b.dims() < 2 {
            let mut rank = SmallVec::from(a.rank());
            rank[0] += b.len();
            let mut build = Vec::with_capacity(rank.iter().product());
            for_1!(a; [l, rk] {
                build.extend(l);
                build.extend(b.list());
            });
            return List::mat(build, a.dims(), rank);
        } else if a.dims() < 2 {
            let mut rank = SmallVec::from(b.rank());
            rank[0] += a.len();
            let mut build = Vec::with_capacity(rank.iter().product());
            for_1!(b; [r, rk] {
                build.extend(a.list());
                build.extend(r);
            });
            return List::mat(build, b.dims(), rank);
        }
    } else if a.dims() == b.dims() {
        if a.dims() <= o.rd[0] as usize && b.dims() <= o.rd[1] as usize {
            let mut r = SmallVec::from(a.rank());
            let last = r.len() - 1;
            if r[0..last].iter().zip(&b.rank()[0..last]).any(|(a, b)| a != b) {
                return err!(Domain);
            }
            r[last] += b.rank()[last];

            let mut v = Vec::with_capacity(a.len() + b.len());
            v.extend(a.list());
            v.extend(b.list());
            return List::mat(v, r.len(), r);
        } else if b.dims() <= o.rd[1] as usize {
            let mut r = SmallVec::from(a.rank());
            let last = b.dims() - 1;
            if r[0..last].iter().zip(&b.rank()[0..last]).any(|(a, b)| a != b) {
                return err!(Domain);
            }
            r[last] += b.rank()[last];

            let mut v = Vec::with_capacity(r.iter().product());
            for_1!(a; [l, rk] {
                v.extend(l);
                v.extend(b.list());
            });
            return List::mat(v, a.dims(), r);
        } else if a.dims() <= o.rd[0] as usize {
            let mut r = SmallVec::from(b.rank());
            let last = a.dims() - 1;
            if r[0..last].iter().zip(&a.rank()[0..last]).any(|(a, b)| a != b) {
                return err!(Domain);
            }
            r[last] += a.rank()[last];

            let mut v = Vec::with_capacity(r.iter().product());
            for_1!(b; [r, rk] {
                v.extend(a.list());
                v.extend(r);
            });
            return List::mat(v, b.dims(), r);
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

    let mut v: Vec<Scalar> = Vec::with_capacity(counter * (size_a + size_b));
    for_n2!(ad, a, bd, b; [(l, lk), (r, rk)] {
        if lk.is_empty() && rk.len() > 1 || rk.is_empty() && lk.len() > 1 {
            std::hint::cold_path();
            return err!(Domain);
        }
        v.extend(l);
        v.extend(r);
    });

    let ldim = usize::max(ad, bd);
    let r = if a.dims() < b.dims() { b.rank() } else { a.rank() };
    let mut rank: SmallVec<[usize; 4]> = smallvec![];
    if ad == 0 && bd == 0 {
        rank.push(2);
        rank.extend(r.iter().cloned());
    } else {
        for i in 0..usize::max(ldim, r.len()) {
            if i + 1 == ldim {
                rank.push(a.rank().get(i).unwrap_or(&1) + b.rank().get(i).unwrap_or(&1));
            } else {
                rank.push(*r.get(i).unwrap_or(&1));
            }
        }
    }

    List::mat(v, rank.len(), rank)
}

// semicolonm | ; | 1 | Mold
pub(crate) fn mold(a: List, _: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(a);
    } else if !a.list()[0].is_box() {
        return Ok(a);
    }

    let lists = a
        .list()
        .iter()
        .map(|b| unsafe { b.as_unboxed_unchecked() }.as_list())
        .collect::<Status<Vec<&List>>>()?;

    let mut rank: SmallVec<[usize; 4]> = SmallVec::new();
    for list in &lists {
        for i in 0..list.dims() {
            if i == rank.len() {
                rank.push(1);
            }
            rank[i] = usize::max(rank[i], list.rank()[i]);
        }
    }

    if lists.iter().any(|v| v.list()[0].is_box()) {
        return err!(ValueS, "No fill character exists for boxes");
    }
    let flattened: Vec<Scalar> =
        lists.iter().map(|v| pad_rank((v.list(), v.rank()), &rank).unwrap()).flatten().collect();
    rank.push(flattened.len());
    List::mat(flattened, rank.len(), rank)
}

// semicolond | ; | 99, 99 | Table
pub(crate) fn table(a: List, b: List, o: &Opts) -> Status<List> {
    if a.is_empty() {
        return Ok(b);
    } else if b.is_empty() {
        return Ok(a);
    }

    if a.list()[0].repr() != b.list()[0].repr() {
        return List::from_vec(vec![Scalar::boxed(&o.a, a)?, Scalar::boxed(&o.a, b)?]);
    } else if a.rank() == b.rank() {
        let mut v = Vec::with_capacity(a.len() + b.len());
        v.extend(a.list());
        v.extend(b.list());
        let mut r = SmallVec::from(a.rank());
        r.push(2);
        return List::mat(v, a.dims() + 1, r);
    }

    let dims = usize::max(a.dims(), b.dims());
    let mut rank = SmallVec::with_capacity(dims);
    for i in 0..dims {
        rank.push(usize::max(*a.rank().get(i).unwrap_or(&1), *b.rank().get(i).unwrap_or(&1)));
    }

    let size: usize = rank.iter().product();
    let mut v = Vec::with_capacity(size * 2);
    v.extend(pad_rank((a.list(), a.rank()), &rank).unwrap());
    v.extend(pad_rank((b.list(), b.rank()), &rank).unwrap());
    rank.push(2);
    List::mat(v, rank.len(), rank)
}
