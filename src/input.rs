use crate::result::Result;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandCompleteness {
    OpenDoubleQuote,
    OpenSingleQuote,
    OpenParens,
    OpenBraces,
    OpenBracket,
    Backslash,
    Complete,
}

pub enum LineInput {
    Line(String),
    Eof,
}

pub fn read_input() -> Result<LineInput> {
    let stdin = std::io::stdin();
    let mut line = String::new();
    let bytes_read = stdin.read_line(&mut line)?;
    if bytes_read == 0 { Ok(LineInput::Eof) } else { Ok(LineInput::Line(line)) }
}

pub fn determine_command_completeness(text: &str) -> CommandCompleteness {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut parens = 0;
    let mut braces = 0;
    let mut brackets = 0;

    let mut iter = text.chars().peekable();
    while let Some(ch) = iter.next() {
        match ch {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote => in_double_quote = !in_double_quote,
            '(' if !in_single_quote && !in_double_quote => parens += 1,
            ')' if !in_single_quote && !in_double_quote && parens > 0 => parens -= 1,
            '{' if !in_single_quote && !in_double_quote => braces += 1,
            '}' if !in_single_quote && !in_double_quote && braces > 0 => braces -= 1,
            '[' if !in_single_quote && !in_double_quote => brackets += 1,
            ']' if !in_single_quote && !in_double_quote && brackets > 0 => brackets -= 1,
            '\\' => match iter.peek().copied() {
                Some('\n') => {
                    iter.next();

                    if iter.peek().is_none() {
                        return CommandCompleteness::Backslash;
                    }
                }
                Some('\r') => {
                    iter.next();
                    if let Some('\n') = iter.peek().copied() {
                        iter.next();
                    }

                    if iter.peek().is_none() {
                        return CommandCompleteness::Backslash;
                    }
                }
                Some(_) => _ = iter.next(),
                None => return CommandCompleteness::Backslash,
            },
            _ => {}
        }
    }

    if in_single_quote {
        return CommandCompleteness::OpenSingleQuote;
    }
    if in_double_quote {
        return CommandCompleteness::OpenDoubleQuote;
    }
    if parens > 0 {
        return CommandCompleteness::OpenParens;
    }
    if braces > 0 {
        return CommandCompleteness::OpenBraces;
    }
    if brackets > 0 {
        return CommandCompleteness::OpenBracket;
    }

    CommandCompleteness::Complete
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_completeness_backslash() {
        let command = [
            "my_command \\",
            "some \\",
            "arguments \\",
            "on \\",
            "multiple \\",
            "lines \\",
        ]
        .join("\n");

        let completeness = determine_command_completeness(&command);
        assert_eq!(completeness, CommandCompleteness::Backslash);
    }

    #[test]
    fn test_command_open_quote() {
        let command = ["my_command ", "some", "arguments", "on", "multiple", "lines '"].join("\n");
        let completeness = determine_command_completeness(&command);
        assert_eq!(completeness, CommandCompleteness::OpenSingleQuote);
    }
}
