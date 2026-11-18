use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::rc::Rc;

use crate::function::Function;
use crate::types::Type;
use crate::types::list::List;
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
    pub(crate) mutate_parent: bool,
    env: HashMap<String, Type>,
    pub(crate) internals: Rc<RefCell<Internals>>,
}

impl ScopeInternal {
    fn new(internals: Rc<RefCell<Internals>>) -> Self {
        Self { parent: None, mutate_parent: false, env: HashMap::new(), internals }
    }

    pub fn child(parent: Rc<RefCell<Self>>) -> Rc<RefCell<Self>> {
        let internals = parent.borrow().internals.clone();
        Rc::new(RefCell::new(Self {
            parent: Some(parent),
            mutate_parent: false,
            env: HashMap::new(),
            internals,
        }))
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
        if self.mutate_parent {
            let mut parent = self.parent.as_ref().unwrap().borrow_mut();
            if parent.has(name) {
                parent.set(name, t);
                return;
            }
        }

        self.env.insert(name.to_string(), t);
    }

    pub fn set_instant(&mut self, name: &str, l: List) {
        if self.mutate_parent {
            let mut parent = self.parent.as_ref().unwrap().borrow_mut();
            if parent.has(name) {
                parent.set_instant(name, l);
                return;
            }
        }

        self.env.insert(name.to_string(), Type::List(l));
    }

    pub fn set_function(&mut self, name: &str, f: Function) {
        if self.mutate_parent {
            let mut parent = self.parent.as_ref().unwrap().borrow_mut();
            if parent.has(name) {
                parent.set_function(name, f);
                return;
            }
        }

        self.env.insert(name.to_string(), Type::Function(f));
    }

    pub fn get_inner(&self) -> &HashMap<String, Type> {
        &self.env
    }

    pub fn swap_inner(&mut self, mut h: HashMap<String, Type>) -> HashMap<String, Type> {
        std::mem::swap(&mut self.env, &mut h);
        h
    }

    #[inline]
    pub fn write_internal(&self, name: &str, v: Type) -> Status {
        self.internals.borrow_mut().write_internal(name, v)
    }
}

impl Debug for ScopeInternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = &self.parent {
            s.fmt(f)?;
        } else {
            write!(f, "{{}}")?;
        }

        write!(f, " -> {:?}", self.env)
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
        // Instants
        #[cfg(not(feature = "wasm"))]
        {
            map.insert("I".to_string(), List::empty().into()); // TODO: Initialize I
            uniq.insert("STDOUT".to_string());
        }
        let x: String = unsafe { crate::utils::BOXES }.iter().collect();
        let boxes = List::string(&x);
        map.insert("BOXC".to_string(), boxes.into());
        map.insert("RANGEI".to_string(), true.into());

        // Functions
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
                let scope = new_scope();
                let mut l = crate::lex::Lexer::new(&contents);
                l.tokenize()?;
                let mut p = crate::parse::Parser::new(arena.clone(), scope.clone(), l.tokens());
                p.parse()?;
                let mut executor = crate::exec::Exec::new(arena.clone(), scope.clone());
                executor.silent = true;
                executor.exec(p.nodes())?;
                crate::types::map::Map::from_scope(scope).map(Type::Map).map(Some)
            }
            // Timing function, evaluates string input. Optional left arg is iterations
            b"time" => {
                use std::time::Instant;
                let iterations = b
                    .map(|n| -> Status<i64> {
                        Ok(n.as_list()?.listi()?.get(0).copied().unwrap_or(1))
                    })
                    .unwrap_or(Ok(1))? as usize;

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
                // TODO: Support other C ABI functions?
                let sym = crate::ffi::internal::link_symbol(&name, None)?;
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
            #[cfg(not(feature = "wasm"))]
            b"STDOUT" => err!(Domain),
            b"RANGEI" => err!(Domain),
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
    pub(crate) fn get_internal_rangei(&self) -> bool {
        unsafe { self.map.get("RANGEI").unwrap().as_list_unchecked().listb_unchecked()[0] }
    }

    #[inline]
    pub(crate) fn write_internal(&mut self, name: &str, v: Type) -> Status {
        if name.as_bytes() == b"BOXC" {
            let v = v.to_list()?;
            if v.len() != 11 && v.len() != 1 {
                return err!(Domain);
            } else if v.get_type() != b'C' {
                return err!(Domain);
            }
            use crate::utils::{BOXES, BOXES_BL, BOXES_BR, BOXES_TL, BOXES_TR};
            let s = str::from_utf8(unsafe { v.listc_unchecked() })?.chars();
            for (i, c) in (0..11).zip(s) {
                unsafe {
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
        } else if name.as_bytes() == b"STDOUT" {
            use std::io::Write;
            let mut stdout = std::io::stdout();
            write!(stdout, "{v}")?;
            return Ok(());
        } else if name.as_bytes() == b"RANGEI" {
            let v = v.as_list()?.is_truthy();
            self.map.insert("RANGEI".to_string(), v.into());
            return Ok(());
        }

        if !self.map.contains_key(name) {
            return err!(Scope);
        }

        self.map.insert(name.to_string(), v);
        Ok(())
    }
}
