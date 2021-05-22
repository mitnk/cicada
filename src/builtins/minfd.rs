use crate::shell::Shell;
use crate::builtins::utils::print_stdout;
use crate::types::{CommandLine, CommandResult};

pub fn run(_sh: &mut Shell, cl: &CommandLine, idx_cmd: usize,
           capture: bool) -> CommandResult {
    let cmd = &cl.commands[idx_cmd];
    let _fd = unsafe { libc::dup(1) };
    let mut cr = CommandResult::new();
    let info = format!("{}", _fd);
    if capture {
        cr.stdout = info;
    } else {
        print_stdout(&info, cmd, cl);
    }
    unsafe { libc::close(_fd); }
    cr
}
