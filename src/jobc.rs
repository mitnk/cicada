use std::io::Write;

use nix::errno::Errno;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use nix::Error;

use crate::shell;
use crate::tools::clog;
use crate::types;

pub fn print_job(job: &types::Job) {
    let mut cmd = job.cmd.clone();
    if cmd.len() > 50 {
        cmd.truncate(50);
        cmd.push_str(" ...");
    }
    let _cmd;
    if job.status != "Running" {
        _cmd = cmd.trim_matches('&').trim();
    } else {
        _cmd = cmd.as_str();
    }
    println_stderr!("[{}] {}  {}    {}", job.id, job.gid, job.status, _cmd);
}

fn cleanup_process_groups(sh: &mut shell::Shell, gid: i32, pid: i32, reason: &str) {
    if let Some(mut job) = sh.remove_pid_from_job(gid, pid) {
        job.status = reason.to_string();
        if job.report {
            print_job(&job);
        }
    }
}

pub fn mark_job_as_stopped(sh: &mut shell::Shell, gid: i32) {
    sh.mark_job_as_stopped(gid);
    if let Some(job) = sh.get_job_by_gid(gid) {
        print_job(job);
    }
}

pub fn mark_job_as_running(sh: &mut shell::Shell, gid: i32, bg: bool) {
    sh.mark_job_as_running(gid, bg);
}

pub fn wait_process(sh: &mut shell::Shell, gid: i32, pid: i32, stop: bool) -> i32 {
    let mut status = 0;
    let flags = if stop {
        Some(WaitPidFlag::WUNTRACED)
    } else {
        Some(WaitPidFlag::WNOHANG)
    };
    match waitpid(Pid::from_raw(pid), flags) {
        Ok(WaitStatus::Stopped(_pid, _)) => {
            status = types::STOPPED;
        }
        Ok(WaitStatus::Exited(npid, status_new)) => {
            cleanup_process_groups(sh, gid, npid.into(), "Done");
            status = status_new;
        }
        Ok(WaitStatus::Signaled(npid, sig, _)) => {
            let reason = if sig == signal::SIGKILL {
                "Killed: 9".to_string()
            } else if sig == signal::SIGTERM {
                "Terminated: 15".to_string()
            } else if sig == signal::SIGQUIT {
                "Quit: 3".to_string()
            } else if sig == signal::SIGINT {
                "Interrupt: 2".to_string()
            } else if sig == signal::SIGHUP {
                "Hangup: 1".to_string()
            } else if sig == signal::SIGABRT {
                "Abort trap: 6".to_string()
            } else {
                format!("Signaled: {:?}", sig)
            };
            cleanup_process_groups(sh, gid, npid.into(), &reason);
            status = sig as i32;
        }
        Ok(_info) => {
            // log!("waitpid ok: {:?}", _info);
        }
        Err(e) => match e {
            Error::Sys(errno) => {
                if errno == Errno::ECHILD {
                    cleanup_process_groups(sh, gid, pid, "Done");
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

pub fn try_wait_bg_jobs(sh: &mut shell::Shell) {
    if sh.jobs.is_empty() {
        return;
    }
    let jobs = sh.jobs.clone();
    for (_i, job) in jobs.iter() {
        for pid in job.pids.iter() {
            wait_process(sh, job.gid, *pid, false);
        }
    }
}
