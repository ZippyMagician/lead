use std::fmt::{self, Debug};
use std::iter::Peekable;
use std::slice::Iter;

use crate::builtins::operators::{self, Operator};
use crate::builtins::symbols;
use crate::function::Function;
use crate::lex::{Token, TokenTag};
use crate::scope::{Scope, ScopeInternal};
use crate::types::list::List;
use crate::types::scalar::Scalar;
use crate::utils::error::{Error, Status, err};
use crate::utils::number::*;
use crate::utils::{Arena, generate_block};

use ecow::EcoVec;

#[derive(Clone)]
pub enum Assignment<'a> {
    Literal(&'a str),
    Accessor(&'a str, Vec<&'a str>),
    Internal(&'a str),
}

#[derive(Clone, Default)]
pub enum Node<'a> {
    /// Primitive list of numerics
    Primitive(List),
    /// Primitive string (list of chars)
    PrimitiveString(List),
    /// Primitive list of boxed strings
    PrimitiveBoxes(List),
    /// Literal identifier (variable)
    Literal(&'a str),
    /// Internal identifier
    LiteralInternal(&'a str),
    /// Tag (bracketed word/symbol)
    Tag(Token<'a>),
    /// Map
    Map(Vec<(&'a str, Vec<Node<'a>>)>),
    /// Accessor
    Accessor(&'a str, Vec<&'a str>),
    /// Any function
    Function(Function),
    /// Parenthesized group of nodes
    Group(Vec<Node<'a>>),
    /// Operator waiting to be passed arguments
    MonadicOp(Operator),
    DyadicOp(Operator),
    /// Full operator
    Op(Operator, Box<Node<'a>>, Box<Option<Node<'a>>>),
    /// Raw colon identifier
    Colon,
    /// Assignment
    Assign {
        head: Assignment<'a>,
        body: Vec<Node<'a>>,
    },
    /// If conditional
    If {
        cond: Box<Node<'a>>,
        body: Vec<Node<'a>>,
        otherwise: Vec<Node<'a>>,
    },
    /// Lazily evaluated node -- for returning from definitions
    Lazy(Box<Node<'a>>),
    /// New line
    #[default]
    Line,
    // Yield two nodes at once
    Two(Box<Node<'a>>, Box<Node<'a>>),
}

pub struct Parser<'a> {
    nodes: Vec<Node<'a>>,
    tokens: Peekable<Iter<'a, Token<'a>>>,

    // State
    upto_store: Vec<TokenTag>,
    arena: Arena,
    scope: Scope,
}

impl<'a> Parser<'a> {
    #[inline]
    pub fn new(arena: Arena, scope: Scope, l: &'a [Token<'a>]) -> Self {
        Self { nodes: vec![], tokens: l.iter().peekable(), upto_store: vec![], arena, scope }
    }

    #[inline]
    pub fn insert_tokens(&mut self, l: &'a [Token<'a>]) {
        self.tokens = l.iter().peekable();
    }

    #[inline(always)]
    fn unpack_node(nodes: &mut Vec<Node<'a>>, node: Status<Node<'a>>) -> Status {
        match node {
            Ok(Node::Two(l, r)) => {
                nodes.push(*l);
                nodes.push(*r);
            }
            Ok(node) => nodes.push(node),
            Err(Error::Complete) => {}
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[inline]
    pub fn parse(&mut self) -> Status {
        while let Some(token) = self.tokens.next() {
            let status = self.consume_node(token);
            Self::unpack_node(&mut self.nodes, status)?;
        }
        self.nodes.push(Node::Line);
        self.second_pass()?;
        Ok(())
    }

    #[inline(always)]
    pub fn nodes(self) -> Vec<Node<'a>> {
        self.nodes
    }

    #[inline(always)]
    pub fn borrow_nodes(&self) -> &[Node<'a>] {
        &self.nodes
    }

    pub fn consume_node(&mut self, t: &'a Token<'a>) -> Status<Node<'a>> {
        use TokenTag::*;

        match t.0 {
            Number => self.consume_primitive(Scalar::Number(parse_number(t.1)?)),
            Float => self.consume_primitive(Scalar::Float(parse_float(t.1)?)),
            String => self.consume_primitive_string(t.1),
            Symbol | CloseBracket => {
                let f = symbols::Symbol::get_symbol(t.1)
                    .unwrap()
                    .getfn(self.arena.clone(), self.scope.clone());
                Ok(Node::Function(f))
            }
            Operator => {
                let o = operators::Operator::get_operator(t.1).unwrap();
                if o.is_monadic() { Ok(Node::MonadicOp(o)) } else { Ok(Node::DyadicOp(o)) }
            }
            Literal => self.maybe_accessor(t),
            Newline | CloseParen | Tab => Ok(Node::Line),
            OpenBracket => self.maybe_tag(),
            OpenParen => self.consume_group(),
            OpenCurly => self.consume_block(TokenTag::CloseCurly),
            Colon => self.maybe_assign(),
            EOF => err!(Complete),
            _ => err!(Syntax, format!("Unexpected token {:?}.", t.1)),
        }
    }

    fn maybe_tag(&mut self) -> Status<Node<'a>> {
        let sym =
            symbols::Symbol::get_symbol("[").unwrap().getfn(self.arena.clone(), self.scope.clone());

        if let Some(next) = self.tokens.next() {
            match next {
                (TokenTag::CloseBracket, _) => self.consume_internal(),
                (TokenTag::Newline, _) => {
                    Ok(Node::Two(Node::Function(sym).into(), Node::Line.into()))
                }
                (t, tag) if self.next_if(TokenTag::CloseBracket).is_some() => {
                    self.consume_tag(*t, tag)
                }
                next => Ok(Node::Two(Node::Function(sym).into(), self.consume_node(next)?.into())),
            }
        } else {
            Ok(Node::Function(sym))
        }
    }

    fn maybe_assign(&mut self) -> Status<Node<'a>> {
        if self.nodes.is_empty() {
            return Ok(Node::Colon);
        }

        let top = unsafe { self.nodes.pop().unwrap_unchecked() };
        match top {
            Node::Literal(s) => self.consume_assign(s, false),
            Node::LiteralInternal(s) => self.consume_assign(s, true),
            Node::Tag(_) | Node::Function(_) | Node::If { .. } | Node::Line | Node::Colon => {
                err!(Syntax, "Invalid use of colon")
            }
            Node::Accessor(head, body) => {
                self.consume_assign_accessor(Assignment::Accessor(head, body))
            }
            _ => self.consume_if(top),
        }
    }

    fn maybe_accessor(&mut self, head: &'a Token<'a>) -> Status<Node<'a>> {
        if self.next_if(TokenTag::Dot).is_some() {
            let mut build = Vec::new();
            loop {
                // TODO: Support variadic accessing?
                if let Some(child) = self.next_if(TokenTag::Literal) {
                    build.push(child.1);
                } else {
                    return err!(Syntax, "Empty accessor");
                }
                if self.next_if(TokenTag::Dot).is_none() {
                    break;
                }
            }
            Ok(Node::Accessor(head.1, build))
        } else {
            Ok(Node::Literal(head.1))
        }
    }

    fn consume_primitive(&mut self, s: Scalar) -> Status<Node<'a>> {
        let mut tokens = vec![s];
        while let Some((tag, s)) =
            self.tokens.next_if(|(t, _)| *t == TokenTag::Number || *t == TokenTag::Float)
        {
            if *tag == TokenTag::Number {
                tokens.push(Scalar::Number(parse_number(s)?));
            } else {
                tokens.push(Scalar::Float(parse_float(s)?));
            }
        }

        if tokens.len() == 1 {
            Ok(Node::Primitive(List::scalar(tokens[0])))
        } else {
            List::from_vec(tokens.into()).map(Node::Primitive)
        }
    }

    fn consume_primitive_string(&mut self, t: &'a str) -> Status<Node<'a>> {
        let mut strings = vec![t];
        while let Some((_, s)) = self.next_if(TokenTag::String) {
            strings.push(s);
        }

        if strings.len() == 1 {
            Ok(Node::PrimitiveString(List::string(strings[0])))
        } else {
            List::from_vec(
                strings
                    .iter()
                    .map(|s| Scalar::boxed(&self.arena, List::string(s)))
                    .collect::<Status<EcoVec<Scalar>>>()?,
            )
            .map(Node::PrimitiveBoxes)
        }
    }

    fn consume_assign(&mut self, head: &'a str, is_internal: bool) -> Status<Node<'a>> {
        let body = self.consume_upto_tag(TokenTag::Newline)?;
        Ok(Node::Assign {
            head: if is_internal { Assignment::Internal(head) } else { Assignment::Literal(head) },
            body,
        })
    }

    fn consume_assign_accessor(&mut self, head: Assignment<'a>) -> Status<Node<'a>> {
        let body = self.consume_upto_tag(TokenTag::Newline)?;
        Ok(Node::Assign { head, body })
    }

    fn consume_if(&mut self, mut cond: Node<'a>) -> Status<Node<'a>> {
        let body = self.consume_upto(|n| matches!(n, Node::Line))?;
        let otherwise = self.consume_upto(|n| matches!(n, Node::Line))?;
        // Unwrap singleton conditions
        if let Node::Group(g) = &mut cond
            && g.len() == 1
        {
            cond = std::mem::take(&mut g[0]);
        }
        Ok(Node::Two(Node::If { cond: cond.into(), body, otherwise }.into(), Node::Line.into()))
    }

    fn consume_tag(&mut self, ident: TokenTag, tag: &'a str) -> Status<Node<'a>> {
        if tag == "{" {
            // block: [{] x + y
            self.consume_block(TokenTag::Newline)
        } else if ident == TokenTag::Colon {
            // map: [:] { a: 1; b: 2 }
            self.consume_map()
        } else {
            Ok(Node::Tag((ident, tag)))
        }
    }

    #[inline]
    fn consume_internal(&mut self) -> Status<Node<'a>> {
        if let Some((_, lit)) = self.next_if(TokenTag::Literal) {
            Ok(Node::LiteralInternal(lit))
        } else {
            err!(Syntax, "Invalid use of internal indicator.")
        }
    }

    #[inline]
    fn consume_group(&mut self) -> Status<Node<'a>> {
        let nodes = self.consume_upto_tag(TokenTag::CloseParen)?;
        self.tokens.next(); // consume_upto does not consume the closing token
        Ok(Node::Group(nodes))
    }

    #[inline]
    fn consume_block(&mut self, end: TokenTag) -> Status<Node<'a>> {
        let nodes = self.consume_upto_tag(end)?;
        if end != TokenTag::Newline {
            self.tokens.next();
        }
        Ok(Node::Function(generate_block(
            nodes,
            ScopeInternal::child(self.scope.clone()),
            self.arena.clone(),
        )))
    }

    #[inline]
    fn consume_map(&mut self) -> Status<Node<'a>> {
        if self.tokens.next_if(|(t, c)| *t == TokenTag::Symbol && *c == "{").is_none() {
            return err!(Syntax, "Expected '{'");
        }

        let mut body = Vec::new();
        while let Some(tok) = self.tokens.next_if(|(t, c)| *t != TokenTag::Symbol || *c != "}") {
            if let (TokenTag::Literal, name) = tok {
                if self.tokens.next_if(|(t, _)| *t == TokenTag::Colon).is_none() {
                    return err!(Syntax, "Expected ':'");
                }
                let mut nodes = Vec::new();
                std::mem::swap(&mut nodes, &mut self.nodes);
                while let Some(tok) =
                    self.tokens.next_if(|(t, c)| *t != TokenTag::Symbol || *c != ";" && *c != "}")
                {
                    let node = self.consume_node(tok);
                    Self::unpack_node(&mut self.nodes, node)?;
                }
                self.tokens.next_if(|(_, c)| *c == ";");
                self.second_pass()?;
                std::mem::swap(&mut nodes, &mut self.nodes);
                body.push((*name, nodes));
            } else {
                return err!(Syntax, "Unexpected token in map");
            }
        }
        self.tokens.next();

        Ok(Node::Map(body))
    }

    #[inline]
    fn next_if(&mut self, n: TokenTag) -> Option<&'a Token<'a>> {
        self.tokens.next_if(|(t, _)| *t == n)
    }

    // Swaps nodes temporarily to support certain parsing
    #[inline]
    fn consume_upto_tag(&mut self, tag: TokenTag) -> Status<Vec<Node<'a>>> {
        self.upto_store.push(tag);
        let mut nodes = Vec::new();
        std::mem::swap(&mut nodes, &mut self.nodes);
        while let Some(tok) = self.tokens.next_if(|(t, _)| self.upto_store.iter().all(|g| *g != *t))
        {
            let node = self.consume_node(tok);
            Self::unpack_node(&mut self.nodes, node)?;
        }
        self.upto_store.pop();
        self.second_pass()?;
        std::mem::swap(&mut nodes, &mut self.nodes);
        Ok(nodes)
    }

    #[inline]
    fn consume_upto(&mut self, check: impl Fn(&Node<'a>) -> bool) -> Status<Vec<Node<'a>>> {
        let mut nodes = Vec::new();
        std::mem::swap(&mut nodes, &mut self.nodes);
        while let Some(tok) =
            self.tokens.next_if(|(t, _)| self.upto_store.last().is_none_or(|g| *g != *t))
        {
            let node = self.consume_node(tok);
            Self::unpack_node(&mut self.nodes, node)?;
            if self.nodes.is_empty()
                || check(unsafe { self.nodes.get_unchecked(self.nodes.len() - 1) })
            {
                self.nodes.pop();
                break;
            }
        }
        self.second_pass()?;
        std::mem::swap(&mut nodes, &mut self.nodes);
        Ok(nodes)
    }

    fn second_pass(&mut self) -> Status {
        let mut i = 0;
        'outer: while i < self.nodes.len() {
            let node = &self.nodes[i];
            match node {
                // Unwrap singleton conditions
                Node::Group(g) if g.is_empty() => {}
                Node::Group(g) if g.len() == 1 => {
                    let Node::Group(mut g) = std::mem::take(&mut self.nodes[i]) else {
                        unsafe { std::hint::unreachable_unchecked() }
                    };
                    self.nodes[i] = std::mem::take(&mut g[0]);
                }
                Node::Group(g) => {
                    for j in 1..g.len() {
                        if !matches!(g[j], Node::Line) {
                            i += 1;
                            continue 'outer;
                        }
                    }
                    let Node::Group(mut g) = std::mem::take(&mut self.nodes[i]) else {
                        unsafe { std::hint::unreachable_unchecked() }
                    };
                    self.nodes[i] = std::mem::take(&mut g[0]);
                }
                Node::MonadicOp(_) => {
                    if i == 0 {
                        return err!(Syntax, "Missing operator parameter.");
                    }
                    let last = self.nodes.remove(i - 1);
                    let Node::MonadicOp(o) = std::mem::take(&mut self.nodes[i - 1]) else {
                        unsafe { std::hint::unreachable_unchecked() }
                    };

                    self.nodes[i - 1] = Node::Op(o, Box::new(last), Box::new(None));
                    i -= 1;
                }
                Node::DyadicOp(_) => {
                    if i == 0 || i == self.nodes.len() - 1 {
                        return err!(Syntax, "Missing operator parameter.");
                    }
                    let mut last = self.nodes.remove(i - 1);
                    let mut next = self.nodes.remove(i);
                    // Swap dyadic operand arguments
                    if let Node::MonadicOp(o) = next
                        && o.get_name() == b"`"
                    {
                        next = self.nodes.remove(i);
                        std::mem::swap(&mut last, &mut next);
                    }

                    let Node::DyadicOp(o) = std::mem::take(&mut self.nodes[i - 1]) else {
                        unsafe { std::hint::unreachable_unchecked() }
                    };
                    self.nodes[i - 1] = Node::Op(o, Box::new(last), Box::new(Some(next)));
                    i -= 1;
                }
                _ => {}
            }

            i += 1;
        }

        Ok(())
    }
}

