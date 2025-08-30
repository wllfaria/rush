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
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub fn new(start: BytePos, end: BytePos) -> Self {
        Self { start, end }
    }

    pub fn slice<'a>(&self, source: &'a str) -> &'a str {
        &source[*self.start..*self.end]
    }

    pub fn len(&self) -> usize {
        *self.end - *self.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(pub(crate) TokenKind, pub(crate) Span);

impl Token {
    pub fn kind(&self) -> TokenKind {
        self.0
    }

    pub fn span(&self) -> Span {
        self.1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenKind {
    Atom,
    Pipe,
    Semi,
    Ampersand,
    Eof,
}

pub trait IntoSpan {
    fn into_span(self) -> Span;
}

impl IntoSpan for (usize, usize) {
    fn into_span(self) -> Span {
        Span::new(self.0.into(), self.1.into())
    }
}

impl IntoSpan for (BytePos, BytePos) {
    fn into_span(self) -> Span {
        Span::new(self.0, self.1)
    }
}

impl IntoSpan for usize {
    fn into_span(self) -> Span {
        let pos = self.into();
        Span::new(pos, pos)
    }
}

impl IntoSpan for BytePos {
    fn into_span(self) -> Span {
        Span::new(self, self)
    }
}

impl IntoSpan for Span {
    fn into_span(self) -> Span {
        self
    }
}

impl TokenKind {
    pub fn into_token(self, position: impl IntoSpan) -> Token {
        Token(self, position.into_span())
    }
}

#[derive(Debug)]
pub struct TokenStream {
    tokens: Vec<Token>,
    cursor: usize,
    eof: BytePos,
}

impl TokenStream {
    pub fn new(tokens: Vec<Token>, eof: impl Into<BytePos>) -> Self {
        Self {
            tokens,
            cursor: 0,
            eof: eof.into(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> TokenKind {
        match self.tokens.get(self.cursor).copied() {
            Some(token) => {
                self.cursor += 1;
                token.0
            }
            None => TokenKind::Eof,
        }
    }

    pub fn next_token(&mut self) -> Token {
        match self.tokens.get(self.cursor).copied() {
            Some(token) => {
                self.cursor += 1;
                token
            }
            None => TokenKind::Eof.into_token(Span::new(self.eof, self.eof)),
        }
    }

    pub fn peek(&self) -> TokenKind {
        self.tokens
            .get(self.cursor)
            .copied()
            .unwrap_or(TokenKind::Eof.into_token(Span::new(self.eof, self.eof)))
            .0
    }

    pub fn peek_token(&self) -> Token {
        self.tokens
            .get(self.cursor)
            .copied()
            .unwrap_or(TokenKind::Eof.into_token(Span::new(self.eof, self.eof)))
    }
}

impl IntoIterator for TokenStream {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = Token;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}
