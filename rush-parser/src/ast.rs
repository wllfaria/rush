use rush_lexer::{Span, TokenKind};

use crate::result::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    Command(SimpleCommand),
    Pipeline(Vec<SimpleCommand>),
    BackgroundJob(Box<Ast>),
    Sequence(Vec<Ast>),
}

impl Ast {
    pub fn into_command(self) -> Result<SimpleCommand> {
        match self {
            Self::Command(cmd) => Ok(cmd),
            _ => Err(Error::ExpectedCommand(TokenKind::Atom)), // TODO: better error
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleCommand {
    pub program: Span,
    pub args: Vec<Span>,
}

pub trait DisplayAst {
    fn to_string(&self, source: &str) -> String;
}

impl DisplayAst for Ast {
    fn to_string(&self, source: &str) -> String {
        let mut formatted = String::new();

        match self {
            Self::BackgroundJob(ast) => formatted.push_str(&format!("{} &", ast.to_string(source))),
            Self::Command(cmd) => formatted.push_str(&cmd.to_string(source)),
            Self::Pipeline(cmds) => cmds.iter().enumerate().for_each(|(i, cmd)| {
                let cmd = cmd.to_string(source);
                let is_last = i == cmds.len() - 1;
                let sep = if is_last { "" } else { " | " };
                formatted.push_str(&format!("{cmd}{sep}"))
            }),
            Self::Sequence(seq) => seq.iter().enumerate().for_each(|(i, ast)| {
                let cmd = ast.to_string(source);
                let is_last = i == seq.len() - 1;
                let sep = if is_last { "" } else { "; " };
                formatted.push_str(&format!("{cmd}{sep}"))
            }),
        };

        formatted
    }
}

impl DisplayAst for SimpleCommand {
    fn to_string(&self, source: &str) -> String {
        let name = self.program.slice(source);
        let args = self
            .args
            .iter()
            .map(|arg| arg.slice(source))
            .collect::<Vec<_>>()
            .join(" ");

        let args = if !args.is_empty() { format!(" {args}") } else { args };

        format!("{name}{args}")
    }
}
