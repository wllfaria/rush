use std::iter::Peekable;
use std::str::CharIndices;

use crate::token::Token;
pub use crate::token::{BytePos, Span, TokenKind, TokenStream};

mod token;

#[derive(Debug)]
pub struct Lexer<'src> {
    source: &'src str,
    chars: Peekable<CharIndices<'src>>,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
        }
    }

    pub fn lex(&mut self) -> TokenStream {
        let mut tokens = vec![];
        if self.source.is_empty() {
            return TokenStream::new(tokens, self.source.len());
        }

        while let Some((byte_pos, curr)) = self.next() {
            if is_space(curr) {
                continue; // skip whitespace
            }

            let next = self.peek();

            match (curr, next) {
                ('|', _) => tokens.push(TokenKind::Pipe.into_token(byte_pos)),
                (';', _) => tokens.push(TokenKind::Semi.into_token(byte_pos)),
                ('&', _) => tokens.push(TokenKind::Ampersand.into_token(byte_pos)),
                _ => tokens.push(self.take_atom(byte_pos)),
            }
        }

        tokens.push(self.eof());
        TokenStream::new(tokens, self.source.len())
    }

    fn take_atom(&mut self, start: usize) -> Token {
        let end = self.take_while(|c| !is_delimiter(c), start);
        TokenKind::Atom.into_token((start, end))
    }

    fn next(&mut self) -> Option<(usize, char)> {
        self.chars.next()
    }

    fn peek(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    fn eof(&self) -> Token {
        TokenKind::Eof.into_token(self.source.len())
    }

    fn take_while<F>(&mut self, f: F, start: usize) -> usize
    where
        F: Fn(char) -> bool,
    {
        let mut last = (start, char::default());
        while let Some((byte_pos, ch)) = self.peek() {
            if !f(ch) {
                break;
            };

            last = (byte_pos, ch);
            self.next();
        }

        last.0 + last.1.len_utf8()
    }
}

#[inline]
fn is_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r')
}

fn is_delimiter(ch: char) -> bool {
    is_space(ch) || matches!(ch, '|' | ';' | '&')
}
