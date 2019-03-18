use std::io::Write;

use crate::shell;
use crate::parsers;
use crate::rcfile;
use crate::types::Tokens;


pub fn run(sh: &mut shell::Shell, tokens: &Tokens) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    if args.len() > 2 {
        println_stderr!("cicada: source: too many arguments");
        return 1;
    }
    if args.len() < 2 {
        println_stderr!("cicada: source: no file specified");
        return 1;
    }
    if !args[1].ends_with("cicadarc") {
        println_stderr!("cicada: source command only supports cicadarc files");
    }

    rcfile::load_file(sh, &args[1], 1);
    0
}
