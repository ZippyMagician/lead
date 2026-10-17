use std::iter::Peekable;
use std::str::CharIndices;

use crate::builtins::operators::Operator;
use crate::builtins::symbols::Symbol;
use crate::builtins::{OPERATORS, SYMBOLS};
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
    /// `+`, `=`, etc
    Symbol,
    /// `&`, `/`, etc
    Operator,
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
    /// `.`
    Dot,
    /// Literal tab
    Tab,
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
        // See utils::generate_block
        let input = Box::leak(input.into());
        Self { input, chars: input.char_indices().peekable(), line: 0, tokens: Vec::new() }
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
            '_' | 'e' | '.' | '0'..='9' => self.maybe_number(chr, start),
            'a'..='z' | 'A'..='Z' => self.consume_lit(start),
            '\t' => Ok((TokenTag::Tab, &self.input[start..start + 1])),
            c if c.is_whitespace() => self.consume_whitespace(c, start),
            c if c.is_ascii_punctuation() => self.maybe_builtin(c, start),
            _ => err!(Syntax, "Unrecognized token."),
        }
    }

    fn maybe_builtin(&mut self, chr: char, start: usize) -> Status<Token<'a>> {
        if chr == '-' && self.next_if(|c| c == '-').is_some() {
            return self.skip_comment().ok_or(Error::Complete);
        } else if chr == ':' {
            return Ok((TokenTag::Colon, &self.input[start..start + 1]));
        }

        match chr {
            '\'' => self.consume_string(start),
            '`' => self.maybe_char(start),
            '{' => {
                if self.next_if(|c| c == '{').is_some() {
                    tok!(OpenCurly, &self.input[start..start + 2])
                } else {
                    self.consume_builtin(start)
                }
            }
            '}' => {
                if self.next_if(|c| c == '}').is_some() {
                    tok!(CloseCurly, &self.input[start..start + 2])
                } else {
                    self.consume_builtin(start)
                }
            }
            '(' => tok!(OpenParen, &self.input[start..start + 1]),
            ')' => tok!(CloseParen, &self.input[start..start + 1]),
            '[' => tok!(OpenBracket, &self.input[start..start + 1]),
            ']' => tok!(CloseBracket, &self.input[start..start + 1]),
            _ => self.consume_builtin(start),
        }
    }

    #[inline]
    fn consume_whitespace(&mut self, chr: char, start: usize) -> Status<Token<'a>> {
        if chr == '\n' {
            self.line += 1;
            return tok!(Newline, &self.input[start..start + 1]);
        }

        self.consume_token()
    }

    #[inline]
    fn maybe_char(&mut self, start: usize) -> Status<Token<'a>> {
        if self.next_if(|c| c.is_alphanumeric()).is_some() {
            tok!(String, &self.input[start + 1..start + 2])
        } else {
            self.consume_builtin(start)
        }
    }

    #[inline]
    fn maybe_number(&mut self, chr: char, start: usize) -> Status<Token<'a>> {
        if chr == 'e' || chr == '.' {
            if let Some((_, c)) = self.chars.peek()
                && (c.is_numeric() || chr == 'e' && *c == '_')
            {
                self.consume_number(start)
            } else if chr == '.' {
                Ok((TokenTag::Dot, &self.input[start..start + 1]))
            } else {
                self.consume_lit(start)
            }
        } else {
            self.consume_number(start)
        }
    }

    #[inline(always)]
    fn nth(&self, i: usize) -> Option<char> {
        self.input.chars().nth(i)
    }

    fn consume_number(&mut self, start: usize) -> Status<Token<'a>> {
        let mut end = start + 1;
        let mut found_e = false;
        let mut found_decimals = 0;
        let mut found_signs = 0;
        match self.nth(start) {
            Some('_') => found_signs = 1,
            Some('e') => {
                found_e = true;
                found_decimals = 1;
            }
            Some('.') => found_decimals = 1,
            _ => {}
        }

        while let Some((_, c)) = self.chars.peek() {
            match c {
                '.' if found_decimals == 0 || found_decimals == 1 && found_e => {
                    found_decimals += 1;
                }
                'e' if !found_e => {
                    found_e = true;
                }
                // I can't use self.nth becuase the borrow checker hates me
                '_' if self.input.chars().nth(start).unwrap() != 'e' || found_signs == 0 => {
                    found_signs += 1;
                    if found_signs == 2 && !found_e {
                        break;
                    }
                }
                '0'..='9' => {}
                _ => break,
            }
            self.chars.next();
            end += 1;
        }

        if found_e || found_decimals > 0 || found_signs == end - start {
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
        for (_, chr) in self.chars.by_ref() {
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
        if self
            .next_if(|_| {
                SYMBOLS.contains(&self.input[start..end + 1].as_bytes())
                    || OPERATORS.contains(&self.input[start..end + 1].as_bytes())
            })
            .is_some()
        {
            end += 1;
        }

        if Operator::get_operator(&self.input[start..end]).is_some() {
            tok!(Operator, &self.input[start..end])
        } else if Symbol::get_symbol(&self.input[start..end]).is_some()
            || &self.input[start..end] == "{" // TODO: Remove these once these symbols are
                                              // implemented
            || &self.input[start..end] == "}"
            || &self.input[start..end] == ";"
        {
            tok!(Symbol, &self.input[start..end])
        } else {
            err!(Syntax, "Unrecognized builtin.")
        }
    }

    fn skip_comment(&mut self) -> Option<Token<'a>> {
        for (start, tok) in self.chars.by_ref() {
            if tok == '\n' {
                self.line += 1;
                return Some((TokenTag::Newline, &self.input[start..start + 1]));
            }
        }

        None // End of file
    }
}
