pub mod list;
pub mod map;
pub mod scalar;

use std::fmt::{self, Display};
use std::hint;

use crate::function::{Function, function};
use crate::scope::Scope;
use crate::utils::Arena;
use crate::utils::error::{Status, err};

use list::List;
use map::Map;

#[derive(Clone, Debug)]
#[repr(C, u8)]
pub enum Type {
    Function(Function),
    Value(Value),
}

impl Type {
    #[inline]
    pub fn is_instant(&self) -> bool {
        matches!(self, Self::Value(Value::List(_)))
    }

    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }

    #[inline]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Value(Value::Map(_)))
    }

    #[inline]
    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    #[inline]
    pub fn to_instant(self) -> Status<List> {
        match self {
            Self::Value(l) => l.to_list(),
            _ => {
                hint::cold_path();
                err!(Value)
            }
        }
    }

    #[inline]
    pub fn to_function(self, arena: Arena, scope: Scope) -> Status<Function> {
        match self {
            Self::Function(f) => Ok(f),
            Self::Value(l @ Value::List(_)) => {
                hint::cold_path();
                Ok(function!(
                    move |_, _, _| Ok(l.clone());
                    arena, scope
                ))
            }
            _ => {
                hint::cold_path();
                err!(Domain)
            }
        }
    }

    #[inline]
    pub unsafe fn as_function_unchecked(&self) -> &Function {
        match &self {
            Self::Function(f) => f,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub fn to_list(self) -> Status<List> {
        match self {
            Self::Value(Value::List(l)) => Ok(l),
            _ => err!(ValueS, "Cannot treat as list"),
        }
    }

    #[inline]
    pub fn as_list(&self) -> Status<&List> {
        match self {
            Self::Value(Value::List(l)) => Ok(l),
            _ => err!(ValueS, "Cannot treat as list"),
        }
    }

    #[inline]
    pub fn to_map(self) -> Status<Map> {
        match self {
            Self::Value(Value::Map(m)) => Ok(m),
            _ => err!(ValueS, "Cannot treat as map"),
        }
    }

    #[inline]
    pub fn as_map(&self) -> Status<&Map> {
        match &self {
            Self::Value(v) => v.as_map(),
            _ => err!(ValueS, "Cannot treat as map"),
        }
    }

    #[inline]
    pub fn as_map_mut(&mut self) -> Status<&mut Map> {
        match self {
            Self::Value(Value::Map(m)) => Ok(m),
            _ => {
                hint::cold_path();
                err!(ValueS, "Cannot treat primitive list as map.")
            }
        }
    }

    #[inline]
    pub fn to_value(self) -> Status<Value> {
        match self {
            Self::Function(_) => {
                hint::cold_path();
                err!(Value)
            }
            Self::Value(v) => Ok(v),
        }
    }

    #[inline]
    pub fn as_value(&self) -> Status<&Value> {
        match &self {
            Self::Function(_) => {
                hint::cold_path();
                err!(Value)
            }
            Self::Value(v) => Ok(v),
        }
    }

    #[inline]
    pub unsafe fn as_value_unchecked(&self) -> &Value {
        match &self {
            Self::Value(v) => v,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }
}

impl From<List> for Type {
    fn from(l: List) -> Self {
        Self::Value(Value::List(l))
    }
}

impl From<Map> for Type {
    fn from(m: Map) -> Self {
        Self::Value(Value::Map(m))
    }
}

impl From<Function> for Type {
    fn from(f: Function) -> Self {
        Self::Function(f)
    }
}

#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Value {
    List(List),
    Map(Map),
}

impl Value {
    #[inline]
    pub fn to_list(self) -> Status<List> {
        match self {
            Self::List(l) => Ok(l),
            Self::Map(_) => {
                hint::cold_path();
                err!(ValueS, "Cannot treat map as primitive list.")
            }
        }
    }

    #[inline]
    pub fn as_list(&self) -> Status<&List> {
        match &self {
            Self::List(l) => Ok(l),
            _ => {
                hint::cold_path();
                err!(ValueS, "Cannot treat map as primitive list.")
            }
        }
    }

    #[inline]
    pub unsafe fn to_list_unchecked(self) -> List {
        match self {
            Self::List(l) => l,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub fn to_map(self) -> Status<Map> {
        match self {
            Self::List(_) => {
                hint::cold_path();
                err!(ValueS, "Cannot treat primitive list as map.")
            }
            Self::Map(m) => Ok(m),
        }
    }

    #[inline]
    pub fn as_map(&self) -> Status<&Map> {
        match &self {
            Self::List(_) => {
                hint::cold_path();
                err!(ValueS, "Cannot treat primitive list as map.")
            }
            Self::Map(m) => Ok(m),
        }
    }

    #[inline]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    #[inline]
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::List(l) => l.is_empty(),
            Self::Map(m) => m.is_empty(),
        }
    }
}

impl PartialOrd for Type {
    #[inline]
    fn partial_cmp(&self, o: &Type) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&o))
    }
}

impl PartialEq for Type {
    #[inline]
    fn eq(&self, o: &Type) -> bool {
        match (&self, &o) {
            (Self::Function(_), _) => false,
            (_, Self::Function(_)) => false,
            (Self::Value(v), Self::Value(o)) => v == o,
        }
    }
}

impl Ord for Type {
    #[inline]
    fn cmp(&self, o: &Type) -> std::cmp::Ordering {
        if self.is_function() && !o.is_function() {
            hint::cold_path();
            std::cmp::Ordering::Less
        } else if !self.is_function() && o.is_function() {
            hint::cold_path();
            std::cmp::Ordering::Greater
        } else if self.is_function() {
            hint::cold_path();
            std::cmp::Ordering::Equal
        } else {
            unsafe { self.as_value_unchecked().cmp(o.as_value_unchecked()) }
        }
    }
}

impl Eq for Type {}

impl From<Value> for Type {
    #[inline]
    fn from(v: Value) -> Self {
        Self::Value(v)
    }
}

impl Default for Type {
    #[inline]
    fn default() -> Self {
        Self::Value(Default::default())
    }
}

impl Display for Type {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Function(fnc) if let Some(s) = fnc.s => {
                write!(f, "{}", str::from_utf8(s.get_name()).unwrap())
            }
            Self::Function(_) => write!(f, "{{{{ .. }}}}"),
            Self::Value(v) => write!(f, "{v}"),
        }
    }
}

impl From<Map> for Value {
    #[inline(always)]
    fn from(m: Map) -> Self {
        Self::Map(m)
    }
}

impl From<List> for Value {
    #[inline(always)]
    fn from(l: List) -> Self {
        Self::List(l)
    }
}

impl Default for Value {
    #[inline(always)]
    fn default() -> Self {
        Self::List(List::default())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List(l) => write!(f, "{l}"),
            Self::Map(m) => write!(f, "{m}"),
        }
    }
}
