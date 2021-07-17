use crate::shell::Shell;
use crate::builtins::utils::print_stdout_with_capture;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(_sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();

    let fd = nix::fcntl::open(
        "/dev/null",
        nix::fcntl::OFlag::empty(),
        nix::sys::stat::Mode::empty()
    );
    match fd {
        Ok(fd) => {
            let info = format!("{}", fd);
            print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
            unsafe { libc::close(fd); }
        }
        Err(e) => {
            println!("cicada: minfd: error: {}", e);
        }
    }

    cr
}
