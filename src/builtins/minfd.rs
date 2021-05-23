use crate::shell::Shell;
use crate::builtins::utils::print_stdout_with_capture;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(_sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let _fd = unsafe { libc::dup(1) };
    let mut cr = CommandResult::new();
    let info = format!("{}", _fd);
    print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
    unsafe { libc::close(_fd); }
    cr
}
