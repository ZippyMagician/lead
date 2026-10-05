use crate::list::List;
use crate::parse::Node;
use crate::scope::Scope;
use crate::utils::error::{Error, Status, err};
use crate::utils::{self, Arena};

pub struct Exec {
    scope: Scope,
    _arena: Arena,

    right: Option<List>,
}

impl Exec {
    pub fn new(arena: Arena, scope: Scope) -> Self {
        Self {
            scope,
            _arena: arena,
            right: None,
        }
    }

    pub fn exec<'a>(&mut self, node_lines: Vec<Node<'a>>) -> Status {
        let mut current = Vec::new();
        for node in node_lines {
            if matches!(node, Node::Line) {
                self.exec_line(std::mem::take(&mut current))?;
                if let Some(v) = &self.right {
                    println!("{v:?}");
                }
            } else {
                current.push(node);
            }
        }
        Ok(())
    }

    #[inline]
    fn exec_line<'a>(&mut self, mut line: Vec<Node<'a>>) -> Status {
        use Node::*;

        let mut i = line.len() - 1;
        loop {
            match std::mem::take(unsafe { line.get_unchecked_mut(i) }) {
                Primitive(list) => self.right = Some(list),
                PrimitiveString(list) => self.right = Some(list),
                PrimitiveBoxes(list) => self.right = Some(list),
                Literal(l) => {
                    let s = self.scope.clone();
                    let value = s.borrow().get(l).ok_or(Error::Domain)?;
                    if value.is_instant() {
                        self.right = Some(value.as_instant().unwrap());
                    } else {
                        let _f = value.as_function().unwrap();
                        todo!("Non instants @ <exec.rs> exec_line");
                    }
                }
                LiteralInternal(_) => todo!("Internals @ <exec.rs> exec_line"),
                Tag(_) => todo!("Tags @ <exec.rs> exec_line"),
                Function(f) => {
                    if self.right.is_none() {
                        return err!(Argument);
                    }

                    let right = self.right.take().unwrap();
                    if i > 0 && utils::is_instant(&line[i - 1], self.scope.clone()) {
                        let left = self.exec_instant(std::mem::take(&mut line[i - 1]))?;
                        i -= 1;
                        self.right = Some(f.dyadic_call(left, right)?);
                    } else {
                        self.right = Some(f.monadic_call(right)?);
                    }
                }
                Assign {
                    head,
                    is_internal,
                    body,
                } => {
                    if is_internal {
                        todo!("Internals @ <exec.rs> exec_line");
                    }
                    let group = Node::Group(body);
                    if utils::is_instant(&group, self.scope.clone()) {
                        let s = self.scope.clone();
                        let list = self.exec_instant(group)?;
                        s.borrow_mut().set_instant(head, list);
                    } else {
                        todo!("Non-instant assignment @ <exec.rs> exec_line");
                    }
                }
                node @ _ => todo!("Execute node {node:?}"),
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
}

impl<'a> std::default::Default for Node<'a> {
    fn default() -> Self {
        Self::Line
    }
}
