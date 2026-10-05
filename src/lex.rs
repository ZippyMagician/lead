use std::iter::Peekable;
use std::str::CharIndices;

use crate::utils::error::{Error, Status, err};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenTag {
    /// `15`, `-3`
    Number,
    Float,
    /// `"hello world"`, `:goodbye`, `"hi"`, `'a`
    String,
    /// `for`, `print`, `add`, etc.
    Literal,
    /// `+`, `!=`, `.`, etc
    Symbol,
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
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// `:`
    Colon,
}

pub type Token<'a> = (TokenTag, &'a str);

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    tokens: Vec<Token<'a>>,
}

macro_rules! tok {
    ($tag:ident, $slice:expr) => {
        Ok((TokenTag::$tag, $slice))
    };
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
    pub fn tokenize(&mut self) -> Status {
        loop {
            let status = self.consume_token();
            match status {
                Ok(tok) => self.tokens.push(tok),
                Err(Error::Complete) => break,
                Err(e) => return Err(e),
            }
        }
        self.tokens.push((TokenTag::EOF, ""));
        Ok(())
    }

    #[inline(always)]
    pub fn tokens(&'a self) -> &'a [Token<'a>] {
        &self.tokens
    }

    #[inline]
    fn consume_token(&mut self) -> Status<Token<'a>> {
        let (start, chr) = self.chars.next().ok_or(Error::Complete)?;
        match chr {
            '_' | 'e' | '0'..='9' => self.maybe_number(start),
            'a'..='z' | 'A'..='Z' => self.consume_lit(start),
            c if c.is_whitespace() => self.consume_whitespace(c, start),
            c if c.is_ascii_punctuation() => self.maybe_builtin(c, start),
            _ => err!(Syntax, "Unrecognized token."),
        }
    }

    fn maybe_builtin(&mut self, chr: char, start: usize) -> Status<Token<'a>> {
        if chr == '-' && self.next_if(|c| c == '-').is_some() {
            return self.skip_comment().ok_or(Error::Complete);
        } else if chr == ':' {
            return Ok((TokenTag::Colon, ":"));
        }

        match chr {
            '\'' => self.consume_string(start),
            '{' => {
                if self.next_if(|c| c == '{').is_some() {
                    tok!(OpenCurly, "{{")
                } else {
                    self.consume_builtin(start)
                }
            }
            '}' => {
                if self.next_if(|c| c == '}').is_some() {
                    tok!(CloseCurly, "}}")
                } else {
                    self.consume_builtin(start)
                }
            }
            '(' => tok!(OpenParen, "("),
            ')' => tok!(CloseParen, ")"),
            '[' => tok!(OpenBracket, "["),
            ']' => tok!(CloseBracket, "]"),
            _ => self.consume_builtin(start),
        }
    }

    #[inline]
    fn consume_whitespace(&mut self, chr: char, _start: usize) -> Status<Token<'a>> {
        if chr == '\n' {
            self.line += 1;
            return tok!(Newline, "\n");
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

    #[inline(always)]
    fn nth(&mut self, i: usize) -> Option<char> {
        self.input.chars().nth(i)
    }

    fn consume_number(&mut self, start: usize) -> Status<Token<'a>> {
        let mut end = start + 1;
        let mut found_e = self.nth(start) == Some('e');
        let mut found_decimals = 0;
        let mut found_signs = 0;
        if self.nth(start) == Some('_') {
            found_signs += 1;
        }

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
            self.chars.next();
            end += 1;
        }

        if found_e && found_decimals == 2 || !found_e && found_decimals == 1 {
            tok!(Float, &self.input[start..end])
        } else {
            tok!(Number, &self.input[start..end])
        }
    }

    #[inline(always)]
    fn valid_lit_chr(chr: char) -> bool {
        chr.is_alphanumeric() || chr == '_'
    }

    #[inline(always)]
    fn calculate_var_bounds(&mut self, start: usize) -> (usize, usize) {
        let mut end = start + 1;
        while self.next_if(Self::valid_lit_chr).is_some() {
            end += 1;
        }

        if self.nth(end) == Some('.') {
            self.chars.next();
            end += 1;
        }

        (start, end)
    }

    // The pass to convert matching symbols to built ins and operators occurs prior to execution
    fn consume_lit(&mut self, start: usize) -> Status<Token<'a>> {
        let (start, end) = self.calculate_var_bounds(start);
        tok!(Literal, &self.input[start..end])
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

        self.line += lines;
        tok!(String, &self.input[start + 1..end - 1])
    }

    // Symbols are one character, possibly with a colon or period following it.
    fn consume_builtin(&mut self, start: usize) -> Status<Token<'a>> {
        let mut end = start + 1;
        if self.next_if(|c| c == ':' || c == '.').is_some() {
            end += 1;
        }

        tok!(Symbol, &self.input[start..end])
    }

    fn skip_comment(&mut self) -> Option<Token<'a>> {
        while let Some((_, tok)) = self.chars.next() {
            if tok == '\n' {
                self.line += 1;
                return Some((TokenTag::Newline, "\n"));
            }
        }

        None // End of file
    }
}
