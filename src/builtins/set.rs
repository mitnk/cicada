use structopt::StructOpt;

use crate::builtins::utils::{print_stdout, print_stderr};
use crate::parsers;
use crate::shell::Shell;
use crate::types::CommandLine;

#[derive(Debug, StructOpt)]
#[structopt(name = "set", about = "Set shell options (BETA)")]
struct OptMain {
    #[structopt(short, help = "exit on error status")]
    exit_on_error: bool,
}

pub fn run(sh: &mut Shell, cl: &CommandLine, idx_cmd: usize) -> i32 {
    let cmd = &cl.commands[idx_cmd];
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);
    let show_usage = args.len() > 1 && (args[1] == "-h" || args[1] == "--help");

    let opt = OptMain::from_iter_safe(args);
    match opt {
        Ok(opt) => {
            if opt.exit_on_error {
                sh.exit_on_error = true;
                return 0;
            } else {
                print_stderr("cicada: set: option not implemented", cmd, cl);
                return 1;
            }
        }
        Err(e) => {
            let info = format!("{}", e);
            if show_usage {
                print_stdout(&info, cmd, cl);
            } else {
                print_stderr(&info, cmd, cl);
            }
            let status = if show_usage { 0 } else { 1 };
            return status;
        }
    }
}
