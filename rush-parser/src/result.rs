pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Expected command, found {0:?}")]
    ExpectedCommand(rush_lexer::TokenKind),
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(rush_lexer::TokenKind),
    #[error("Empty command")]
    EmptyCommand,
}
