mod ast;
mod result;

pub use ast::{Ast, DisplayAst, SimpleCommand};
use result::{Error, Result};
use rush_lexer::{TokenKind, TokenStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BindingPower(u8);

impl BindingPower {
    const BACKGROUND: BindingPower = BindingPower(20);
    const MIN: BindingPower = BindingPower(0);
    const PIPELINE: BindingPower = BindingPower(30);
    const SEQUENCE: BindingPower = BindingPower(10);

    fn operator_binding_power(token: TokenKind) -> Option<BindingPower> {
        match token {
            TokenKind::Semi => Some(BindingPower::SEQUENCE),
            TokenKind::Ampersand => Some(BindingPower::BACKGROUND),
            TokenKind::Pipe => Some(BindingPower::PIPELINE),
            _ => None,
        }
    }
}

pub struct Parser {}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse(&self, mut tokens: TokenStream) -> Result<Ast> {
        self.parse_expression(&mut tokens, BindingPower::MIN)
    }

    fn parse_expression(&self, tokens: &mut TokenStream, min_bp: BindingPower) -> Result<Ast> {
        let mut left = self.parse_primary(tokens)?;

        while let Some(operator_binding_power) = BindingPower::operator_binding_power(tokens.peek()) {
            if operator_binding_power < min_bp {
                break;
            }

            match tokens.next() {
                // ; is a infix operator that denotes a sequence of commands
                // if the left side is already a sequence, we flatten by pushing the right side
                TokenKind::Semi if matches!(left, Ast::Sequence(_)) => {
                    let Ast::Sequence(mut seq) = left else { unreachable!() };
                    seq.push(self.parse_expression(tokens, operator_binding_power)?);
                    left = Ast::Sequence(seq);
                }
                // otherwise we make a sequence from left and right expressions
                TokenKind::Semi => {
                    let right = self.parse_expression(tokens, operator_binding_power)?;
                    left = Ast::Sequence(vec![left, right]);
                }
                // & is a postfix operator, no right operand is needed.
                TokenKind::Ampersand => left = Ast::BackgroundJob(Box::new(left)),
                // | is a infix operator, so it requires both left and right side
                // if the left side is already a pipeline, we flatten by pushing the right side
                TokenKind::Pipe if matches!(left, Ast::Pipeline(_)) => {
                    let Ast::Pipeline(mut commands) = left else { unreachable!() };
                    let right_command = self.parse_expression(tokens, operator_binding_power)?.into_command()?;
                    commands.push(right_command);
                    left = Ast::Pipeline(commands);
                }
                // otherwise, create a pipeline from left and right commands
                TokenKind::Pipe => {
                    let right = self.parse_expression(tokens, operator_binding_power)?;
                    let left_cmd = left.into_command()?;
                    let right_cmd = right.into_command()?;
                    left = Ast::Pipeline(vec![left_cmd, right_cmd]);
                }
                op => return Err(Error::UnexpectedToken(op)),
            }
        }

        Ok(left)
    }

    fn parse_primary(&self, tokens: &mut TokenStream) -> Result<Ast> {
        match tokens.peek() {
            TokenKind::Atom => Ok(Ast::Command(self.parse_command(tokens)?)),
            TokenKind::Eof => Err(Error::UnexpectedEof),
            other => Err(Error::ExpectedCommand(other)),
        }
    }

    fn parse_command(&self, tokens: &mut TokenStream) -> Result<SimpleCommand> {
        // expect at least one atom for the program name
        let program_token = tokens.next_token();
        let program_span = match program_token.kind() {
            TokenKind::Atom => program_token.span(),
            TokenKind::Eof => return Err(Error::UnexpectedEof),
            other => return Err(Error::ExpectedCommand(other)),
        };

        let mut args = vec![];
        while matches!(tokens.peek(), TokenKind::Atom) {
            let arg_token = tokens.next_token();
            args.push(arg_token.span());
        }

        Ok(SimpleCommand {
            program: program_span,
            args,
        })
    }
}

#[cfg(test)]
mod tests {
    use rush_lexer::Span;

    use super::*;

    trait IntoSnapshot {
        fn into_snapshot(self, source: &str) -> SnapshotAst;
    }

    #[derive(Debug, Clone, PartialEq)]
    struct SimpleCommandSnapshot {
        program: Span,
        args: Vec<Span>,
        source: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum SnapshotAst {
        Command(SimpleCommandSnapshot),
        Pipeline(Vec<SimpleCommandSnapshot>),
        BackgroundJob(Box<SnapshotAst>),
        Sequence(Vec<SnapshotAst>),
    }

    impl SnapshotAst {
        pub fn into_command(self) -> SimpleCommandSnapshot {
            match self {
                SnapshotAst::Command(command) => command,
                _ => unreachable!(),
            }
        }
    }

    impl IntoSnapshot for SimpleCommand {
        fn into_snapshot(self, source: &str) -> SnapshotAst {
            SnapshotAst::Command(SimpleCommandSnapshot {
                program: self.program,
                source: self.to_string(source),
                args: self.args,
            })
        }
    }

    impl IntoSnapshot for Ast {
        fn into_snapshot(self, source: &str) -> SnapshotAst {
            match self {
                Ast::Command(command) => command.into_snapshot(source),
                Ast::Pipeline(commands) => SnapshotAst::Pipeline(
                    commands
                        .into_iter()
                        .map(|cmd| cmd.into_snapshot(source).into_command())
                        .collect(),
                ),
                Ast::BackgroundJob(ast) => SnapshotAst::BackgroundJob(Box::new(ast.into_snapshot(source))),
                Ast::Sequence(asts) => {
                    SnapshotAst::Sequence(asts.into_iter().map(|ast| ast.into_snapshot(source)).collect())
                }
            }
        }
    }

    #[test]
    fn test_parsing_sequence() {
        let source = "ls -la;pwd";
        let tokens = rush_lexer::Lexer::new(source).lex();
        let ast = Parser::new().parse(tokens).unwrap();

        assert!(matches!(ast, Ast::Sequence(_)));
        insta::assert_debug_snapshot!(ast.into_snapshot(source));
    }

    #[test]
    fn test_parsing_pipeline() {
        let source = "echo hello | wc -l";
        let tokens = rush_lexer::Lexer::new(source).lex();
        let ast = Parser::new().parse(tokens).unwrap();
        assert!(matches!(ast, Ast::Pipeline(_)));
        insta::assert_debug_snapshot!(ast.into_snapshot(source));
    }

    #[test]
    fn test_parsing_background_job() {
        let source = "sleep 2 &";
        let tokens = rush_lexer::Lexer::new(source).lex();
        let ast = Parser::new().parse(tokens).unwrap();
        assert!(matches!(ast, Ast::BackgroundJob(_)));
        insta::assert_debug_snapshot!(ast.into_snapshot(source));
    }
}
