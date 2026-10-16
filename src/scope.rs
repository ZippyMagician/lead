use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Display};
use std::hint;
use std::rc::Rc;

use crate::exec::Value;
use crate::function::{Function, function};
use crate::list::List;
use crate::map::Map;
use crate::utils::Arena;
use crate::utils::error::{Status, err};

pub type Scope = Rc<RefCell<ScopeInternal>>;

pub fn new_scope() -> Scope {
    Rc::new(RefCell::new(ScopeInternal::new()))
}

#[derive(Clone, Debug)]
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
                std::hint::cold_path();
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

#[derive(Clone)]
pub struct ScopeInternal {
    parent: Option<Scope>,
    env: HashMap<String, Type>,
}

impl ScopeInternal {
    fn new() -> Self {
        Self {
            parent: None,
            env: HashMap::new(),
        }
    }

    pub fn child(parent: Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            parent: Some(parent),
            env: HashMap::new(),
        }))
    }

    pub fn has(&self, name: &str) -> bool {
        self.env.contains_key(name)
            || self
                .parent
                .as_ref()
                .is_some_and(|s| s.borrow().env.contains_key(name))
    }

    pub fn get(&self, name: &str) -> Option<Type> {
        self.env
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.borrow().get(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Type> {
        self.env.get_mut(name)
    }

    pub fn set_instant(&mut self, name: &str, l: List) {
        self.env
            .insert(name.to_string(), Type::Value(Value::List(l)));
    }

    pub fn set_value(&mut self, name: &str, l: Value) {
        self.env.insert(name.to_string(), Type::Value(l));
    }

    pub fn set_function(&mut self, name: &str, f: Function) {
        self.env.insert(name.to_string(), Type::Function(f));
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
