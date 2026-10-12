use std::collections::HashMap;
use std::fmt::{self, Display};

use crate::list::List;
use crate::scalar::Scalar;

use smallvec::smallvec;
use typed_arena::Arena;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Map {
    pub internal: HashMap<String, List>,
}

impl Map {
    pub fn new() -> Self {
        Self {
            internal: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: List) -> Option<List> {
        self.internal.insert(key.to_string(), value)
    }

    pub fn get(&self, key: &str) -> Option<&List> {
        self.internal.get(key)
    }
}

#[macro_export]
macro_rules! map {
    ($($key:expr => $value:expr),*,) => {{
        seqr::map::Map {
            internal: ::std::collections::HashMap::from([
                $(($key.to_string(), $value)),*,
            ]),
        }
    }}
}

pub use map;

impl Display for Map {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pairs = Vec::new();
        let mut arena = Arena::new();
        if self.internal.is_empty() {
            return write!(f, "{}", Scalar::boxed_custom(&mut arena, List::empty()).unwrap());
        }
        for (key, value) in &self.internal {
            pairs.push(Scalar::boxed_custom(&mut arena, List::string(key)).unwrap());
            pairs.push(Scalar::boxed_custom(&mut arena, value.clone()).unwrap());
        }

        let list = List::mat(pairs, 2, smallvec![2, self.internal.len()]).unwrap();
        let mut formatted = format!("{list}");
        let Some(first) = formatted.chars().next() else {
            unsafe { std::hint::unreachable_unchecked() }
        };
        formatted.replace_range(..first.len_utf8(), ":");
        write!(f, "{formatted}")
    }
}
