use lineread::{Function, Prompter, Terminal};
use std::io;

use crate::parsers::parser_line;

pub struct EnterFunction;

impl<T: Terminal> Function<T> for EnterFunction {
    fn execute(&self, prompter: &mut Prompter<T>, count: i32, _ch: char) -> io::Result<()> {
        let buf = prompter.buffer();
        // heredoc bodies are data: they are kept away from the line parser, so
        // that e.g. an apostrophe in one does not read as an unclosed quote
        let (cmds, in_heredoc) = crate::parsers::heredoc::split_buffer(buf);
        let linfo = parser_line::parse_line(&cmds);
        if linfo.is_complete && !in_heredoc {
            prompter.accept_input()
        } else if count > 0 {
            match prompter.insert(count as usize, '\n') {
                Ok(_) => {}
                Err(e) => {
                    println!("sub-prompt error: {}", e);
                }
            }
            prompter.insert_str(">> ")
        } else {
            Ok(())
        }
    }
}
