use std::collections::HashMap;
use std::io::{Write, stdout};
use std::sync::{Arc, Mutex};

use rush_runner::ExecCtx;

use crate::input::{CommandCompleteness, LineInput, determine_command_completeness, read_input};
use crate::result::Result;

pub struct Rush {
    jobs: Arc<Mutex<HashMap<u32, rush_runner::Job>>>,
    next_job_id: Arc<Mutex<u32>>,
    shell_pgid: nix::unistd::Pid,
    shell_terminal: i32,
}

impl Rush {
    pub fn new() -> Self {
        let (shell_pgid, shell_terminal) = rush_runner::init_shell().expect("Failed to initialize shell");

        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            next_job_id: Arc::new(Mutex::new(1)),
            shell_pgid,
            shell_terminal,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut stdout = stdout();
        let mut input_buffer = String::new();
        let mut completeness = CommandCompleteness::Complete;

        loop {
            input_buffer.clear();

            match completeness {
                CommandCompleteness::Complete => write!(stdout, "$ ")?,
                CommandCompleteness::OpenDoubleQuote => write!(stdout, "(dquote)> ")?,
                CommandCompleteness::OpenSingleQuote => write!(stdout, "(quote)> ")?,
                CommandCompleteness::OpenParens => write!(stdout, "(paren)> ")?,
                CommandCompleteness::OpenBraces => write!(stdout, "(brace)> ")?,
                CommandCompleteness::OpenBracket => write!(stdout, "(bracket)> ")?,
                CommandCompleteness::Backslash => write!(stdout, "> ")?,
            }

            stdout.flush()?;
            let LineInput::Line(line) = read_input()? else {
                writeln!(stdout)?;
                break;
            };

            input_buffer.push_str(&line);
            completeness = determine_command_completeness(&input_buffer);
            if completeness != CommandCompleteness::Complete {
                continue;
            }

            let tokens = rush_lexer::Lexer::new(&input_buffer).lex();
            let Ok(commands) = rush_parser::Parser::new().parse(tokens) else {
                writeln!(stdout, "Parse error")?;
                continue;
            };

            rush_runner::update_job_statuses(self.jobs.clone());

            let mut ctx = ExecCtx {
                source: &input_buffer,
                jobs: self.jobs.clone(),
                next_job_id: self.next_job_id.clone(),
                shell_pgid: self.shell_pgid,
                shell_terminal: self.shell_terminal,
            };

            rush_runner::execute(&mut ctx, commands)?;
        }

        Ok(())
    }
}
