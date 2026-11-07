macro_rules! iter_ints {
    ($l:expr, |$it:ident| $body:expr) => {
        match &$l {
            List::Integers(i) => {
                let $it = i.list().iter().copied();
                $body
            }
            List::Doubles(d) => {
                let $it = d.list().iter().map(|d| d.to_num().unwrap());
                $body
            }
            List::Booleans(b) => {
                let $it = b.list().iter().map(|b| b.to_num().unwrap());
                $body
            }
            List::String(s) => {
                let $it = s.list().iter().map(|c| c.to_num().unwrap());
                $body
            }
            List::Boxes(_) => {
                return err!(Box);
            }
        }
    };
}

macro_rules! iter_floats {
    ($l:expr, |$it:ident| $body:expr) => {
        match &$l {
            List::Integers(i) => {
                let $it = i.list().iter().map(|i| i.to_float().unwrap());
                $body
            }
            List::Doubles(d) => {
                let $it = d.list().iter().copied();
                $body
            }
            List::Booleans(b) => {
                let $it = b.list().iter().map(|b| b.to_float().unwrap());
                $body
            }
            List::String(s) => {
                let $it = s.list().iter().map(|c| c.to_float().unwrap());
                $body
            }
            List::Boxes(_) => {
                return err!(Box);
            }
        }
    }
}

macro_rules! with_list {
    ($l:expr, |$x:ident| $body:expr) => {
        match $l {
            List::Integers($x) => $body,
            List::Doubles($x) => $body,
            List::Booleans($x) => $body,
            List::String($x) => $body,
            List::Boxes($x) => $body,
        }
    };
}

pub(crate) use iter_ints;
pub(crate) use iter_floats;
pub(crate) use with_list;
