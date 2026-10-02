use std::iter::Peekable;
use std::str::CharIndices;

use crate::builtins::Symbol;
use crate::utils::error::{Error, Status, err, print_error};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token<'a> {
    /// `15`, `-3`
    Number(i32),
    Float(f32),
    /// `"hello world"`, `:goodbye`, `"hi"`, `'a`
    String(&'a str),
    /// `for`, `print`, `add`, etc.
    Literal(&'a str),
    /// `+`, `!=`, `.`, etc
    Symbol(Symbol),
    /// Has semantic meaning, unlike other whitespace
    /// e.g. in `-> x : . + \n 1 2 3 x`, x's definition should end with the newline
    Newline,
    /// EOF
    EOF,
    /// `{{`
    OpenCurly,
    /// `}}`
    CloseCurly,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
}

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    tokens: Vec<Token<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input,
            chars: input.char_indices().peekable(),
            line: 0,
            tokens: Vec::new(),
        }
    }

    #[inline]
    fn next_if(&mut self, f: impl FnOnce(char) -> bool) -> Option<(usize, char)> {
        self.chars.next_if(|&(_, c)| f(c))
    }

    /// Entry function for tokenization
    #[inline]
    pub fn tokenize(&mut self) {
        loop {
            let status = self.consume_token();
            match status {
                Ok(tok) => self.tokens.push(tok),
                Err(Error::LexComplete) => break,
                Err(e) => print_error(e),
            }
        }

        self.tokens.push(Token::EOF);
    }

    #[inline(always)]
    pub fn tokens(&'a self) -> &'a [Token<'a>] {
        &self.tokens
    }

    #[inline]
    fn consume_token(&mut self) -> Status<Token<'a>> {
        let (start, chr) = self.chars.next().ok_or(Error::LexComplete)?;
        match chr {
            '_' | 'e' | '0'..='9' => self.maybe_number(start),
            'a'..='z' | 'A'..='Z' => self.consume_lit(start),
            c if c.is_whitespace() => self.consume_whitespace(c, start),
            c if c.is_ascii_punctuation() => self.maybe_sym(c, start),
            _ => err!(Syntax, "Unrecognized token."),
        }
    }

    fn maybe_sym(&mut self, chr: char, start: usize) -> Status<Token<'a>> {
        if chr == '-' && self.next_if(|c| c == '-').is_some() {
            return self.skip_comment().ok_or(Error::LexComplete);
        }
        match chr {
            '\'' => self.consume_string(start),
            '{' => {
                if self.next_if(|c| c == '{').is_some() {
                    Ok(Token::OpenCurly)
                } else {
                    self.consume_symbol(start)
                }
            }
            '}' => {
                if self.next_if(|c| c == '}').is_some() {
                    Ok(Token::CloseCurly)
                } else {
                    self.consume_symbol(start)
                }
            }
            '(' => Ok(Token::OpenParen),
            ')' => Ok(Token::CloseParen),
            _ => self.consume_symbol(start),
        }
    }

    #[inline]
    fn consume_whitespace(&mut self, chr: char, _start: usize) -> Status<Token<'a>> {
        if chr == '\n' {
            let tok = Token::Newline;
            self.line += 1;
            return Ok(tok);
        }

        self.consume_token()
    }

    #[inline]
    fn maybe_number(&mut self, start: usize) -> Status<Token<'a>> {
        let c = &self.input[start..start + 1];
        if c == "e" {
            if let Some(_) = self.next_if(|c| c.is_numeric() || c == '_') {
                self.consume_number(start)
            } else {
                self.consume_lit(start)
            }
        } else {
            self.consume_number(start)
        }
    }

    fn consume_number(&mut self, start: usize) -> Status<Token<'a>> {
        let mut end = start + 1;
        let mut found_e = self.input.chars().nth(start) == Some('e');
        let mut found_decimals = 0;
        let mut found_signs = 1;

        if found_e {
            found_decimals += 1;
        }
        while let Some((_, c)) = self.chars.peek() {
            match c {
                '.' if found_decimals == 0 || found_decimals == 1 && found_e => {
                    found_decimals += 1;
                }
                'e' if !found_e => {
                    found_e = true;
                }
                '_' if found_signs == 1 && found_e => {
                    found_signs += 1;
                }
                '0'..'9' => {}
                _ => break,
            }
            end += 1;
        }

        if found_e && found_decimals == 2 || !found_e && found_decimals == 1 {
            Ok(Token::Float(self.input[start..end].parse::<f32>()?))
        } else {
            Ok(Token::Number(self.input[start..end].parse::<i32>()?))
        }
    }

    fn valid_lit_chr(chr: char) -> bool {
        chr.is_alphanumeric() || chr == '_'
    }

    fn calculate_var_bounds(&mut self, start: usize) -> (usize, usize) {
        let mut end = start + 1;
        while self.next_if(Self::valid_lit_chr).is_some() {
            end += 1;
        }

        (start, end)
    }

    // The pass to convert matching symbols to built ins and operators occurs prior to execution
    fn consume_lit(&mut self, start: usize) -> Status<Token<'a>> {
        let (start, end) = self.calculate_var_bounds(start);
        Ok(Token::Literal(&self.input[start..end]))
    }

    fn consume_string(&mut self, start: usize) -> Status<Token<'a>> {
        let mut lines = 0;
        let mut end = start + 2;
        while let Some((_, chr)) = self.chars.next() {
            if chr == '\'' {
                break;
            }
            if chr == '\n' {
                lines += 1;
            }
            end += 1;
        }

        let tok = Ok(Token::String(&self.input[start + 1..end - 1]));
        self.line += lines;
        tok
    }

    fn consume_symbol(&mut self, start: usize) -> Status<Token<'a>> {
        let mut end = start + 1;
        while self
            .next_if(|_| Symbol::get_symbol(&self.input[start..end + 1]).is_some())
            .is_some()
        {
            end += 1;
        }

        Ok(Token::Symbol(
            Symbol::get_symbol(&self.input[start..end])
                .ok_or(Error::Syntax("Unrecognized symbol."))?,
        ))
    }

    fn skip_comment(&mut self) -> Option<Token<'a>> {
        while let Some((_, tok)) = self.chars.next() {
            if tok == '\n' {
                let t = Some(Token::Newline);
                self.line += 1;
                return t;
            }
        }

        None // End of file
    }
}
