use crate::shell::Shell;
use crate::builtins::utils::print_stdout;
use crate::types::CommandLine;

pub fn run(_sh: &mut Shell, cl: &CommandLine, idx_cmd: usize) -> i32 {
    let cmd = &cl.commands[idx_cmd];
    let _fd = unsafe { libc::dup(1) };
    let info = format!("{}", _fd);
    print_stdout(&info, cmd, cl);
    unsafe { libc::close(_fd); }
    0
}
