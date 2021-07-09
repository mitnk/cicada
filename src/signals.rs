use errno::{errno, set_errno};

use nix::sys::signal;
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::Pid;

use crate::shell;
use crate::tools::clog;

extern fn handle_sigchld(_sig: i32) {
    let saved_errno = errno();

    let wait_flag = Some(WaitPidFlag::WNOHANG);
    loop {
        match waitpid(Pid::from_raw(-1), wait_flag) {
            Ok(WaitStatus::Exited(_pid, _status)) => {
                log!("reaped pid:{} status:{}", _pid, _status);
            }
            Ok(WaitStatus::StillAlive) => {
                break;
            }
            Ok(_) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }

    set_errno(saved_errno);
}

pub fn setup_sigchld_handler(_sh: &mut shell::Shell) {
    let sigset = signal::SigSet::empty();
    let handler = signal::SigHandler::Handler(handle_sigchld);
    let sa = signal::SigAction::new(handler, signal::SaFlags::empty(), sigset);
    unsafe {
        match signal::sigaction(signal::SIGCHLD, &sa) {
            Ok(_) => {},
            Err(e) => {
                log!("sigaction error: {:?}", e);
            }
        }
    }
}
