use errno::{errno, set_errno};

use nix::sys::signal;
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::Pid;
use std::sync::Mutex;
use std::collections::HashMap;

use crate::tools::clog;

lazy_static! {
    static ref REAP_MAP: Mutex<HashMap<i32, i32>> = Mutex::new(HashMap::new());
}

fn insert_reap_map(pid: i32, status: i32) {
    REAP_MAP.lock().unwrap().insert(pid, status);
}

pub fn pop_reap_map(pid: i32) -> Option<i32> {
    REAP_MAP.lock().unwrap().remove(&pid)
}

pub fn block_signals() {
    let mut sigset = signal::SigSet::empty();
    sigset.add(signal::SIGCHLD);
    match signal::sigprocmask(signal::SigmaskHow::SIG_BLOCK, Some(&sigset), None) {
        Ok(_) => {},
        Err(e) => {
            log!("sigprocmask block error: {:?}", e);
        }
    }
}

pub fn unblock_signals() {
    let mut sigset = signal::SigSet::empty();
    sigset.add(signal::SIGCHLD);
    match signal::sigprocmask(signal::SigmaskHow::SIG_UNBLOCK, Some(&sigset), None) {
        Ok(_) => {},
        Err(e) => {
            log!("sigprocmask unblock error: {:?}", e);
        }
    }
}

extern fn handle_sigchld(_sig: i32) {
    let saved_errno = errno();

    let wait_flag = Some(WaitPidFlag::WNOHANG);
    loop {
        match waitpid(Pid::from_raw(-1), wait_flag) {
            Ok(WaitStatus::Exited(pid, status)) => {
                insert_reap_map(i32::from(pid), status);
            }
            Ok(_others) => {
                break;
            }
            Err(_e) => {
                // log!("sigchld waitpid error: {:?}", _e);
                break;
            }
        }
    }

    set_errno(saved_errno);
}

pub fn setup_sigchld_handler() {
    let sigset = signal::SigSet::empty();
    let handler = signal::SigHandler::Handler(handle_sigchld);
    // automatically restart system calls interrupted by this signal handler
    let flags = signal::SaFlags::SA_RESTART;
    let sa = signal::SigAction::new(handler, flags, sigset);
    unsafe {
        match signal::sigaction(signal::SIGCHLD, &sa) {
            Ok(_) => {},
            Err(e) => {
                log!("sigaction error: {:?}", e);
            }
        }
    }
}
