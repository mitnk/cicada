use std::io;
use lineread::{Function, Prompter, Terminal};

use crate::parsers::parser_line;

pub struct EnterFunction;

impl<T: Terminal> Function<T> for EnterFunction {
    fn execute(&self, prompter: &mut Prompter<T>, count: i32, _ch: char) -> io::Result<()> {
        let buf = prompter.buffer();
        let linfo = parser_line::parse_line(buf);
        if linfo.is_complete {
            prompter.accept_input()
        } else if count > 0 {
            match prompter.insert(count as usize, '\n') {
                Ok(_) => {},
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
