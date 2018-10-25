use std::io::Write;

use nix::sys::signal;
use nix::sys::wait::{WaitStatus, WaitPidFlag, waitpid};
use nix::unistd::Pid;

use shell;
use tools::clog;

pub fn wait_process(sh: &mut shell::Shell, gid: i32, pid: i32,
    stop: bool
) -> i32 {
    log!("\nenter wait_process(): pid: {}", pid);
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
            log!("waitpid ok: {:?}", _info);
        }
        Err(_e) => {
            log!("waitpid error: {:?}", _e);
            status = 1;
        }
    }
    log!("enter wait_process(): pid: {}\n", pid);
    status
}

pub fn cleanup_process_groups(sh: &mut shell::Shell, gid: i32, pid: i32,
    report: bool
) {
    log!("clean up jobs gid: {} pid: {}", gid, pid);

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
