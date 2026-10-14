use std::collections::HashMap;
use std::fmt::{self, Display};

use crate::exec::Value;
use crate::list::List;
use crate::scalar::Scalar;

use smallvec::smallvec;
use typed_arena::Arena;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Map {
    pub internal: HashMap<String, Value>,
}

impl Map {
    #[inline]
    pub fn new() -> Self {
        Self {
            internal: HashMap::new(),
        }
    }

    #[inline]
    pub fn insert(&mut self, key: String, value: Value) -> Option<Value> {
        self.internal.insert(key.to_string(), value)
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.internal.get(key)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.internal.is_empty()
    }

    #[inline]
    fn as_repr(&self) -> List {
        if self.is_empty() {
            return List::empty();
        }
        let mut pairs = Vec::new();
        let mut arena = Arena::new();
        for (key, value) in &self.internal {
            pairs.push(Scalar::boxed_custom(&mut arena, Value::List(List::string(key))).unwrap());
            pairs.push(Scalar::boxed_custom(&mut arena, value.clone()).unwrap());
        }
        List::mat(pairs, 2, smallvec![2, self.internal.len()]).unwrap()
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

impl PartialOrd for Map {
    #[inline]
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&o))
    }
}

impl Ord for Map {
    #[inline]
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.as_repr().cmp(&o.as_repr())
    }
}

impl Display for Map {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pairs = Vec::new();
        let mut arena = Arena::new();
        if self.internal.is_empty() {
            return write!(
                f,
                "{}",
                Scalar::boxed_custom(&mut arena, Value::List(List::empty())).unwrap()
            );
        }
        for (key, value) in &self.internal {
            pairs.push(Scalar::boxed_custom(&mut arena, Value::List(List::string(key))).unwrap());
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
