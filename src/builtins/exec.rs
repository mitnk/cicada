use exec;

use crate::builtins::utils::print_stderr_with_capture;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{Command, CommandLine, CommandResult};

pub fn run(_sh: &Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = cmd.tokens.clone();
    let args = parsers::parser_line::tokens_to_args(&tokens);
    let len = args.len();
    if len == 1 {
        print_stderr_with_capture("invalid usage", &mut cr, cl, cmd, capture);
        return cr;
    }

    let mut _cmd = exec::Command::new(&args[1]);
    let err = _cmd.args(&args[2..len]).exec();
    let info = format!("cicada: exec: {}", err);
    print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
    cr
}
