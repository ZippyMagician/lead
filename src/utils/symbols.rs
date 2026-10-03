macro_rules! foreach_0 {
    (map; $a:ident; [$v:ident] $b:expr) => {{
        let values = $a.list_mut();
        for $v in values.iter_mut() {
            *$v = $b;
        }
        return Ok($a);
    }};

    ($a:ident; [$v:ident] $b:expr) => {{
        let values = $a.list_mut();
        for $v in values.iter_mut() {
            $b;
        }
        return Ok($a);
    }};
}

pub(crate) use foreach_0;
