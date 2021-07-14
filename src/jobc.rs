use std::io::Write;

use nix::sys::signal;
use nix::sys::wait::waitpid;
use nix::sys::wait::WaitPidFlag as WF;
use nix::sys::wait::WaitStatus as WS;
use nix::unistd::Pid;

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
    let _cmd = if job.is_bg && job.status == "Running" {
        format!("{} &", job.cmd)
    } else {
        job.cmd.clone()
    };
    println_stderr!("[{}] {}  {}    {}", job.id, job.gid, job.status, _cmd);
}

pub fn cleanup_process_groups(sh: &mut shell::Shell, gid: i32, pid: i32, reason: &str) {
    if let Some(mut job) = sh.remove_pid_from_job(gid, pid) {
        job.status = reason.to_string();
        if job.is_bg {
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

pub fn mark_job_member_stopped(sh: &mut shell::Shell, pid: i32, gid: i32) {
    let _gid = if gid == 0 {
        unsafe { libc::getpgid(pid) }
    } else { gid };

    if let Some(job) = sh.mark_job_member_stopped(pid, gid) {
        if job.all_members_stopped() {
            mark_job_as_stopped(sh, gid);
        }
    }
}

pub fn mark_job_as_running(sh: &mut shell::Shell, gid: i32, bg: bool) {
    sh.mark_job_as_running(gid, bg);
}

#[allow(unreachable_patterns)]
pub fn waitpid_all() -> types::WaitStatus {
    let options = Some(WF::WUNTRACED | WF::WCONTINUED);
    match waitpid(Pid::from_raw(-1), options) {
        Ok(WS::Exited(pid, status)) => {
            let pid = i32::from(pid);
            return types::WaitStatus::from_exited(pid, status);
        }
        Ok(WS::Stopped(pid, sig)) => {
            let pid = i32::from(pid);
            return types::WaitStatus::from_stopped(pid, sig as i32);
        }
        Ok(WS::Continued(pid)) => {
            let pid = i32::from(pid);
            return types::WaitStatus::from_continuted(pid);
        }
        Ok(WS::Signaled(pid, sig, _core_dumped)) => {
            let pid = i32::from(pid);
            return types::WaitStatus::from_signaled(pid, sig as i32);
        }
        Ok(WS::StillAlive) => {
            return types::WaitStatus::empty();
        }
        Ok(_others) => {
            // this is for PtraceEvent and PtraceSyscall on Linux,
            // unreachable on other platforms.
            return types::WaitStatus::from_others();
        }
        Err(e) => {
            return types::WaitStatus::from_error(e as i32);
        }
    }
}

pub fn wait_process(sh: &mut shell::Shell, gid: i32, pid: i32, stop: bool) -> i32 {
    log!("jobc enter wait_process pid: {}", pid);
    let flags = if stop {
        Some(WF::WUNTRACED)
    } else {
        Some(WF::WNOHANG)
    };

    match waitpid(Pid::from_raw(pid), flags) {
        Ok(WS::Stopped(_pid, _)) => {
            return types::WS_STOPPED;
        }
        Ok(WS::Exited(npid, status_new)) => {
            cleanup_process_groups(sh, gid, npid.into(), "Done");
            return status_new;
        }
        Ok(WS::Signaled(npid, sig, _)) => {
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
            nix::Error::ECHILD => {
                // log!("jobc wait_process ECHILD: pid: {:?}", pid);
                cleanup_process_groups(sh, gid, pid, "Done");

                // since we installed a signal handler for SIGCHLD,
                // commands like `sleep 2 | sleep 1 | exit 3`, the latter
                // two children is reaped by it, we have to fetch/sync
                // the exit status of them from the signal handler.
                if let Some(status) = signals::pop_reap_map(pid) {
                    // log!("jobc ECHILD: pid:{:?} got status:{}", pid, status);
                    return status;
                }

                // log!("no reap info found for ECHILD: status --> 1 pid:{}", pid);
                return 1;
            }
            nix::Error::EINTR => {
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
            _ => {
                log!("jobc waitpid error: {:?}", e);
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
            if signals::pop_stopped_map(*pid) {
                mark_job_member_stopped(sh, *pid, job.gid);
            } else if signals::pop_cont_map(*pid) {
                mark_job_as_running(sh, job.gid, true);
            }
        }
    }
}
