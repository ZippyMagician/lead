use std::collections::HashMap;
use std::fmt::{self, Display};

use super::Type;
use super::list::{Array, List};
use super::scalar::{self, Boxed};
use crate::function::Function;
use crate::scope::Scope;
use crate::utils;
use crate::utils::error::Status;

use typed_arena::Arena;

#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Map {
    pub internal: HashMap<String, Boxed>,
}

impl Map {
    #[inline]
    pub fn from_scope(s: Scope, a: &utils::Arena) -> Status<Self> {
        let mut internal = HashMap::new();
        for (key, t) in s.take().into_inner() {
            let p = scalar::boxed_t(a, t)?;
            internal.insert(key, p);
        }
        Ok(Self { internal })
    }

    #[inline]
    pub fn new() -> Self {
        Self { internal: HashMap::new() }
    }

    #[inline]
    pub fn insert(&mut self, key: String, value: Type, a: &utils::Arena) -> Status {
        let p = scalar::boxed_t(a, value)?;
        self.internal.insert(key.to_string(), p);
        Ok(())
    }

    #[inline]
    pub fn insert_function(&mut self, key: String, value: Function, a: &utils::Arena) -> Status {
        let p = scalar::boxed_t(a, Type::Function(value))?;
        self.internal.insert(key.to_string(), p);
        Ok(())
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<&Type> {
        self.internal.get(key).map(|n| unsafe { &***n })
    }

    #[inline]
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Type> {
        self.internal.get_mut(key).map(|n| unsafe { &mut ***n })
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
    fn as_repr(&self) -> (Option<Arena<Type>>, List) {
        if self.is_empty() {
            return (None, List::empty());
        }

        let mut arena = Arena::new();
        let boxes = unsafe {
            Array::<Boxed>::try_alloc_iter(
                &[2, self.internal.len()],
                self.internal.iter().flat_map(|(k, v)| {
                    [scalar::boxed_custom(&mut arena, List::string(k).into()), Ok(*v)]
                }),
            ).unwrap()
        };
        (Some(arena), List::Boxes(boxes))
    }

    #[inline]
    pub(crate) fn keys(&self, a: &utils::Arena) -> Status<List> {
        if self.is_empty() {
            return Ok(List::empty());
        }

        unsafe {
            Array::<Boxed>::try_alloc_iter(
                &[self.internal.len()],
                self.internal.keys().map(|v| scalar::boxed(a, List::string(v))),
            )
            .map(List::Boxes)
        }
    }

    #[inline]
    pub(crate) fn values(&self) -> Status<List> {
        if self.is_empty() {
            return Ok(List::empty());
        }

        unsafe {
            Array::<Boxed>::alloc_iter(&[self.internal.len()], self.internal.values().copied())
        }
    }

    #[inline]
    pub(crate) fn key_values(&self, a: &utils::Arena) -> Status<List> {
        if self.is_empty() {
            return Ok(List::empty());
        }

        unsafe {
            Array::<Boxed>::try_alloc_iter(
                &[2, self.internal.len()],
                self.internal.iter().flat_map(|(k, v)| [scalar::boxed(a, List::string(k)), Ok(*v)]),
            )
            .map(List::Boxes)
        }
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
        let (_a, r) = self.as_repr();
        let (_a, r2) = o.as_repr();
        r.cmp(&r2)
    }
}

impl Display for Map {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut arena = Arena::new();
        if self.internal.is_empty() {
            let mut formatted =
                format!("{}", scalar::boxed_custom(&mut arena, Default::default()).unwrap());
            let Some(first) = formatted.chars().next() else {
                unsafe { std::hint::unreachable_unchecked() }
            };
            formatted.replace_range(..first.len_utf8(), ":");
            return write!(f, "{formatted}");
        }
        let (a, list) = self.as_repr();
        let mut formatted = format!("{list}");
        drop(a);
        let Some(first) = formatted.chars().next() else {
            unsafe { std::hint::unreachable_unchecked() }
        };
        formatted.replace_range(..first.len_utf8(), ":");
        write!(f, "{formatted}")
    }
}
