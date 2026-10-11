use std::cell::RefCell;
use std::collections::HashMap;
use std::hint;
use std::rc::Rc;

use crate::function::{Function, function};
use crate::list::List;
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
                let m = l.clone();
                let d = l;
                Ok(function!(
                    move |_, _| Ok(m.clone());
                    move |_, _, _| Ok(d.clone());
                    arena, scope
                ))
            }
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

    pub fn set_function(&mut self, name: &str, f: Function) {
        self.env.insert(name.to_string(), MaybeInstant::Function(f));
    }
}
