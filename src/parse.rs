use std::fmt::{self, Debug};
use std::iter::Peekable;
use std::slice::Iter;

use crate::builtins::symbols;
use crate::function::Function;
use crate::lex::{Token, TokenTag};
use crate::list::List;
use crate::scalar::Scalar;
use crate::scope::Scope;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err};
use crate::utils::number::*;

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
    Tag(&'a str),
    /// Any function
    Function(Function),
    /// Parenthesized group of nodes
    Group(Vec<Node<'a>>),
    /// Raw colon identifier
    Colon,
    /// Assignment
    Assign {
        head: &'a str,
        is_internal: bool,
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
    arena: Arena,
    #[allow(dead_code)]
    scope: Scope,
}

impl<'a> Parser<'a> {
    pub fn new(arena: Arena, scope: Scope, l: &'a [Token<'a>]) -> Self {
        Self {
            nodes: vec![],
            tokens: l.iter().peekable(),
            arena,
            scope,
        }
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
        Ok(())
    }

    #[inline(always)]
    pub fn nodes(self) -> Vec<Node<'a>> {
        self.nodes
    }

    pub fn consume_node(&mut self, t: &'a Token<'a>) -> Status<Node<'a>> {
        use TokenTag::*;

        match t.0 {
            Number => self.consume_primitive(Scalar::Number(parse_number(t.1)?)),
            Float => self.consume_primitive(Scalar::Float(parse_float(t.1)?)),
            String => self.consume_primitive_string(t.1),
            Symbol => Ok(Node::Function(
                symbols::Symbol::get_symbol(t.1)
                    .ok_or(Error::Syntax("Unrecognized symbol."))?
                    .getfn(),
            )),
            Literal => Ok(if t.1.ends_with('.') {
                Node::LiteralInternal(t.1.trim_end_matches('.'))
            } else {
                Node::Literal(t.1)
            }),
            Newline | CloseParen => Ok(Node::Line),
            OpenBracket => self.maybe_tag(),
            OpenParen => self.consume_group(),
            OpenCurly => self.consume_block(),
            Colon => self.maybe_assign(),
            EOF => err!(Complete),
            _ => err!(Syntax, "Unrecognized token."),
        }
    }

    fn maybe_tag(&mut self) -> Status<Node<'a>> {
        let sym = symbols::Symbol::get_symbol("[")
            .ok_or(Error::Syntax("TODO: Left identity function."))
            .map(|n| n.getfn());

        if let Some(next) = self.tokens.next() {
            match next {
                (TokenTag::CloseBracket, _) => self.consume_accessor(),
                (TokenTag::Newline, _) => {
                    Ok(Node::Two(Node::Function(sym?).into(), Node::Line.into()))
                }
                (_, tag) if self.next_if(TokenTag::CloseBracket).is_some() => self.consume_tag(tag),
                next => Ok(Node::Two(
                    Node::Function(sym?).into(),
                    self.consume_node(next)?.into(),
                )),
            }
        } else {
            Ok(Node::Function(sym?))
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
            _ => self.consume_if(top),
        }
    }

    fn consume_primitive(&mut self, s: Scalar) -> Status<Node<'a>> {
        let mut tokens = vec![s];
        while let Some((tag, s)) = self
            .tokens
            .next_if(|(t, _)| *t == TokenTag::Number || *t == TokenTag::Float)
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
            List::from_vec(tokens).map(Node::Primitive)
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
                    .map(|s| Scalar::boxed(self.arena.clone(), List::string(s)))
                    .collect::<Status<Vec<Scalar>>>()?,
            )
            .map(Node::PrimitiveBoxes)
        }
    }

    fn consume_assign(&mut self, head: &'a str, is_internal: bool) -> Status<Node<'a>> {
        let body = self.consume_upto_tag(TokenTag::Newline)?;
        self.tokens.next();
        Ok(Node::Two(
            Node::Assign {
                head,
                is_internal,
                body,
            }
            .into(),
            Node::Line.into(),
        ))
    }

    fn consume_if(&mut self, cond: Node<'a>) -> Status<Node<'a>> {
        let body = self.consume_upto(|n| matches!(n, Node::Line))?;
        let otherwise = self.consume_upto(|n| matches!(n, Node::Line))?;
        Ok(Node::Two(
            Node::If {
                cond: cond.into(),
                body,
                otherwise,
            }
            .into(),
            Node::Line.into(),
        ))
    }

    fn consume_tag(&mut self, tag: &'a str) -> Status<Node<'a>> {
        // TODO: Support some syntactic tags
        Ok(Node::Tag(tag))
    }

    fn consume_accessor(&mut self) -> Status<Node<'a>> {
        todo!("Consume accessor @ <parse.rs> consume_accessor")
    }

    fn consume_group(&mut self) -> Status<Node<'a>> {
        let nodes = self.consume_upto_tag(TokenTag::CloseParen)?;
        self.tokens.next(); // consume_upto does not consume the closing token
        Ok(Node::Group(nodes))
    }

    fn consume_block(&mut self) -> Status<Node<'a>> {
        todo!("Consume block @ <parse.rs> consume_block")
    }

    #[inline]
    fn next_if(&mut self, n: TokenTag) -> Option<&'a Token<'a>> {
        self.tokens.next_if(|(t, _)| *t == n)
    }

    // Swaps nodes temporarily to support certain parsing
    #[inline]
    fn consume_upto_tag(&mut self, tag: TokenTag) -> Status<Vec<Node<'a>>> {
        let mut nodes = Vec::new();
        std::mem::swap(&mut nodes, &mut self.nodes);
        while let Some(tok) = self.tokens.next_if(|(t, _)| *t != tag) {
            let node = self.consume_node(tok);
            Self::unpack_node(&mut self.nodes, node)?;
        }

        std::mem::swap(&mut nodes, &mut self.nodes);
        Ok(nodes)
    }

    #[inline]
    fn consume_upto(&mut self, check: impl Fn(&Node<'a>) -> bool) -> Status<Vec<Node<'a>>> {
        let mut nodes = Vec::new();
        std::mem::swap(&mut nodes, &mut self.nodes);
        while let Some(tok) = self.tokens.next() {
            let node = self.consume_node(tok);
            Self::unpack_node(&mut self.nodes, node)?;
            if check(unsafe { self.nodes.get_unchecked(self.nodes.len() - 1) }) {
                self.nodes.pop();
                break;
            }
        }

        std::mem::swap(&mut nodes, &mut self.nodes);
        Ok(nodes)
    }
}

impl<'a> Debug for Node<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Primitive(l) => write!(f, "{l:?}"),
            Node::PrimitiveString(l) => write!(f, "{l:?}"),
            Node::PrimitiveBoxes(l) => write!(f, "{l:?}"),
            Node::Literal(l) => write!(f, "Lit({l})"),
            Node::LiteralInternal(l) => write!(f, "Internal({l}.)"),
            Node::Function(_) => write!(f, "Function"),
            Node::Line => write!(f, "NL"),
            Node::Tag(t) => write!(f, "[{t}]"),
            Node::Group(g) => write!(f, "({g:?})"),
            Node::If {
                cond,
                body,
                otherwise,
            } => {
                write!(f, "IF({cond:?}){{{{{body:?}}}}}ELSE{{{{{otherwise:?}}}}}")
            }
            Node::Assign {
                head,
                is_internal,
                body,
            } => {
                write!(
                    f,
                    "{head}{} : {body:?}",
                    if *is_internal { "." } else { "" }
                )
            }
            _ => todo!(),
        }
    }
}
