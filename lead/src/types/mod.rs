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
use scalar::Scalar;

#[derive(Clone, Debug)]
#[repr(C, u8)]
pub enum Type {
    Function(Function),
    List(List),
    Scalar(Scalar),
    Map(Map),
}

impl Type {
    #[inline]
    pub fn is_instant(&self) -> bool {
        !self.is_function()
    }

    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }

    #[inline]
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_) | Self::Scalar(_))
    }

    #[inline]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_)) || self.is_list() && self.as_list().unwrap().is_scalar()
    }

    #[inline]
    pub fn is_scalar_strict(&self) -> bool {
        matches!(self, Self::Scalar(_))
    }

    #[inline]
    pub fn is_list_strict(&self) -> bool {
        matches!(self, Self::List(_))
    }

    #[inline]
    pub fn to_function(self, arena: Arena, scope: Scope) -> Status<Function> {
        match self {
            Self::Function(f) => Ok(f),
            l => {
                hint::cold_path();
                Ok(function!(
                    move |_, _, _| Ok(l.clone());
                    arena, scope
                ))
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
            Self::List(l) => Ok(l),
            Self::Scalar(s) => Ok(List::scalar(s)),
            _ => err!(ValueS, "Cannot treat as list"),
        }
    }

    #[inline]
    pub fn as_list(&self) -> Status<&List> {
        match self {
            Self::List(l) => Ok(l),
            _ => err!(ValueS, "Cannot treat as list"),
        }
    }

    #[inline]
    pub fn to_map(self) -> Status<Map> {
        match self {
            Self::Map(m) => Ok(m),
            _ => err!(ValueS, "Cannot treat as map"),
        }
    }

    #[inline]
    pub fn as_map(&self) -> Status<&Map> {
        match &self {
            Self::Map(m) => Ok(m),
            _ => err!(ValueS, "Cannot treat as map"),
        }
    }

    #[inline]
    pub fn as_map_mut(&mut self) -> Status<&mut Map> {
        match self {
            Self::Map(m) => Ok(m),
            _ => {
                hint::cold_path();
                err!(ValueS, "Cannot treat as map.")
            }
        }
    }

    #[inline]
    pub fn to_scalar(self) -> Status<Scalar> {
        match self {
            Self::Scalar(s) => Ok(s),
            _ => {
                hint::cold_path();
                err!(ValueS, "Cannot treat as map.")
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
    pub unsafe fn as_list_unchecked(&self) -> &List {
        match self {
            Self::List(l) => l,
            _ => unsafe { hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self {
            Self::Function(_) => true,
            Self::List(l) => l.is_empty(),
            Self::Map(m) => m.is_empty(),
            Self::Scalar(_) => false,
        }
    }
}

impl From<Scalar> for Type {
    fn from(s: Scalar) -> Self {
        Self::Scalar(s)
    }
}

impl From<List> for Type {
    fn from(l: List) -> Self {
        Self::List(l)
    }
}

impl From<Map> for Type {
    fn from(m: Map) -> Self {
        Self::Map(m)
    }
}

impl From<Function> for Type {
    fn from(f: Function) -> Self {
        Self::Function(f)
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
            (Self::List(l), Self::List(r)) => l == r,
            (Self::Map(l), Self::Map(r)) => l == r,
            _ => false,
        }
    }
}

use std::cmp::Ordering;
impl Ord for Type {
    #[inline]
    fn cmp(&self, o: &Type) -> Ordering {
        if self.is_function() && !o.is_function() {
            hint::cold_path();
            Ordering::Less
        } else if !self.is_function() && o.is_function() {
            hint::cold_path();
            Ordering::Greater
        } else if self.is_function() {
            hint::cold_path();
            Ordering::Equal
        } else {
            match (&self, &o) {
                (Self::List(l), Self::List(r)) => l.cmp(r),
                (Self::List(_), Self::Map(_)) => Ordering::Greater,
                (Self::Map(l), Self::Map(r)) => l.cmp(r),
                _ => Ordering::Less,
            }
        }
    }
}

impl Eq for Type {}

impl Default for Type {
    #[inline]
    fn default() -> Self {
        Self::List(List::empty())
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
            Self::List(v) => write!(f, "{v}"),
            Self::Map(m) => write!(f, "{m}"),
            Self::Scalar(s) => write!(f, "{s}"),
        }
    }
}
