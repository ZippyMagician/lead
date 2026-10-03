use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::function::Function;
use crate::list::List;

pub type Scope = Rc<RefCell<ScopeInternal>>;

pub fn new_scope() -> Scope {
    Rc::new(RefCell::new(ScopeInternal::new()))
}

#[derive(Clone)]
pub enum MaybeInstant {
    Instant(List),
    Function(Function),
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

    pub fn child(parent: Rc<RefCell<Self>>) -> Self {
        Self {
            parent: Some(parent),
            env: HashMap::new(),
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.env.contains_key(name)
            || self
                .parent
                .as_ref()
                .map_or(false, |s| s.borrow().env.contains_key(name))
    }

    pub fn get(&self, name: &str) -> Option<MaybeInstant> {
        self.env
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.borrow().get(name))
    }
}
