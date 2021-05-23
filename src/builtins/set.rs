use structopt::StructOpt;

use crate::builtins::utils::print_stderr_with_capture;
use crate::builtins::utils::print_stdout_with_capture;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

#[derive(Debug, StructOpt)]
#[structopt(name = "set", about = "Set shell options (BETA)")]
struct OptMain {
    #[structopt(short, help = "exit on error status")]
    exit_on_error: bool,
}

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);
    let show_usage = args.len() > 1 && (args[1] == "-h" || args[1] == "--help");

    let opt = OptMain::from_iter_safe(args);
    match opt {
        Ok(opt) => {
            if opt.exit_on_error {
                sh.exit_on_error = true;
                return cr;
            } else {
                let info = "cicada: set: option not implemented";
                print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
        Err(e) => {
            let info = format!("{}", e);
            if show_usage {
                print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
                cr.status = 0;
            } else {
                print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                cr.status = 1;
            }
            return cr;
        }
    }
}
