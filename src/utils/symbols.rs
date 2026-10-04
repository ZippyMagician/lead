macro_rules! foreach_0 {
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

pub(crate) use foreach_0;

macro_rules! scalar_op {
    ($a:ident, $b:ident, $d:ident) => {
        if $a.len() == $b.len() {
            let values = $a.list_mut();
            for (l, r) in values.iter_mut().zip($b.list()) {
                *l = l.$d(*r);
            }
            Ok($a)
        } else if $a.is_empty() {
            Ok($b)
        } else if $b.is_empty() {
            Ok($a)
        } else if $a.is_scalar() {
            let a = $a.list()[0];
            foreach_0!($b; [r] = r.$d(a));
            Ok($b)
        } else if $b.is_scalar() {
            let b = $b.list()[0];
            foreach_0!($a; [l] = l.$d(b));
            Ok($a)
        } else if $a.len() % $b.len() == 0 || $b.len() % $a.len() == 0 {
            let m;
            let o;
            if $a.len() > $b.len() {
                m = $a.list_mut();
                o = $b.list();
            } else {
                m = $b.list_mut();
                o = $a.list();
            }

            for (l, r) in m.iter_mut().zip(o.iter().cycle()) {
                *l = l.$d(*r);
            }

            Ok(if $a.len() > $b.len() { $a } else { $b })
        } else {
            err!(Domain)
        }
    };
}

pub(crate) use scalar_op;
