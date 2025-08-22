use crate::token::TokenStream;

mod token;

#[derive(Debug)]
pub struct Lexer<'src> {
    needle: &'src str,
    source: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(text: &'src str) -> Self {
        Self {
            needle: text,
            source: text,
        }
    }

    pub fn lex(&mut self) -> TokenStream {
        let mut tokens = vec![];
        if self.source.is_empty() {
            return TokenStream::new(tokens);
        }

        loop {
            let mut chars = self.needle.chars().peekable();
            let curr = chars.next().expect("next character should always be available");
            let next = chars.peek();
            todo!()
        }

        TokenStream::new(tokens)
    }
}
