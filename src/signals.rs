use errno::{errno, set_errno};

use nix::sys::signal;
use nix::sys::wait::{WaitPidFlag as WF, WaitStatus as WS, waitpid};
use nix::unistd::Pid;
use std::sync::Mutex;
use std::collections::{HashMap, HashSet};

use crate::tools::clog;

lazy_static! {
    static ref REAP_MAP: Mutex<HashMap<i32, i32>> = Mutex::new(HashMap::new());
    static ref STOP_MAP: Mutex<HashSet<i32>> = Mutex::new(HashSet::new());
    static ref CONT_MAP: Mutex<HashSet<i32>> = Mutex::new(HashSet::new());
}

fn insert_cont_map(pid: i32) {
    CONT_MAP.lock().unwrap().insert(pid);
}

pub fn pop_cont_map(pid: i32) -> bool {
    CONT_MAP.lock().unwrap().remove(&pid)
}

pub fn insert_stopped_map(pid: i32) {
    STOP_MAP.lock().unwrap().insert(pid);
}

pub fn pop_stopped_map(pid: i32) -> bool {
    STOP_MAP.lock().unwrap().remove(&pid)
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
    log!("enter handle_sigchld ...");

    let options = Some(WF::WUNTRACED | WF::WNOHANG | WF::WCONTINUED);
    loop {
        match waitpid(Pid::from_raw(-1), options) {
            Ok(WS::Exited(pid, status)) => {
                log!("chld Exited: {}", pid);
                insert_reap_map(i32::from(pid), status);
            }
            Ok(WS::Stopped(pid, sig)) => {
                log!("chld Stopped: {} sig:{}", pid, sig);
                insert_stopped_map(i32::from(pid));
            }
            Ok(WS::Continued(pid)) => {
                log!("chld Continued: pid:{}", pid);
                insert_cont_map(i32::from(pid));
            }
            Ok(WS::StillAlive) => {
                break;
            }
            Ok(_others) => {
                log!("chld others: {:?}", _others);
            }
            Err(_e) => {
                log!("sigchld waitpid error: {:?}", _e);
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
