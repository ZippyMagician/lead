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

pub(crate) use for_0;

macro_rules! scalar_op {
    ($a:ident, $b:ident, $d:ident) => {
        /*if $a.len() == $b.len() {
            let values = $a.list_mut();
            for (l, r) in values.iter_mut().zip($b.list()) {
                *l = l.$d(*r)?;
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
            let a = $a.list()[0];
            for_0!($b; [r] = r.$d(a)?);
            Ok($b)
        } else if $b.is_scalar() {
            let b = $b.list()[0];
            for_0!($a; [l] = l.$d(b)?);
            Ok($a)
        } else if $a.len() % $b.len() == 0 {
            let m = $a.list_mut();
            let o = $b.list();
            for (l, r) in m.iter_mut().zip(o.iter().cycle()) {
                *l = l.$d(*r)?;
            }
            Ok($a)
        } else if $b.len() % $a.len() == 0 {
            let m = $b.list_mut();
            let o = $a.list();
            for (l, r) in m.iter_mut().zip(o.iter().cycle()) {
                *l = r.$d(*l)?;
            }
            Ok($b)
        } else {
            err!(Domain)
        }*/
        scalar_op!($a, $b, 
            [l, r] = { l.$d(*r)? }; { r.$d(l)? }; { l.$d(r)? };
        )
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
