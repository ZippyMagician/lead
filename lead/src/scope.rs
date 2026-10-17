use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::function::Function;
use crate::types::list::List;
use crate::types::{Type, Value};

pub type Scope = Rc<RefCell<ScopeInternal>>;

pub fn new_scope() -> Scope {
    Rc::new(RefCell::new(ScopeInternal::new()))
}

#[derive(Clone)]
pub struct ScopeInternal {
    parent: Option<Scope>,
    env: HashMap<String, Type>,
}

impl ScopeInternal {
    fn new() -> Self {
        Self { parent: None, env: HashMap::new() }
    }

    pub fn child(parent: Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { parent: Some(parent), env: HashMap::new() }))
    }

    pub fn has(&self, name: &str) -> bool {
        self.env.contains_key(name)
            || self.parent.as_ref().is_some_and(|s| s.borrow().env.contains_key(name))
    }

    pub fn get(&self, name: &str) -> Option<Type> {
        self.env.get(name).cloned().or_else(|| self.parent.as_ref()?.borrow().get(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Type> {
        self.env.get_mut(name)
    }

    pub fn set_instant(&mut self, name: &str, l: List) {
        self.env.insert(name.to_string(), Type::Value(Value::List(l)));
    }

    pub fn set_value(&mut self, name: &str, l: Value) {
        self.env.insert(name.to_string(), Type::Value(l));
    }

    pub fn set_function(&mut self, name: &str, f: Function) {
        self.env.insert(name.to_string(), Type::Function(f));
    }
}
