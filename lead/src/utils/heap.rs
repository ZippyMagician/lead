use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::types::Type;
use crate::types::list::List;
use crate::types::scalar::Boxed;

use typed_arena::Arena;

#[derive(Default)]
pub struct Heap {
    arena: Arena<Type>,
    // For debugging
    pub(crate) live: RefCell<Vec<*mut Type>>,
    pub(crate) free: RefCell<Vec<*mut Type>>,

    last_free: usize,
}

impl Heap {
    #[inline]
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            live: RefCell::new(Vec::new()),
            free: RefCell::new(Vec::new()),
            last_free: 1,
        }
    }

    #[inline]
    pub fn alloc(&self, t: Type) -> *mut Type {
        let mut free = self.free.borrow_mut();
        let mut live = self.live.borrow_mut();
        let p = if let Some(p) = free.pop() {
            unsafe {
                *p = t;
            }
            p
        } else {
            self.arena.alloc(t)
        };

        live.push(p);
        p
    }

    #[inline]
    pub fn should_collect(&self) -> bool {
        self.free.borrow().is_empty() && self.last_free * 4 < self.live.borrow().len()
    }

    #[inline]
    pub fn collect(&mut self, mut stack: Vec<Boxed>) {
        // mark
        let mut marked = HashSet::new();
        while let Some(b) = stack.pop() {
            if marked.insert(b.0) {
                unsafe {
                    self.trace(&*b.0, &mut stack);
                }
            }
        }
        // free
        let mut keep = Vec::with_capacity(marked.len());
        let mut live = self.live.borrow_mut();
        let mut free = self.free.borrow_mut();
        for p in live.drain(..) {
            if marked.contains(&p) {
                keep.push(p);
            } else {
                unsafe {
                    *p = Default::default();
                } // cheapest type
                free.push(p);
            }
        }
        drop(live);
        self.last_free = keep.len();
        self.live.replace(keep);
    }

    #[inline]
    fn trace(&self, t: &Type, stack: &mut Vec<Boxed>) {
        if let Type::List(List::Boxes(b)) = t {
            for x in b.list() {
                self.trace_box(x, stack);
            }
        }
    }

    #[inline]
    fn trace_box(&self, b: &Boxed, stack: &mut Vec<Boxed>) {
        stack.push(*b);
        unsafe {
            self.trace(&*b.0, stack);
        }
    }

    #[inline]
    pub fn build_trace(&self, env: &HashMap<String, Type>) -> Vec<Boxed> {
        let live = self.live.borrow();
        let mut trace = Vec::with_capacity(live.len());
        for t in env.values() {
            self.trace(t, &mut trace);
        }

        trace
    }
}
