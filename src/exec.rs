use std::fmt::{self, Display};
use std::hint::unreachable_unchecked;
use std::mem::take;

use crate::builtins::operators::Operator;
use crate::function::{Function, function};
use crate::list::List;
use crate::map::{self, Map};
use crate::parse::Node;
use crate::scalar::Scalar;
use crate::scope::{MaybeInstant, Scope};
use crate::utils::error::{Error, Status, err};
use crate::utils::{self, Arena};

use smallvec::{SmallVec, smallvec};

pub struct Exec {
    scope: Scope,
    arena: Arena,

    pub silent: bool,
    pub(crate) block: bool,
    pub(crate) reference: Option<Vec<Node<'static>>>,
    pub(crate) right: Option<Value>,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum Value {
    List(List),
    Map(Map),
}

impl Value {
    #[inline]
    pub fn to_list(self) -> Status<List> {
        match self {
            Self::List(l) => Ok(l),
            Self::Map(_) => err!(ValueS, "Cannot treat map as primitive list."),
        }
    }

    #[inline]
    pub fn expect_list(self) -> Status<List> {
        match self {
            Self::List(l) => Ok(l),
            Self::Map(_) => {
                std::hint::cold_path();
                err!(Value)
            }
        }
    }

    #[inline]
    pub unsafe fn to_list_unchecked(self) -> List {
        match self {
            Self::List(l) => l,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    #[inline]
    pub fn to_map(self) -> Status<Map> {
        match self {
            Self::List(_) => err!(ValueS, "Cannot treat primitive list as map."),
            Self::Map(m) => Ok(m),
        }
    }

    #[inline]
    pub fn as_map(&self) -> Status<&Map> {
        match &self {
            Self::List(_) => err!(ValueS, "Cannot treat primitive list as map."),
            Self::Map(m) => Ok(m),
        }
    }

    #[inline]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    #[inline]
    pub fn is_list(&self) -> bool {
        matches!(self, Self::List(_))
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::List(l) => l.is_empty(),
            Self::Map(m) => m.is_empty(),
        }
    }
}

impl Exec {
    #[inline]
    pub fn new(arena: Arena, scope: Scope) -> Self {
        Self {
            scope,
            arena: arena,
            silent: false,
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
            silent: false,
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
                Primitive(list) => self.right = Some(Value::List(list)),
                PrimitiveString(list) => self.right = Some(Value::List(list)),
                PrimitiveBoxes(list) => self.right = Some(Value::List(list)),
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
                If {
                    cond,
                    body,
                    otherwise,
                } => self.exec_if(cond, body, otherwise, &mut line, &mut i)?,
                n @ Group(_) if utils::is_instant(&n, &self.scope) => {
                    self.right = Some(self.exec_instant(n)?)
                }
                Group(g) => {
                    let train = self.exec_train(g)?;
                    self.right = Some(self.exec_function(train, &mut line, &mut i)?);
                }
                n @ (Map(_) | Accessor(_, _)) => self.right = Some(self.exec_instant(n)?),
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
    fn exec_instant<'a>(&mut self, node: Node<'a>) -> Status<Value> {
        use Node::*;

        match node {
            Primitive(l) => Ok(Value::List(l)),
            PrimitiveString(l) => Ok(Value::List(l)),
            PrimitiveBoxes(l) => Ok(Value::List(l)),
            Literal(l) => self.scope.borrow().get(l).unwrap().to_value(),
            LiteralInternal(_) => err!(Internal, "TODO: Internals @ <exec.rs> exec_instant"),
            Assign {
                head,
                is_internal,
                body,
            } => {
                if is_internal {
                    return err!(Internal, "Internals @ <exec.rs> exec_instant");
                }
                let s = self.scope.clone();
                let list = self.exec_instant(Node::Group(body))?;
                s.borrow_mut().set_value(head, list.clone());
                Ok(list)
            }
            Group(body) => {
                let r = self.right.take();
                self.exec_line(body)?;
                let out = self.right.take();
                self.right = r;
                Ok(out.unwrap_or(Value::List(List::empty())))
            }
            If {
                cond,
                body,
                otherwise,
            } => {
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
                for (key, value) in m {
                    map.insert(
                        key.to_string(),
                        self.exec_instant(Node::Group(value))?,
                        &self.arena,
                    )?;
                }
                Ok(Value::Map(map))
            }
            Accessor(head, body) => {
                let s = self.scope.borrow();
                let map = s.get(head).ok_or(Error::Scope)?.to_map()?;
                drop(s);
                let mut acc = &map;
                if body.len() > 1 {
                    for k in 0..=body.len() - 2 {
                        acc = acc.get(&body[k]).ok_or(Error::Domain)?.as_map()?;
                    }
                }
                acc.get(&body.last().unwrap()).cloned().ok_or(Error::Domain)
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
    ) -> Status<Value> {
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
                Ok(Value::List(List::empty()))
            } else {
                let f = utils::generate_block(
                    self.reference.take().unwrap(),
                    self.scope.clone(),
                    self.arena.clone(),
                );
                // TODO: Check for maps, support alternate calls for blocks
                if let Some(left) = left {
                    f.dyadic_call(left, right)
                } else {
                    f.monadic_call(right)
                }
            }
        } else {
            let value = s.borrow().get(name).ok_or(Error::Scope)?;
            if value.is_function() {
                let f = value
                    .to_function(self.arena.clone(), self.scope.clone())
                    .unwrap();
                self.exec_function(f, list, i)
            } else {
                value.to_value()
            }
        }
    }

    #[inline]
    fn exec_internal(&mut self, _name: &str) -> Status<Value> {
        err!(Internal, "TODO: Internals @ <exec.rs> exec_internal")
    }

    #[inline]
    fn exec_function<'a>(
        &mut self,
        f: Function,
        line: &mut [Node<'a>],
        i: &mut usize,
    ) -> Status<Value> {
        if self.right.is_none() {
            return err!(Argument);
        }
        let right = self.right.take().unwrap();
        // TODO: Check is value is map, support alternate block calls
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
                        move |a, b, _| f.monadic_call(g.call_fn(a, b)?);
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
        &mut self,
        head: &'a str,
        body: Vec<Node<'a>>,
        is_internal: bool,
        preserve_instant: bool,
    ) -> Status {
        if is_internal {
            return err!(Internal, "TODO: Internals @ <exec.rs> exec_assignment");
        }
        let group = Node::Group(body);
        if utils::is_instant(&group, &self.scope) {
            let s = self.scope.clone();
            let list = self.exec_instant(group)?;
            if preserve_instant {
                // Preserves when used
                self.right = Some(list.clone());
            }
            s.borrow_mut().set_value(head, list);
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
    fn exec_if<'a>(
        &mut self,
        cond: Box<Node<'a>>,
        body: Vec<Node<'a>>,
        otherwise: Vec<Node<'a>>,
        line: &mut Vec<Node<'a>>,
        i: &mut usize,
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
    fn exec_tag<'a>(&mut self, tag: crate::lex::Token<'a>) -> Status {
        use crate::lex::TokenTag;

        match tag {
            // Silent
            (TokenTag::Symbol, "~") => self.right = None,
            // Get type
            (TokenTag::Symbol, "?") => {
                let right = self.right.take().ok_or(Error::Argument)?;
                if right.is_map() {
                    self.right = Some(Value::List(List::string("`:")));
                } else {
                    let right = unsafe { right.to_list_unchecked() };
                    if right.is_empty() {
                        self.right = Some(Value::List(List::string("()")));
                        return Ok(());
                    }
                    self.right = Some(Value::List(match right.list()[0] {
                        Scalar::Number(_) | Scalar::Float(_) => List::string("`N"),
                        Scalar::Boolean(_) => List::string("`B"),
                        Scalar::Char(_) => List::string("`C"),
                        Scalar::Box(_) => List::string("`X"),
                    }))
                }
            }
            (TokenTag::Symbol, ">") => {
                let right = self.right.take().ok_or(Error::Argument)?;
                if right.is_empty() || right.is_map() {
                    return err!(Value);
                }
                let right = unsafe { right.to_list_unchecked() };
                self.right = Some(unsafe { right.list()[0].steal_box() });
            }
            (TokenTag::String, s) if s.len() == 1 => {
                let mut right = self.right.take().ok_or(Error::Value)?.to_list()?;
                if right.is_empty() {
                    self.right = Some(Value::List(right));
                } else {
                    let ident = s.as_bytes()[0] as char;
                    crate::utils::symbols::for_0!(right; [v] = v.to_cast(ident)?);
                    self.right = Some(Value::List(right));
                }
            }
            _ => return err!(Syntax, "Unrecognized tag."),
        }

        Ok(())
    }

    #[inline]
    fn maybe_instant<'a>(&mut self, node: Node<'a>) -> Status<MaybeInstant> {
        if utils::is_instant(&node, &self.scope) {
            Ok(MaybeInstant::from(self.exec_instant(node)?))
        } else if let Node::Function(f) = node {
            Ok(MaybeInstant::Function(f))
        } else if let Node::Op(o, l, r) = node {
            Ok(MaybeInstant::Function(self.exec_operator(o, *l, *r)?))
        } else if let Node::Group(g) = node {
            Ok(MaybeInstant::Function(self.exec_train(g)?))
        } else {
            err!(Syntax, "Unsupported node provided to operator")
        }
    }
}

impl From<Map> for Value {
    #[inline(always)]
    fn from(m: Map) -> Self {
        Self::Map(m)
    }
}

impl From<List> for Value {
    #[inline(always)]
    fn from(l: List) -> Self {
        Self::List(l)
    }
}

impl Default for Value {
    #[inline(always)]
    fn default() -> Self {
        Self::List(List::default())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List(l) => write!(f, "{l}"),
            Self::Map(m) => write!(f, "{m}"),
        }
    }
}
