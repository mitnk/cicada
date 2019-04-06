use std::io::Write;

use crate::shell;
use crate::types::Tokens;

pub fn run(sh: &mut shell::Shell, tokens: &Tokens) -> i32 {
    if tokens.len() != 2 {
        println_stderr!("unalias syntax error");
        println_stderr!("unalias usage example: alias foo");
        return 1;
    }

    let input = &tokens[1].1;
    if !sh.remove_alias(input) {
        println_stderr!("cicada: unalias: {}: not found", input);
        return 1;
    }
    0
}
