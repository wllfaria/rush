use std::iter::Peekable;
use std::str::CharIndices;

use token::{Token, TokenKind};

use crate::token::TokenStream;

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
            return TokenStream::new(tokens);
        }

        while let Some((byte_pos, curr)) = self.next() {
            let next = self.peek();

            match (curr, next) {
                ('|', _) => tokens.push(TokenKind::Pipe.into_token(byte_pos)),
                (';', _) => tokens.push(TokenKind::Semi.into_token(byte_pos)),
                _ => tokens.push(self.take_atom(byte_pos)),
            }
        }

        tokens.push(self.eof());
        TokenStream::new(tokens)
    }

    fn take_atom(&mut self, start: usize) -> Token {
        let end = self.take_while(|c| !is_delimiter(c));
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

    fn take_while<F>(&mut self, f: F) -> usize
    where
        F: Fn(char) -> bool,
    {
        let mut last = None;
        while let Some((byte_pos, ch)) = self.peek() {
            if !f(ch) {
                break;
            };

            last = Some((byte_pos, ch));
            self.next();
        }

        match last {
            Some((i, ch)) => i + ch.len_utf8(),
            None => self.source.len(),
        }
    }
}

#[inline]
fn is_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r')
}

fn is_delimiter(ch: char) -> bool {
    is_space(ch) || matches!(ch, '|' | ';')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::BytePos;

    trait TokenSource<'src> {
        fn token_source(&self, source: &'src str) -> &'src str;
    }

    impl<'src> TokenSource<'src> for (BytePos, BytePos) {
        fn token_source(&self, source: &'src str) -> &'src str {
            &source[*self.0..*self.1]
        }
    }

    #[derive(Debug)]
    struct TokenTest<'src>(TokenKind, &'src str);

    #[test]
    fn test_lexing_atom() {
        let source = "some-command-here";
        let tokens = Lexer::new(source)
            .lex()
            .into_iter()
            .map(|t| TokenTest(t.0, t.1.token_source(source)))
            .collect::<Vec<_>>();

        panic!("{tokens:#?}")
    }
}
