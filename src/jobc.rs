use std::io::Write;

use nix::errno::Errno;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use nix::Error;

use shell;
use tools::clog;

pub fn wait_process(sh: &mut shell::Shell, gid: i32, pid: i32, stop: bool) -> i32 {
    let mut status = 0;
    let flags = if stop {
        Some(WaitPidFlag::WUNTRACED)
    } else {
        Some(WaitPidFlag::WNOHANG)
    };
    match waitpid(Pid::from_raw(pid), flags) {
        Ok(WaitStatus::Stopped(_pid, _)) => {
            status = 148;
        }
        Ok(WaitStatus::Exited(npid, status_new)) => {
            cleanup_process_groups(sh, gid, npid.into(), false);
            status = status_new;
        }
        Ok(WaitStatus::Signaled(npid, signal::SIGINT, _)) => {
            cleanup_process_groups(sh, gid, npid.into(), false);
            status = 130;
        }
        Ok(_info) => {
            // log!("waitpid ok: {:?}", _info);
        }
        Err(e) => match e {
            Error::Sys(errno) => {
                if errno == Errno::ECHILD {
                    cleanup_process_groups(sh, gid, pid, false);
                } else {
                    log!("waitpid error: errno: {:?}", errno);
                }
            }
            _ => {
                log!("waitpid error: {:?}", e);
                status = 1;
            }
        },
    }
    status
}

pub fn cleanup_process_groups(sh: &mut shell::Shell, gid: i32, pid: i32, report: bool) {
    let mut empty_pids = false;
    if let Some(x) = sh.jobs.get_mut(&gid) {
        if let Ok(i) = x.binary_search(&pid) {
            x.remove(i);
        }
        empty_pids = x.is_empty();
    }

    if empty_pids {
        sh.jobs.remove(&gid);
        if report {
            println_stderr!("[todo]  Done    todo");
        }
    }
}

pub fn try_wait_bg_jobs(sh: &mut shell::Shell) {
    if sh.jobs.is_empty() {
        return;
    }

    let jobs = sh.jobs.clone();

    for (gid, v) in jobs.iter() {
        for pid in v.iter() {
            wait_process(sh, *gid, *pid, false);
        }
    }
}