impl<'a> fmt::Display for Assignment<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(l) => write!(f, "{l}"),
            Self::Internal(l) => write!(f, "[]{l}"),
            Self::Accessor(h, b) => {
                write!(f, "{h}")?;
                for i in b {
                    write!(f, ".{i}")?;
                }
                Ok(())
            }
        }
    }
}

impl<'a> Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Primitive(l) => write!(f, "{l:?}"),
            Node::PrimitiveString(l) => write!(f, "{l:?}"),
            Node::PrimitiveBoxes(l) => write!(f, "{l:?}"),
            Node::Literal(l) => write!(f, "Lit({l})"),
            Node::LiteralInternal(l) => write!(f, "Internal([]{l})"),
            Node::Function(_) => write!(f, "Function"),
            Node::MonadicOp(_) => write!(f, "Adverb"),
            Node::DyadicOp(_) => write!(f, "Conjunction"),
            Node::Op(o, l, r) => write!(f, "({l:?}) {o:?} ({r:?})"),
            Node::Line => write!(f, "NL"),
            Node::Tag(t) => write!(f, "[{t:?}]"),
            Node::Group(g) => write!(f, "({g:?})"),
            Node::If { cond, body, otherwise } => {
                write!(f, "IF({cond:?}){{{{{body:?}}}}}ELSE{{{{{otherwise:?}}}}}")
            }
            Node::Assign { head, body } => {
                write!(f, "{head} : {body:?}",)
            }
            Node::Map(keys) => write!(f, "Map({keys:?})"),
            Node::Accessor(head, body) => {
                write!(f, "{head}")?;
                for item in body {
                    write!(f, ".{item}")?;
                }
                Ok(())
            }
            _ => todo!(),
        }
    }
}
