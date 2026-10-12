use std::hint::unreachable_unchecked;
use std::mem::take;

use crate::builtins::operators::Operator;
use crate::function::{Function, function};
use crate::list::List;
use crate::parse::Node;
use crate::scalar::Scalar;
use crate::scope::{MaybeInstant, Scope};
use crate::utils::error::{Error, Status, err};
use crate::utils::{self, Arena};

use smallvec::{SmallVec, smallvec};

pub struct Exec {
    scope: Scope,
    arena: Arena,

    pub(crate) block: bool,
    pub(crate) reference: Option<Vec<Node<'static>>>,
    pub(crate) right: Option<List>,
}

impl Exec {
    #[inline]
    pub fn new(arena: Arena, scope: Scope) -> Self {
        Self {
            scope,
            arena: arena,
            block: false,
            reference: None,
            right: None,
        }
    }

    #[inline]
    pub fn new_imm(arena: Arena, scope: Scope) -> Self {
        Self {
            scope,
            arena: arena,
            block: true,
            reference: None,
            right: None,
        }
    }

    pub fn exec<'a>(&mut self, mut node_lines: Vec<Node<'a>>) -> Status {
        let mut current = Vec::new();
        node_lines.push(Node::Line);
        if self.block && self.reference.is_none() {
            self.reference = Some(unsafe {
                std::mem::transmute::<Vec<Node<'a>>, Vec<Node<'static>>>(node_lines.clone())
            });
        }
        for node in node_lines {
            if matches!(node, Node::Line) {
                self.exec_line(take(&mut current))?;
                if !self.block
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
                Primitive(list) => self.right = Some(list),
                PrimitiveString(list) => self.right = Some(list),
                PrimitiveBoxes(list) => self.right = Some(list),
                Literal(l) => self.right = Some(self.exec_literal(l, &mut line, &mut i)?),
                LiteralInternal(l) => self.right = Some(self.exec_internal(l)?),
                Tag(t) => self.exec_tag(t)?,
                Function(f) => self.right = Some(self.exec_function(f, &mut line, &mut i)?),
                Op(o, l, r) => {
                    let op = self.exec_operator(o, *l, *r)?;
                    self.right = Some(self.exec_function(op, &mut line, &mut i)?);
                }
                Assign {
                    head,
                    is_internal,
                    body,
                } => self.exec_assignment(
                    head,
                    body,
                    is_internal,
                    i > 0 && !utils::is_instant(&line[i - 1], &self.scope),
                )?,
                n @ Group(_) if utils::is_instant(&n, &self.scope) => {
                    self.right = Some(self.exec_instant(n)?)
                }
                Group(g) => {
                    let train = self.exec_train(g)?;
                    self.right = Some(self.exec_function(train, &mut line, &mut i)?);
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
    fn exec_instant<'a>(&mut self, node: Node<'a>) -> Status<List> {
        use Node::*;

        match node {
            Primitive(l) => Ok(l),
            PrimitiveString(l) => Ok(l),
            PrimitiveBoxes(l) => Ok(l),
            Literal(l) => self.scope.borrow().get(l).unwrap().to_instant(),
            LiteralInternal(_) => todo!("Internals @ <exec.rs> exec_instant"),
            Assign {
                head,
                is_internal,
                body,
            } => {
                if is_internal {
                    todo!("Internals @ <exec.rs> exec_instant");
                }
                let s = self.scope.clone();
                let list = self.exec_instant(Node::Group(body))?;
                s.borrow_mut().set_instant(head, list.clone());
                Ok(list)
            }
            Group(body) => {
                let r = self.right.take();
                self.exec_line(body)?;
                let out = self.right.take();
                self.right = r;
                Ok(out.unwrap_or(List::empty()))
            }
            _ => unreachable!(),
        }
    }

    #[inline]
    fn exec_literal<'a>(
        &mut self,
        name: &'a str,
        list: &mut Vec<Node<'a>>,
        i: &mut usize,
    ) -> Status<List> {
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
                if let Some(left) = left {
                    sc.set_instant("x", left);
                }
                sc.set_instant("y", right);
                *list = self.reference.as_ref().unwrap().clone();
                *i = list.len();
                Ok(List::empty())
            } else {
                let f = utils::generate_block(
                    take(self.reference.as_mut().unwrap()),
                    self.scope.clone(),
                    self.arena.clone(),
                );
                if let Some(left) = left {
                    f.dyadic_call(left, right)
                } else {
                    f.monadic_call(right)
                }
            }
        } else {
            let value = s.borrow().get(name).ok_or(Error::Scope)?;
            if value.is_instant() {
                Ok(value.to_instant().unwrap())
            } else {
                let f = value
                    .to_function(self.arena.clone(), self.scope.clone())
                    .unwrap();
                self.exec_function(f, list, i)
            }
        }
    }

    #[inline]
    fn exec_internal(&mut self, _name: &str) -> Status<List> {
        todo!("Internals @ <exec.rs> exec_internal");
    }

    #[inline]
    fn exec_function<'a>(
        &mut self,
        f: Function,
        line: &mut [Node<'a>],
        i: &mut usize,
    ) -> Status<List> {
        if self.right.is_none() {
            return err!(Argument);
        }
        let right = self.right.take().unwrap();
        if *i > 0 && utils::is_instant(&line[*i - 1], &self.scope) {
            let left = self.exec_instant(take(&mut line[*i - 1]))?;
            *i -= 1;
            f.dyadic_call(left, right)
        } else {
            f.monadic_call(right)
        }
    }

    #[inline]
    fn exec_operator<'a>(
        &mut self,
        o: Operator,
        l: Node<'a>,
        r: Option<Node<'a>>,
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
            return Ok(Function::scalar(
                List::empty(),
                self.arena.clone(),
                self.scope.clone(),
            ));
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
                let topr = top.clone();
                let left = self.exec_instant(n)?;
                let leftr = left.clone();
                if build.is_empty() {
                    build.push(function!(
                        move |a, _| top.dyadic_call(left.clone(), a);
                        move |_, b, _| topr.dyadic_call(leftr.clone(), b);
                        self.arena.clone(), self.scope.clone()
                    ));
                } else {
                    let rightfn = build.pop().unwrap();
                    let rightfnr = rightfn.clone();
                    build.push(function!(
                        move |a, _| {
                            top.dyadic_call(left.clone(), rightfn.monadic_call(a)?)
                        };
                        move |a, b, _| {
                            topr.dyadic_call(leftr.clone(), rightfnr.dyadic_call(a, b)?)
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
                f = self
                    .scope
                    .borrow()
                    .get(n)
                    .ok_or(Error::Scope)?
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
                    let fr = f.clone();
                    let g = build.pop().unwrap();
                    let gr = g.clone();
                    build.push(function!(
                        move |a, _| f.monadic_call(g.monadic_call(a)?);
                        move |a, b, _| fr.monadic_call(gr.dyadic_call(a, b)?);
                        self.arena.clone(), self.scope.clone()
                    ));
                }
            } else {
                if build.len() == 2 {
                    let g = build.pop().unwrap();
                    let h = build.pop().unwrap();

                    let fr = f.clone();
                    let gr = g.clone();
                    let hr = h.clone();
                    build.push(function!(
                        move |a, _| {
                            g.dyadic_call(f.monadic_call(a.clone())?, h.monadic_call(a)?)
                        };
                        move |a, b, _| {
                            gr.dyadic_call(
                                fr.dyadic_call(a.clone(), b.clone())?,
                                hr.dyadic_call(a, b)?,
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
            let fr = f.clone();
            let gr = g.clone();
            Ok(function!(
                move |a, _| f.monadic_call(g.monadic_call(a)?);
                move |a, b, _| fr.monadic_call(gr.dyadic_call(a, b)?);
                self.arena.clone(), self.scope.clone()
            ))
        } else {
            build.pop().ok_or(Error::Syntax("Empty expression.".into()))
        }
    }

    #[inline]
    fn exec_assignment<'a>(
        &mut self,
        head: &'a str,
        body: Vec<Node<'a>>,
        is_internal: bool,
        preserve_instant: bool,
    ) -> Status {
        if is_internal {
            todo!("Internals @ <exec.rs> exec_line");
        }
        let group = Node::Group(body);
        if utils::is_instant(&group, &self.scope) {
            let s = self.scope.clone();
            let list = self.exec_instant(group)?;
            if preserve_instant {
                // Preserves when used
                self.right = Some(list.clone());
            }
            s.borrow_mut().set_instant(head, list);
        } else {
            // SAFETY: This is set as a `Node::Group` a few lines ago
            let Node::Group(mut body) = group else {
                unsafe { unreachable_unchecked() }
            };
            if body.len() == 1
                && let Node::Function(f) = std::mem::take(&mut body[0])
            {
                let s = self.scope.clone();
                s.borrow_mut().set_function(head, f);
            } else {
                let f = self.exec_train(body)?;
                let s = self.scope.clone();
                s.borrow_mut().set_function(head, f);
            }
        }

        Ok(())
    }

    #[inline]
    fn exec_tag(&mut self, tag: &str) -> Status {
        match tag {
            // Silent
            "~" => self.right = None,
            // Get type
            "?" => {
                let right = self.right.take().ok_or(Error::Value)?;
                if right.is_empty() {
                    self.right = Some(List::string("()"));
                } else {
                    self.right = Some(match right.list()[0] {
                        Scalar::Number(_) | Scalar::Float(_) => List::string("N"),
                        Scalar::Boolean(_) => List::string("B"),
                        Scalar::Char(_) => List::string("S"),
                        Scalar::Box(_) => List::string("B"),
                    })
                }
            }
            _ => return err!(Syntax, "Unrecognized tag."),
        }

        Ok(())
    }

    #[inline]
    fn maybe_instant<'a>(&mut self, node: Node<'a>) -> Status<MaybeInstant> {
        if utils::is_instant(&node, &self.scope) {
            Ok(MaybeInstant::Instant(self.exec_instant(node)?))
        } else if let Node::Function(f) = node {
            Ok(MaybeInstant::Function(f))
        } else if let Node::Op(o, l, r) = node {
            Ok(MaybeInstant::Function(self.exec_operator(o, *l, *r)?))
        } else if let Node::Group(g) = node {
            Ok(MaybeInstant::Function(self.exec_train(g)?))
        } else {
            unreachable!("Internal error @ src/exec.rs:maybe_instant");
        }
    }
}
