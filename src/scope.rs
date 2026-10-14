use std::cell::RefCell;
use std::collections::HashMap;
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
pub enum MaybeInstant {
    Instant(List),
    Function(Function),
    Map(Map),
}

impl MaybeInstant {
    #[inline]
    pub fn is_instant(&self) -> bool {
        matches!(self, Self::Instant(_))
    }

    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(self, Self::Function(_))
    }

    #[inline]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    #[inline]
    pub fn to_instant(self) -> Status<List> {
        match self {
            Self::Instant(l) => Ok(l),
            _ => err!(Value),
        }
    }

    #[inline]
    pub fn to_function(self, arena: Arena, scope: Scope) -> Status<Function> {
        match self {
            Self::Function(f) => Ok(f),
            Self::Instant(l) => {
                hint::cold_path();
                Ok(function!(
                    move |_, _, _| Ok(l.clone());
                    arena, scope
                ))
            }
            Self::Map(_) => err!(Domain),
        }
    }

    #[inline]
    pub fn to_value(self) -> Status<Value> {
        match self {
            Self::Function(_) => err!(Value),
            Self::Instant(l) => Ok(Value::List(l)),
            Self::Map(m) => Ok(Value::Map(m)),
        }
    }
}

#[derive(Clone)]
pub struct ScopeInternal {
    parent: Option<Scope>,
    env: HashMap<String, MaybeInstant>,
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

    pub fn get(&self, name: &str) -> Option<MaybeInstant> {
        self.env
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.borrow().get(name))
    }

    pub fn set_instant(&mut self, name: &str, l: List) {
        self.env.insert(name.to_string(), MaybeInstant::Instant(l));
    }

    pub fn set_value(&mut self, name: &str, l: Value) {
        self.env.insert(
            name.to_string(),
            match l {
                Value::List(l) => MaybeInstant::Instant(l),
                Value::Map(m) => MaybeInstant::Map(m),
            },
        );
    }

    pub fn set_function(&mut self, name: &str, f: Function) {
        self.env.insert(name.to_string(), MaybeInstant::Function(f));
    }
}

impl From<Value> for MaybeInstant {
    #[inline]
    fn from(v: Value) -> Self {
        match v {
            Value::List(l) => Self::Instant(l),
            Value::Map(m) => Self::Map(m),
        }
    }
}

