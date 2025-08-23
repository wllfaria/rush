#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytePos(usize);

impl std::ops::Deref for BytePos {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for BytePos {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(pub(crate) TokenKind, pub(crate) (BytePos, BytePos));

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenKind {
    Atom,
    Pipe,
    Semi,
    Eof,
}

pub trait IntoBytePos {
    fn into_byte_pos(self) -> (BytePos, BytePos);
}

impl IntoBytePos for (usize, usize) {
    fn into_byte_pos(self) -> (BytePos, BytePos) {
        (self.0.into(), self.1.into())
    }
}

impl IntoBytePos for usize {
    fn into_byte_pos(self) -> (BytePos, BytePos) {
        (self.into(), self.into())
    }
}

impl TokenKind {
    pub fn into_token(self, position: impl IntoBytePos) -> Token {
        Token(self, position.into_byte_pos())
    }
}

#[derive(Debug)]
pub struct TokenStream {
    tokens: Vec<Token>,
    cursor: usize,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn next(&mut self) -> TokenKind {
        match self.tokens.get(self.cursor).copied() {
            Some(token) => {
                self.cursor += 1;
                token.0
            }
            None => TokenKind::Eof,
        }
    }
}

impl IntoIterator for TokenStream {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = Token;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}
