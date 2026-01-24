use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;

use crate::builtins::utils::print_stdout_with_capture;
use crate::shell::Shell;
use crate::types::{Command, CommandLine, CommandResult};

pub fn run(_sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();

    match File::open("/dev/null") {
        Ok(f) => {
            let info = format!("{}", f.as_raw_fd());
            print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
        }
        Err(e) => {
            println_stderr!("cicada: minfd: error: {}", e);
        }
    }

    cr
}
