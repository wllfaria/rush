mod input;
mod result;

use std::error::Error;
use std::io::{self, Write, stdout};

use input::{CommandCompleteness, LineInput, determine_command_completeness, read_input};

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = stdout();
    let mut input_buffer = String::new();
    let mut completeness = CommandCompleteness::Complete;

    loop {
        match completeness {
            CommandCompleteness::Complete => write!(stdout, "$ ")?,
            CommandCompleteness::OpenDoubleQuote => write!(stdout, "(dquote)> ")?,
            CommandCompleteness::OpenSingleQuote => write!(stdout, "(quote)> ")?,
            CommandCompleteness::OpenParens => write!(stdout, "(paren)> ")?,
            CommandCompleteness::OpenBraces => write!(stdout, "(brace)> ")?,
            CommandCompleteness::OpenBracket => write!(stdout, "(bracket)> ")?,
            CommandCompleteness::Backslash => write!(stdout, "> ")?,
        }
        io::stdout().flush()?;

        let LineInput::Line(line) = read_input()? else {
            writeln!(stdout)?;
            break;
        };

        input_buffer.push_str(&line);

        completeness = determine_command_completeness(&input_buffer);
        if completeness != CommandCompleteness::Complete {
            continue;
        }

        write!(stdout, "\n\n\n{input_buffer}\n\n\n")?;
        input_buffer.clear();
    }

    Ok(())
}
