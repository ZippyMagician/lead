use std::collections::HashMap;
use std::fmt::{self, Display};

use super::Type;
use super::list::List;
use super::scalar::Scalar;
use crate::function::Function;
use crate::utils;
use crate::utils::error::Status;

use smallvec::smallvec;
use typed_arena::Arena;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Map {
    pub internal: HashMap<String, *mut Type>,
}

impl Map {
    #[inline]
    pub fn new() -> Self {
        Self { internal: HashMap::new() }
    }

    #[inline]
    pub fn insert(&mut self, key: String, value: Type, a: &utils::Arena) -> Status {
        let Scalar::Box(p) = Scalar::boxed_t(a, value)? else {
            unsafe { std::hint::unreachable_unchecked() }
        };
        self.internal.insert(key.to_string(), p);
        Ok(())
    }

    #[inline]
    pub fn insert_function(&mut self, key: String, value: Function, a: &utils::Arena) -> Status {
        let Scalar::Box(p) = Scalar::boxed_t(a, Type::Function(value))? else {
            unsafe { std::hint::unreachable_unchecked() }
        };
        self.internal.insert(key.to_string(), p);
        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&Type> {
        self.internal.get(key).map(|n| unsafe { &**n })
    }

    #[inline]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Type> {
        self.internal.get_mut(key).map(|n| unsafe { &mut **n })
    }

    #[inline]
    pub fn get_into(&self, key: &str, rest: &[&str]) -> Option<&Type> {
        if rest.is_empty() {
            self.get(key)
        } else {
            self.get(key).map(|m| m.as_map().ok()?.get_into(&rest[0], &rest[1..]))?
        }
    }

    #[inline]
    pub fn get_into_mut(&mut self, key: &str, rest: &[&str]) -> Option<&mut Type> {
        if rest.is_empty() {
            self.get_mut(key)
        } else {
            self.get_mut(key).map(|m| m.as_map_mut().ok()?.get_into_mut(&rest[0], &rest[1..]))?
        }
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
            pairs.push(Scalar::boxed_custom(&mut arena, List::string(key).into()).unwrap());
            pairs.push(Scalar::Box(*value));
        }
        List::mat(pairs, 2, smallvec![2, self.internal.len()]).unwrap()
    }

    #[inline]
    pub(crate) fn keys(&self, a: &utils::Arena) -> Status<List> {
        if self.is_empty() {
            return Ok(List::empty());
        }

        let mut keys = Vec::with_capacity(self.internal.len());
        for key in self.internal.keys() {
            keys.push(Scalar::boxed(a, List::string(key))?);
        }
        List::from_vec(keys)
    }

    #[inline]
    pub(crate) fn values(&self) -> Status<List> {
        if self.is_empty() {
            return Ok(List::empty());
        }

        let mut values = Vec::with_capacity(self.internal.len());
        for value in self.internal.values() {
            values.push(Scalar::Box(*value));
        }
        List::from_vec(values)
    }
}

#[macro_export]
macro_rules! map {
    ($($key:expr => $value:expr),*,) => {{
        $crate::map::Map {
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
            let mut formatted =
                format!("{}", Scalar::boxed_custom(&mut arena, Default::default()).unwrap());
            let Some(first) = formatted.chars().next() else {
                unsafe { std::hint::unreachable_unchecked() }
            };
            formatted.replace_range(..first.len_utf8(), ":");
            return write!(f, "{formatted}");
        }
        for (key, value) in &self.internal {
            pairs.push(Scalar::boxed_custom(&mut arena, List::string(key).into()).unwrap());
            pairs.push(Scalar::Box(*value));
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
