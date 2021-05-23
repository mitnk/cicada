use crate::builtins::utils::print_stderr_with_capture;
use crate::parsers;
use crate::scripting;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(&tokens);

    if args.len() < 2 {
        let info = "cicada: source: no file specified";
        print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let status = scripting::run_script(sh, &args);
    cr.status = status;
    cr
}
