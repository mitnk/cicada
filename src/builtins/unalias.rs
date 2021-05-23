use crate::builtins::utils::print_stderr_with_capture;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let tokens = cmd.tokens.clone();
    let mut cr = CommandResult::new();

    if tokens.len() != 2 {
        let info = "cicada: unalias: syntax error";
        print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let input = &tokens[1].1;
    if !sh.remove_alias(input) {
        let info = format!("cicada: unalias: {}: not found", input);
        print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
        return cr;
    }
    cr
}
