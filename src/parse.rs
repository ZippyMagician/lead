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

#[derive(Clone)]
pub enum Node<'a> {
    Primitive(List),
    PrimitiveString(List),
    PrimitiveBoxes(List),
    Literal(&'a str),
    LiteralInternal(&'a str),
    Tag(&'a str),
    Function(Function),
    Group(Vec<Node<'a>>),
    If {
        cond: Box<Node<'a>>,
        body: Vec<Node<'a>>,
        otherwise: Vec<Node<'a>>,
    },
    Lazy(Box<Node<'a>>),
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
    pub fn nodes(&'a self) -> &'a [Node<'a>] {
        &self.nodes
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
            EOF => err!(Complete),
            _ => err!(Syntax, "Unrecognized token."),
        }
    }

    fn consume_primitive(&mut self, s: Scalar) -> Status<Node<'a>> {
        let mut tokens = vec![s];
        while let Some(tok) = self.tokens.next() {
            if let (TokenTag::Number, n) = tok {
                tokens.push(Scalar::Number(parse_number(*n)?));
            } else if let (TokenTag::Float, f) = tok {
                tokens.push(Scalar::Float(parse_float(*f)?));
            } else {
                break;
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
        while let Some((TokenTag::String, s)) = self.tokens.peek() {
            self.tokens.next();
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
                (_, tag @ _) if self.next_is(TokenTag::CloseBracket) => Ok(Node::Tag(tag)),
                next @ _ => Ok(Node::Two(
                    Node::Function(sym?).into(),
                    self.consume_node(next)?.into(),
                )),
            }
        } else {
            Ok(Node::Function(sym?))
        }
    }

    fn consume_accessor(&mut self) -> Status<Node<'a>> {
        todo!("Consume accessor")
    }

    #[inline]
    fn next_is(&mut self, n: TokenTag) -> bool {
        self.tokens.next_if(|(t, _)| *t == n).is_some()
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
            _ => todo!(),
        }
    }
}
