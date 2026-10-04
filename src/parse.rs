use std::fmt::{self, Debug};
use std::iter::Peekable;
use std::slice::Iter;

use crate::function::Function;
use crate::lex::Token;
use crate::list::List;
use crate::scalar::Scalar;
use crate::utils::Arena;
use crate::utils::error::{Error, Status, err, print_error};

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
    Block(Vec<Node<'a>>),
    If {
        cond: Box<Node<'a>>,
        body: Vec<Node<'a>>,
        otherwise: Vec<Node<'a>>,
    },
    Lazy(Box<Node<'a>>),
    Line,
}

pub struct Parser<'a> {
    nodes: Vec<Node<'a>>,
    tokens: Peekable<Iter<'a, Token<'a>>>,
    arena: Arena,
}

impl<'a> Parser<'a> {
    pub fn new(arena: Arena, l: &'a [Token<'a>]) -> Self {
        Self {
            nodes: vec![],
            tokens: l.iter().peekable(),
            arena,
        }
    }

    #[inline]
    pub fn parse(&mut self) {
        loop {
            let status = self.consume_node();
            match status {
                Ok(node) => self.nodes.push(node),
                Err(Error::Complete) => break,
                Err(e) => print_error(e),
            }
        }

        self.nodes.push(Node::Line);
    }

    #[inline(always)]
    pub fn nodes(&'a self) -> &'a [Node<'a>] {
        &self.nodes
    }

    pub fn consume_node(&mut self) -> Status<Node<'a>> {
        let node = self.tokens.peek().ok_or(Error::Complete)?;
        match node {
            Token::Number(_) | Token::Float(_) => self.consume_primitive(),
            Token::String(_) => self.consume_primitive_string(),
            Token::Symbol(s) => Ok(Node::Function(s.getfn())),
            Token::EOF => err!(Complete),
            _ => err!(Syntax, "Unrecognized token."),
        }
    }

    fn consume_primitive(&mut self) -> Status<Node<'a>> {
        let mut tokens = vec![];
        while let Some(tok) = self.tokens.peek() {
            if let Token::Number(n) = tok {
                tokens.push(Scalar::Number(*n));
            } else if let Token::Float(f) = tok {
                tokens.push(Scalar::Float(*f));
            } else {
                break;
            }
            self.tokens.next();
        }

        if tokens.len() == 1 {
            Ok(Node::Primitive(List::scalar(tokens[0])))
        } else {
            List::from_vec(tokens).map(Node::Primitive)
        }
    }

    fn consume_primitive_string(&mut self) -> Status<Node<'a>> {
        let mut strings = vec![];
        while let Some(Token::String(s)) = self.tokens.peek() {
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
