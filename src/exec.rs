use std::hint::unreachable_unchecked;
use std::mem::take;

use crate::function::Function;
use crate::list::List;
use crate::parse::Node;
use crate::scalar::Scalar;
use crate::scope::Scope;
use crate::utils::error::{Error, Status, err};
use crate::utils::{self, Arena};

pub struct Exec {
    scope: Scope,
    arena: Arena,

    pub(crate) inner: bool,
    pub(crate) right: Option<List>,
}

impl Exec {
    #[inline]
    pub fn new(arena: Arena, scope: Scope) -> Self {
        Self {
            scope,
            arena: arena,
            inner: false,
            right: None,
        }
    }

    #[inline]
    pub fn new_imm(arena: Arena, scope: Scope) -> Self {
        Self {
            scope,
            arena: arena,
            inner: true,
            right: None,
        }
    }

    pub fn exec<'a>(&mut self, mut node_lines: Vec<Node<'a>>) -> Status {
        let mut current = Vec::new();
        node_lines.push(Node::Line);
        for node in node_lines {
            if matches!(node, Node::Line) {
                self.exec_line(take(&mut current))?;
                if !self.inner
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
                Assign {
                    head,
                    is_internal,
                    body,
                } => self.exec_assignment(
                    head,
                    body,
                    is_internal,
                    i > 0 && !utils::is_instant(&line[i - 1], self.scope.clone()),
                )?,
                node => todo!("Execute node {node:?}"),
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
            Literal(l) => self.scope.borrow().get(l).unwrap().as_instant(),
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
                self.exec_line(body)?;
                Ok(self.right.take().unwrap_or(List::empty()))
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
        let value = s.borrow().get(name).ok_or(Error::Scope)?;
        if value.is_instant() {
            Ok(value.as_instant().unwrap())
        } else {
            let f = value
                .as_function(self.arena.clone(), self.scope.clone())
                .unwrap();
            self.exec_function(f, list, i)
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
        if *i > 0 && utils::is_instant(&line[*i - 1], self.scope.clone()) {
            let left = self.exec_instant(take(&mut line[*i - 1]))?;
            *i -= 1;
            f.dyadic_call(left, right)
        } else {
            f.monadic_call(right)
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
        if utils::is_instant(&group, self.scope.clone()) {
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
                todo!("Trains @ <exec.rs> exec_line");
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
}
