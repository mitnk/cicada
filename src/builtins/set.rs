use structopt::StructOpt;

use crate::builtins::utils::print_stderr;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{Command, CommandLine};

#[derive(Debug, StructOpt)]
#[structopt(name = "set", about = "Set shell options")]
struct OptMain {
    #[structopt(short, help = "exit on error status")]
    exit_on_error: bool,
}

pub fn run(sh: &mut Shell, cmd: &Command, cl: &CommandLine) -> i32 {
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);

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
            print_stderr(&info, cmd, cl);
            return 1;
        }
    }
}
