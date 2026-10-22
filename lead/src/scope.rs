use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[allow(unused_imports)]
use crate::function::{Function, function};
use crate::types::Type;
use crate::types::list::List;
use crate::types::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};

pub type Scope = Rc<RefCell<ScopeInternal>>;

pub fn new_scope() -> Scope {
    let internals = Rc::new(RefCell::new(Internals::new()));
    Rc::new(RefCell::new(ScopeInternal::new(internals)))
}

#[derive(Clone, Default)]
pub struct ScopeInternal {
    parent: Option<Scope>,
    env: HashMap<String, Type>,
    pub(crate) internals: Rc<RefCell<Internals>>,
}

impl ScopeInternal {
    fn new(internals: Rc<RefCell<Internals>>) -> Self {
        Self { parent: None, env: HashMap::new(), internals }
    }

    pub fn child(parent: Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        let internals = parent.borrow().internals.clone();
        Rc::new(RefCell::new(Self { parent: Some(parent), env: HashMap::new(), internals }))
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

    pub fn set(&mut self, name: &str, t: Type) {
        self.env.insert(name.to_string(), t);
    }

    pub fn set_instant(&mut self, name: &str, l: List) {
        self.env.insert(name.to_string(), Type::List(l));
    }

    pub fn set_function(&mut self, name: &str, f: Function) {
        self.env.insert(name.to_string(), Type::Function(f));
    }

    pub fn into_inner(self) -> HashMap<String, Type> {
        self.env
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Internals {
    map: HashMap<String, Type>,
    uniq: HashSet<String>,
}

#[allow(dead_code)]
impl Internals {
    #[inline]
    pub(crate) fn new() -> Self {
        let mut uniq = HashSet::new();
        let mut map = HashMap::new();
        map.insert("I".to_string(), Type::from(List::empty())); // TODO: Initialize I
        let boxes = List::from_vec(
            unsafe { crate::utils::BOXES }.into_iter().map(|c| Scalar::Char(c)).collect(),
        );
        map.insert("BOXC".to_string(), Type::from(boxes.unwrap()));
        #[cfg(feature = "ffi")]
        {
            uniq.insert("extern".to_string());
            uniq.insert("link".to_string());
        }
        uniq.insert("time".to_string());
        uniq.insert("import".to_string());
        Self { map, uniq }
    }

    #[inline(always)]
    pub(crate) fn preprocess(&self, name: &str) -> bool {
        match name.as_bytes() {
            #[cfg(feature = "ffi")]
            b"extern" => true,
            #[cfg(feature = "ffi")]
            b"link" => true,
            _ => false,
        }
    }

    #[inline]
    pub(crate) fn has(&self, name: &str) -> bool {
        self.map.contains_key(name) || self.uniq.contains(name)
    }

    #[inline]
    pub(crate) fn is_instant(&self, name: &str) -> bool {
        self.map.get(name).is_some_and(|v| v.is_instant())
    }

    #[inline]
    pub(crate) fn get(&self, name: &str) -> Status<&Type> {
        self.map.get(name).ok_or(Error::Scope)
    }

    // SAFETY: Assumes called checks ahead of time if it is instant
    #[inline]
    unsafe fn get_function(&self, name: &str) -> &Function {
        unsafe { self.map.get(name).unwrap().as_function_unchecked() }
    }

    #[inline]
    pub(crate) fn exec_internal(
        &self, name: &str, a: Type, b: Option<Type>, scope: &Scope, arena: &Arena,
    ) -> Status<Option<Type>> {
        match name.as_bytes() {
            // Import file from within lead's path
            b"import" => {
                let name = format!("{}", a.to_list()?);
                let contents = std::fs::read_to_string(&name).map_err(|_| Error::Scope)?;
                let child = ScopeInternal::child(scope.clone());
                let mut executor = crate::exec::Exec::new(arena.clone(), child.clone());
                executor.silent = true;
                let mut l = crate::lex::Lexer::new(&contents);
                l.tokenize()?;
                let mut p = crate::parse::Parser::new(arena.clone(), child.clone(), l.tokens());
                p.parse()?;
                executor.exec(p.nodes())?;
                crate::types::map::Map::from_scope(child, arena).map(Type::Map).map(Some)
            }
            // Timing function, evaluates string input. Optional left arg is iterations
            b"time" => {
                use std::time::Instant;
                let iterations = b
                    .map(|n| n.as_list()?.list().get(0).map(|n| n.to_num()).unwrap_or(Ok(1)))
                    .unwrap_or(Ok(1))? as usize;

                let arena = Rc::new(RefCell::new(typed_arena::Arena::new()));
                let line = format!("{a}");
                let mut times = Vec::with_capacity(iterations);
                let mut exec = crate::exec::Exec::new(arena.clone(), scope.clone());
                exec.silent = true;
                let mut l = crate::lex::Lexer::new(&line);
                l.tokenize()?;

                for _ in 0..iterations {
                    let t0 = Instant::now();
                    let mut p = crate::parse::Parser::new(arena.clone(), scope.clone(), l.tokens());
                    p.parse()?;
                    exec.exec(p.nodes())?;
                    let t1 = Instant::now();
                    times.push(t1 - t0);
                }

                drop(arena);
                let mut acc = times[0];
                for i in 1..iterations {
                    acc += times[i];
                }
                acc /= iterations as u32;
                Ok(Some(List::string(&format!("{acc:?}")).into()))
            }
            // Extern declaration of shared object library
            #[cfg(feature = "ffi")]
            b"extern" => {
                use std::path::{Path, absolute};
                if b.is_some() {
                    return crate::err!(Domain);
                }
                let name = format!("{a}");
                let path = absolute(Path::new(&name)).unwrap();
                crate::ffi::internal::load_library(&path)?;
                Ok(None)
            }
            // Yields a function pointer to the specified external function symbol. Searches
            // through all currently declared shared object libraries. Expects specific
            // signature to be valid.
            #[cfg(feature = "ffi")]
            b"link" => {
                if b.is_some() {
                    return crate::err!(Domain);
                }
                let name = format!("{a}");
                let sym = crate::ffi::internal::link_symbol(&name)?;
                let ar = arena.clone();
                Ok(Some(
                    function!(move |a, b, _| {
                    crate::ffi::internal::call_symbol(
                        sym.clone(), &ar, a.to_list()?,
                        b.map(|n| n.to_list()).transpose()?,
                    ).map(Type::from)
                }; arena.clone(), scope.clone())
                    .into(),
                ))
            }
            _ => self
                .map
                .get(name)
                .unwrap()
                .clone()
                .to_function(arena.clone(), scope.clone())?
                .call_fn(a, b)
                .map(Some),
        }
    }

    #[inline]
    pub(crate) fn write_internal(&mut self, name: &str, v: Type) -> Status {
        if name.as_bytes() == b"BOXC" {
            let v = v.to_list()?;
            if v.len() != 11 && v.len() != 1 {
                return err!(Domain);
            } else if !v.list()[0].is_char() {
                return err!(Domain);
            }
            use crate::utils::{BOXES, BOXES_BL, BOXES_BR, BOXES_TL, BOXES_TR};
            for (i, c) in (0..11).zip(v.list().iter().cycle()) {
                unsafe {
                    let c = c.to_char_unchecked();
                    BOXES[i] = c;
                }
            }
            // Big box of nonsense
            unsafe {
                BOXES_TL = [
                    BOXES[0], BOXES[3], BOXES[0], BOXES[3], BOXES[0], BOXES[3], BOXES[0], BOXES[3],
                    BOXES[1], BOXES[4], BOXES[1], BOXES[4], BOXES[1], BOXES[4], BOXES[1], BOXES[4],
                ];
                BOXES_TR = [
                    BOXES[2], BOXES[5], BOXES[1], BOXES[4], BOXES[2], BOXES[5], BOXES[1], BOXES[4],
                    BOXES[2], BOXES[5], BOXES[1], BOXES[4], BOXES[2], BOXES[5], BOXES[1], BOXES[4],
                ];
                BOXES_BR = [
                    BOXES[8], BOXES[8], BOXES[7], BOXES[7], BOXES[5], BOXES[5], BOXES[4], BOXES[4],
                    BOXES[8], BOXES[8], BOXES[7], BOXES[7], BOXES[5], BOXES[5], BOXES[4], BOXES[4],
                ];
                BOXES_BL = [
                    BOXES[6], BOXES[6], BOXES[6], BOXES[6], BOXES[3], BOXES[3], BOXES[3], BOXES[3],
                    BOXES[7], BOXES[7], BOXES[7], BOXES[7], BOXES[4], BOXES[4], BOXES[4], BOXES[4],
                ];
            }
            self.map.insert("BOXC".to_string(), v.into());

            return Ok(());
        }

        if !self.map.contains_key(name) {
            return err!(Scope);
        }

        self.map.insert(name.to_string(), v);
        Ok(())
    }
}
