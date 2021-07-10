use std::io::Write;

use nix::errno::Errno;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use nix::Error;

use crate::shell;
use crate::signals;
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
    let flags = if stop {
        Some(WaitPidFlag::WUNTRACED)
    } else {
        Some(WaitPidFlag::WNOHANG)
    };

    match waitpid(Pid::from_raw(pid), flags) {
        Ok(WaitStatus::Stopped(_pid, _)) => {
            return types::STOPPED;
        }
        Ok(WaitStatus::Exited(npid, status_new)) => {
            cleanup_process_groups(sh, gid, npid.into(), "Done");
            return status_new;
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
            return sig as i32;
        }
        Ok(_info) => {
            // log!("waitpid ok: {:?}", _info);
            return 0;
        }
        Err(e) => match e {
            Error::Sys(errno) => {
                if errno == Errno::ECHILD {
                    // log!("jobc wait_process ECHILD: pid: {:?}", pid);
                    cleanup_process_groups(sh, gid, pid, "Done");

                    // similar with EINTR branch, for some commands
                    // e.g. `sleep 2 | sleep 1 | exit 3` we will miss the
                    // chance to get the exit status of `exit` in EINTR branch,
                    // we have to catch the info here.
                    if let Some(status) = signals::pop_reap_map(pid) {
                        // log!("jobc ECHILD: pid:{:?} got status:{}", pid, status);
                        return status;
                    }

                    // log!("no reap info found for ECHILD: status --> 1 pid:{}", pid);
                    return 1;
                }

                if errno == Errno::EINTR {
                    // log!("jobc wait_process got EINTR: {}", pid);

                    // since we have installed a signal handler for SIGCHLD,
                    // it will always interrupt the waitpid() here, thus
                    // the exit status will be lost. we have to fetch the info
                    // from the signal::pop_reap_map().

                    // UPDATED: if we use SA_RESTART when calling sigaction(),
                    // this EINTR branch won't be reached, but we cannot
                    // assume all OS implementations follow this flag. Thus
                    // we still handle EINTR here.

                    // NOTE the OS implementation differences: on Mac,
                    // waitpid() in signal handler always handles first;
                    // on Linux, waitpid() in wait_process() handles first.
                    // for example, normal commands like `echo hi | wc`,
                    // on Mac both zombies are reaped by signal handler, and
                    // the waitpid() in wait_process() here got interrupt;
                    // while on Linux, they are reaped by wait_process(),
                    // the signal handler won't be called.
                    if let Some(status) = signals::pop_reap_map(pid) {
                        // log!("jobc EINTR pid:{} got status:{}", pid, status);
                        cleanup_process_groups(sh, gid, pid, "Done");
                        return status;
                    } else {
                        // one example: `sleep 2 | sleep 1 | exit 3`
                        // log!("jobc pid {} not yet terminated, re-wait", pid);
                        return wait_process(sh, gid, pid, stop);
                    }
                }

                log!("waitpid error 1: errno: {:?}", errno);
                return 1;
            }
            _ => {
                log!("waitpid error 2: {:?}", e);
                return 1;
            }
        }
    }
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
