use std::hint::unreachable_unchecked;
use std::mem::take;

use crate::builtins::operators::Operator;
use crate::function::{Function, function};
use crate::parse::{Assignment, Node};
use crate::scope::Scope;
use crate::types::Type;
use crate::types::list::List;
use crate::types::map;
use crate::types::scalar::Scalar;
use crate::utils::error::{Error, Status, err};
use crate::utils::{self, Arena};

use smallvec::{SmallVec, smallvec};

pub struct Exec {
    scope: Scope,
    arena: Arena,

    pub silent: bool,
    pub(crate) block: bool,
    pub(crate) reference: Option<Vec<Node<'static>>>,
    pub(crate) right: Option<Type>,
}

impl Exec {
    #[inline]
    pub fn new(arena: Arena, scope: Scope) -> Self {
        Self { scope, arena: arena, silent: false, block: false, reference: None, right: None }
    }

    #[inline]
    pub fn new_imm(arena: Arena, scope: Scope) -> Self {
        Self { scope, arena: arena, silent: false, block: true, reference: None, right: None }
    }

    pub fn exec<'a>(&mut self, mut node_lines: Vec<Node<'a>>) -> Status {
        let mut current = Vec::new();
        node_lines.push(Node::Line);
        if self.block && self.reference.is_none() {
            // SAFETY: Nodes lifetimes refer to &'a str, which are slices of a leaked input string.
            // Therefore, they will always point to data that exists until the end of the program.
            self.reference = Some(unsafe {
                std::mem::transmute::<Vec<Node<'a>>, Vec<Node<'static>>>(node_lines.clone())
            });
        }
        for node in node_lines {
            if matches!(node, Node::Line) {
                self.exec_line(take(&mut current))?;
                if !self.block
                    && !self.silent
                    && let Some(v) = self.right.take()
                {
                    println!("{v}");
                }
            } else {
                current.push(node);
            }
        }
        Ok(())
    }

    #[inline]
    fn exec_line<'a>(&mut self, mut line: Vec<Node<'a>>) -> Status {
        if line.is_empty() {
            return Ok(());
        }
        use Node::*;

        let mut i = line.len() - 1;
        loop {
            match take(unsafe { line.get_unchecked_mut(i) }) {
                Primitive(list) => self.right = Some(list.into()),
                PrimitiveString(list) => self.right = Some(list.into()),
                PrimitiveBoxes(list) => self.right = Some(list.into()),
                Literal(l) => self.right = Some(self.exec_literal(l, &mut line, &mut i)?),
                LiteralInternal(l) => self.right = self.exec_internal(l, &mut line, &mut i)?,
                Tag(t) => self.exec_tag(t, &mut line, &mut i)?,
                Function(f) => self.right = Some(self.exec_function(&f, &mut line, &mut i)?),
                Op(o, l, r) => {
                    let op = self.exec_operator(o, *l, *r)?;
                    self.right = Some(self.exec_function(&op, &mut line, &mut i)?);
                }
                Assign { head, body } => self.exec_assignment(
                    head,
                    body,
                    i > 0 && !utils::is_instant(&line[i - 1], &self.scope),
                )?,
                If { cond, body, otherwise } => {
                    self.exec_if(cond, body, otherwise, &mut line, &mut i)?
                }
                n @ Group(_) if utils::is_instant(&n, &self.scope) => {
                    self.right = Some(self.exec_instant(n)?)
                }
                Group(g) => {
                    let train = self.exec_train(g)?;
                    self.right = Some(self.exec_function(&train, &mut line, &mut i)?);
                }
                n @ Map(_) => self.right = Some(self.exec_instant(n)?),
                n @ Accessor(_, _) if utils::is_instant(&n, &self.scope) => {
                    self.right = Some(self.exec_instant(n)?)
                }
                Accessor(head, body) => {
                    let sc = self.scope.clone();
                    let s = sc.borrow();
                    let map = s.get(head).ok_or(Error::Scope)?.to_map()?;
                    let f = map
                        .get_into(&body[0], &body[1..])
                        .map(|n| unsafe { n.as_function_unchecked() })
                        .ok_or(Error::Domain)?;
                    self.right = Some(self.exec_function(f, &mut line, &mut i)?);
                }
                Line => {
                    if !self.block {
                        return err!(Internal, "Unexpected node in line");
                    }
                    self.right = None;
                }
                node => return err!(Internal, format!("TODO: Execute node {node:?}.")),
            }

            if i == 0 {
                break;
            } else {
                i -= 1;
            }
        }
        Ok(())
    }

    #[inline]
    fn exec_instant<'a>(&mut self, node: Node<'a>) -> Status<Type> {
        use Node::*;

        match node {
            Primitive(l) => Ok(l.into()),
            PrimitiveString(l) => Ok(l.into()),
            PrimitiveBoxes(l) => Ok(l.into()),
            Literal(l) => self.scope.borrow().get(l).ok_or(Error::Scope),
            LiteralInternal(s) => self.scope.borrow().internals.borrow().get(s).cloned(),
            Assign { head: Assignment::Literal(head), body } => {
                let s = self.scope.clone();
                let list = self.exec_instant(Node::Group(body))?;
                s.borrow_mut().set(head, list.clone());
                Ok(list)
            }
            Assign { head: Assignment::Accessor(head, tail), body } => {
                let s = self.scope.clone();
                let mut sc = s.borrow_mut();
                let m = sc.get_mut(head).ok_or(Error::Scope)?;
                let last = if tail.len() == 1 {
                    m.as_map_mut()?
                } else {
                    m.as_map_mut()?
                        .get_into_mut(&tail[0], &tail[1..tail.len() - 1])
                        .ok_or(Error::Domain)?
                        .as_map_mut()?
                };
                let list = self.exec_instant(Node::Group(body))?;
                last.insert(tail.last().unwrap().to_string(), list.clone(), &self.arena)?;
                Ok(list)
            }
            Group(body) => {
                let r = self.right.take();
                self.exec_line(body)?;
                let out = self.right.take();
                self.right = r;
                Ok(out.unwrap_or(List::empty().into()))
            }
            If { cond, body, otherwise } => {
                let prev = self.right.take();
                let list = self.exec_instant(*cond)?.to_list()?;
                if list.is_empty() || list.list()[0].to_bool()? {
                    self.exec_line(body)?;
                } else {
                    self.exec_line(otherwise)?;
                }
                let res = self.right.take();
                self.right = prev;
                Ok(res.ok_or(Error::Value)?)
            }
            Map(m) => {
                let mut map = map::Map::new();
                for (key, mut value) in m {
                    let value = if value.len() == 1 {
                        std::mem::take(&mut value[0])
                    } else {
                        Node::Group(value)
                    };
                    if utils::is_instant(&value, &self.scope) {
                        map.insert(key.to_string(), self.exec_instant(value)?, &self.arena)?;
                    } else {
                        let mut body =
                            if let Node::Group(value) = value { value } else { vec![value] };
                        let res = if let Some(Node::LiteralInternal(s)) = body.get(0).cloned()
                            && self.scope.borrow().internals.borrow().preprocess(s)
                        {
                            if !utils::is_instant(&body[body.len() - 1], &self.scope) {
                                return err!(Syntax, format!("Invalid use of internal []{s}"));
                            }
                            body.remove(0);
                            let right = self.exec_instant(Node::Group(body))?;
                            self.scope
                                .borrow()
                                .internals
                                .borrow()
                                .exec_internal(s, right, None, &self.scope, &self.arena)?
                                .ok_or(Error::Domain)?
                                .to_function(self.arena.clone(), self.scope.clone())?
                        } else {
                            self.exec_train(body)?
                        };
                        map.insert_function(key.to_string(), res, &self.arena)?;
                    }
                }
                Ok(map.into())
            }
            Accessor(head, body) => {
                let s = self.scope.borrow();
                s.get(head)
                    .ok_or(Error::Scope)?
                    .as_map()?
                    .get_into(&body[0], &body[1..])
                    .cloned()
                    .ok_or(Error::Domain)
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn exec_literal<'a>(
        &mut self, name: &'a str, list: &mut Vec<Node<'a>>, i: &mut usize,
    ) -> Status<Type> {
        let s = self.scope.clone();
        if self.block && name == "v" {
            if self.right.is_none() {
                return err!(Argument);
            }
            let mut left = None;
            let right = self.right.take().unwrap();
            if *i > 0 && utils::is_instant(&list[*i - 1], &self.scope) {
                left = Some(self.exec_instant(take(&mut list[*i - 1]))?);
                *i -= 1;
            }
            if *i == 0 {
                let mut sc = s.borrow_mut();
                // TODO: Check for maps, support alternate calls for blocks
                if let Some(left) = left {
                    sc.set_instant("x", left.to_list()?);
                }
                sc.set_instant("y", right.to_list()?);
                *list = self.reference.as_ref().unwrap().clone();
                *i = list.len();
                Ok(List::empty().into())
            } else {
                let f = utils::generate_block(
                    self.reference.take().unwrap(),
                    self.scope.clone(),
                    self.arena.clone(),
                );
                // TODO: Check for maps, support alternate calls for blocks
                if let Some(left) = left {
                    f.dyadic_call(left, right).map(Type::from)
                } else {
                    f.monadic_call(right).map(Type::from)
                }
            }
        } else {
            let typ = s.borrow().get(name).ok_or(Error::Scope)?;
            if typ.is_function() {
                let f = typ.to_function(self.arena.clone(), self.scope.clone()).unwrap();
                self.exec_function(&f, list, i)
            } else {
                Ok(typ)
            }
        }
    }

    #[inline]
    fn exec_internal<'a>(
        &mut self, name: &'a str, list: &mut Vec<Node<'a>>, i: &mut usize,
    ) -> Status<Option<Type>> {
        let s = self.scope.clone();
        let sc = s.borrow();
        let itc = sc.internals.clone();
        let it = itc.borrow();
        drop(sc);
        if !it.has(name) {
            drop(it);
            return err!(Scope);
        }

        if it.is_instant(name) {
            Ok(Some(it.get(name)?.clone()))
        } else {
            if self.right.is_none() {
                return err!(Argument);
            }
            let mut left = None;
            let right = self.right.take().unwrap();
            if *i > 0 && utils::is_instant(&list[*i - 1], &self.scope) {
                left = Some(self.exec_instant(take(&mut list[*i - 1]))?);
                *i -= 1;
            }

            it.exec_internal(name, right, left, &self.scope, &self.arena)
        }
    }

    #[inline]
    fn exec_function<'a>(
        &mut self, f: &Function, line: &mut [Node<'a>], i: &mut usize,
    ) -> Status<Type> {
        if self.right.is_none() {
            return err!(Argument);
        }
        let right = self.right.take().unwrap();
        // TODO: Check is value is map, support alternate block calls
        if *i > 0 && utils::is_instant(&line[*i - 1], &self.scope) {
            let left = self.exec_instant(take(&mut line[*i - 1]))?;
            *i -= 1;
            f.dyadic_call(left, right).map(Type::from)
        } else {
            f.monadic_call(right).map(Type::from)
        }
    }

    #[inline]
    fn exec_operator<'a>(
        &mut self, o: Operator, l: Node<'a>, r: Option<Node<'a>>,
    ) -> Status<Function> {
        o.getop(
            self.maybe_instant(l)?,
            r.map_or(Ok(None), |n| self.maybe_instant(n).map(Some))?,
            self.arena.clone(),
            self.scope.clone(),
        )
    }

    #[inline]
    fn exec_train<'a>(&mut self, mut g: Vec<Node<'a>>) -> Status<Function> {
        if g.is_empty() {
            return Ok(Function::scalar(List::empty(), self.arena.clone(), self.scope.clone()));
        }
        let mut has_colon = false;
        if matches!(&g[0], Node::Colon) {
            has_colon = true;
            g.remove(0);
        }

        let mut build: SmallVec<[Function; 3]> = smallvec![];
        let mut i = g.len() - 1;
        loop {
            let n = take(&mut g[i]);
            let f: Function;
            // I don't have to check for operators since these are only stored for now
            if utils::is_instant(&n, &self.scope) {
                let top = build.pop().unwrap();
                let left = self.exec_instant(n)?;
                if build.is_empty() {
                    build.push(function!(
                        move |a, b, _| top.dyadic_call(left.clone(), b.unwrap_or(a));
                        self.arena.clone(), self.scope.clone()
                    ));
                } else {
                    let rightfn = build.pop().unwrap();
                    build.push(function!(
                        move |a, b, _| {
                            top.dyadic_call(left.clone(), rightfn.call_fn(a, b)?)
                        }; self.arena.clone(), self.scope.clone()
                    ));
                }

                if i == 0 {
                    break;
                }
                i -= 1;
                continue;
            } else if let Node::Function(func) = n {
                f = func;
            } else if let Node::Op(o, l, r) = n {
                f = self.exec_operator(o, *l, *r)?;
            } else if let Node::Literal(n) = n {
                if self.block && n == "v" {
                    f = utils::generate_block(
                        self.reference.take().unwrap(),
                        self.scope.clone(),
                        self.arena.clone(),
                    );
                } else {
                    f = self
                        .scope
                        .borrow()
                        .get(n)
                        .ok_or(Error::Scope)?
                        .to_function(self.arena.clone(), self.scope.clone())?;
                }
            } else if let Node::LiteralInternal(s) = n {
                f = self
                    .scope
                    .borrow()
                    .internals
                    .borrow()
                    .get(s)?
                    .clone()
                    .to_function(self.arena.clone(), self.scope.clone())?;
            } else if let Node::Group(g) = n {
                f = self.exec_train(g)?;
            } else {
                return err!(Syntax, "Unexpected node in train");
            }

            if has_colon {
                if build.is_empty() {
                    build.push(f);
                } else {
                    let g = build.pop().unwrap();
                    build.push(function!(
                        move |a, b, _|
                            f.monadic_call(g.call_fn(a, b)?);
                        self.arena.clone(), self.scope.clone()
                    ));
                }
            } else {
                if build.len() == 2 {
                    let g = build.pop().unwrap();
                    let h = build.pop().unwrap();
                    build.push(function!(
                        move |a, b, _| {
                            g.dyadic_call(
                                f.call_fn(a.clone(), b.clone())?,
                                h.call_fn(a, b)?,
                            )
                        }; self.arena.clone(), self.scope.clone()
                    ));
                } else {
                    build.push(f);
                }
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        if build.len() == 2 {
            let f = build.pop().unwrap();
            let g = build.pop().unwrap();
            Ok(function!(
                move |a, b, _| f.monadic_call(g.call_fn(a, b)?);
                self.arena.clone(), self.scope.clone()
            ))
        } else {
            build.pop().ok_or(Error::Syntax("Empty expression.".into()))
        }
    }

    #[inline]
    fn exec_assignment<'a>(
        &mut self, head: Assignment<'a>, body: Vec<Node<'a>>, preserve_instant: bool,
    ) -> Status {
        let group = Node::Group(body);
        if utils::is_instant(&group, &self.scope) {
            let s = self.scope.clone();
            let list = self.exec_instant(group)?;
            if preserve_instant {
                // Preserves when used
                self.right = Some(list.clone());
            }

            if let Assignment::Literal(head) = head {
                s.borrow_mut().set(head, list);
            } else if let Assignment::Accessor(head, body) = head {
                let mut sc = s.borrow_mut();
                let m = sc.get_mut(head).ok_or(Error::Scope)?.as_map_mut()?;
                let last = if body.len() == 1 {
                    m
                } else {
                    m.get_into_mut(&body[0], &body[1..body.len() - 1])
                        .ok_or(Error::Domain)?
                        .as_map_mut()?
                };
                last.insert(body.last().unwrap().to_string(), list, &self.arena)?;
            } else if let Assignment::Internal(head) = head {
                let sc = s.borrow_mut();
                let mut it = sc.internals.borrow_mut();
                it.write_internal(head, list.into())?;
            }
        } else {
            // SAFETY: This is set as a `Node::Group` a few lines ago
            let Node::Group(mut body) = group else { unsafe { unreachable_unchecked() } };
            // Is it a preprocessed internal?
            let res = if let Some(Node::LiteralInternal(s)) = body.get(0).cloned()
                && self.scope.borrow().internals.borrow().preprocess(s)
            {
                if !utils::is_instant(&body[body.len() - 1], &self.scope) {
                    return err!(Syntax, format!("Invalid use of internal []{s}"));
                }
                body.remove(0);
                let right = self.exec_instant(Node::Group(body))?;
                self.scope
                    .borrow()
                    .internals
                    .borrow()
                    .exec_internal(s, right, None, &self.scope, &self.arena)?
                    .ok_or(Error::Domain)?
                    .to_function(self.arena.clone(), self.scope.clone())?
            } else {
                self.exec_train(body)?
            };

            if let Assignment::Literal(head) = head {
                self.scope.borrow_mut().set_function(head, res);
            } else if let Assignment::Accessor(head, body) = head {
                let s = self.scope.clone();
                let mut sc = s.borrow_mut();
                let m = sc.get_mut(head).ok_or(Error::Scope)?.as_map_mut()?;
                let last = if body.len() == 1 {
                    m
                } else {
                    m.get_into_mut(&body[0], &body[1..body.len() - 1])
                        .ok_or(Error::Domain)?
                        .as_map_mut()?
                };
                last.insert_function(body.last().unwrap().to_string(), res, &self.arena)?;
            } else if let Assignment::Internal(head) = head {
                let sc = self.scope.borrow_mut();
                let mut it = sc.internals.borrow_mut();
                it.write_internal(head, res.into())?;
            }
        }

        Ok(())
    }

    #[inline]
    fn exec_if<'a>(
        &mut self, cond: Box<Node<'a>>, body: Vec<Node<'a>>, otherwise: Vec<Node<'a>>,
        line: &mut Vec<Node<'a>>, i: &mut usize,
    ) -> Status {
        let list = self.exec_instant(*cond)?.to_list()?;
        line.drain(*i..);
        if list.is_empty() || list.list()[0].to_bool()? {
            line.reserve(body.len());
            line.extend(body);
        } else {
            line.extend(otherwise);
        }
        *i = line.len();
        Ok(())
    }

    #[inline]
    fn exec_tag<'a>(
        &mut self, tag: crate::lex::Token<'a>, _line: &mut [Node<'a>], _i: &mut usize,
    ) -> Status {
        use crate::lex::TokenTag;

        match tag {
            // Silent
            (TokenTag::Symbol, "~") => self.right = None,
            // Get type
            (TokenTag::Symbol, "?") => {
                let right = self.right.take().ok_or(Error::Argument)?;
                if right.is_map() {
                    self.right = Some(List::string("[:]").into());
                } else if right.is_function() {
                    self.right = Some(List::string("{}").into());
                } else {
                    let s = if right.is_scalar_strict() {
                        unsafe { right.to_scalar_unchecked() }
                    } else {
                        let right = unsafe { right.to_list_unchecked() };
                        if right.is_empty() {
                            self.right = Some(List::string("()").into());
                            return Ok(());
                        }
                        right.list()[0]
                    };
                    self.right = Some(
                        match s {
                            Scalar::Number(_) | Scalar::Float(_) => List::string("N"),
                            Scalar::Boolean(_) => List::string("B"),
                            Scalar::Char(_) => List::string("C"),
                            Scalar::Box(_) => List::string("X"),
                        }
                        .into(),
                    )
                }
            }
            (TokenTag::Symbol, ">") => {
                let right = self.right.take().ok_or(Error::Argument)?;
                if right.is_empty() || right.is_map() {
                    return err!(Value);
                }
                // SAFETY: `is_empty` returns true for functions
                let obj = if right.is_scalar() {
                    unsafe { right.to_scalar_unchecked().steal_box() }
                } else {
                    let right = unsafe { right.to_list_unchecked() };
                    unsafe { right.list()[0].steal_box() }
                };
                // Can treat a stolen function as an actual function
                self.right = Some(obj);
            }
            (TokenTag::String, s) if s.len() == 1 => {
                let mut right = self.right.take().ok_or(Error::Value)?.to_list()?;
                if right.is_empty() {
                    self.right = Some(right.into());
                } else {
                    let ident = s.as_bytes()[0] as char;
                    crate::utils::symbols::for_0!(right; [v] = v.to_cast(ident)?);
                    self.right = Some(right.into());
                }
            }
            _ => return err!(Syntax, "Unrecognized tag."),
        }

        Ok(())
    }

    #[inline]
    fn maybe_instant<'a>(&mut self, node: Node<'a>) -> Status<Type> {
        if utils::is_instant(&node, &self.scope) {
            Ok(Type::from(self.exec_instant(node)?))
        } else if let Node::Function(f) = node {
            Ok(Type::Function(f))
        } else if let Node::Op(o, l, r) = node {
            Ok(Type::Function(self.exec_operator(o, *l, *r)?))
        } else if let Node::Group(g) = node {
            Ok(Type::Function(self.exec_train(g)?))
        } else if let Node::Literal(name) = node {
            let t = self.scope.borrow().get(name).ok_or(Error::Scope)?;
            Ok(t)
        } else if let Node::LiteralInternal(name) = node {
            let t = self.scope.borrow().internals.borrow().get(name)?.clone();
            Ok(t)
        } else {
            err!(Syntax, "Unsupported node provided to operator")
        }
    }
}
