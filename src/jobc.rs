use std::io::Write;

use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::sys::wait::WaitPidFlag as WF;
use nix::sys::wait::WaitStatus as WS;
use nix::unistd::Pid;

use crate::shell;
use crate::signals;
use crate::types::{self, CommandResult};

pub fn get_job_line(job: &types::Job, trim: bool) -> String {
    let mut cmd = job.cmd.clone();
    if trim && cmd.len() > 50 {
        cmd.truncate(50);
        cmd.push_str(" ...");
    }
    let _cmd = if job.is_bg && job.status == "Running" {
        format!("{} &", cmd)
    } else {
        cmd
    };
    format!("[{}] {}  {}   {}", job.id, job.gid, job.status, _cmd)
}

pub fn print_job(job: &types::Job) {
    let line = get_job_line(job, true);
    println_stderr!("{}", line);
}

pub fn mark_job_as_done(sh: &mut shell::Shell, gid: i32, pid: i32, reason: &str) {
    if let Some(mut job) = sh.remove_pid_from_job(gid, pid) {
        job.status = reason.to_string();
        if job.is_bg {
            println_stderr!("");
            print_job(&job);
        }
    }
}

pub fn mark_job_as_stopped(sh: &mut shell::Shell, gid: i32, report: bool) {
    sh.mark_job_as_stopped(gid);
    if !report {
        return
    }

    // add an extra line to separate output of fg commands if any.
    if let Some(job) = sh.get_job_by_gid(gid) {
        println_stderr!("");
        print_job(job);
    }
}

pub fn mark_job_member_stopped(sh: &mut shell::Shell, pid: i32, gid: i32,
                               report: bool) {
    let _gid = if gid == 0 {
        unsafe { libc::getpgid(pid) }
    } else { gid };

    if let Some(job) = sh.mark_job_member_stopped(pid, gid) {
        if job.all_members_stopped() {
            mark_job_as_stopped(sh, gid, report);
        }
    }
}

pub fn mark_job_member_continued(sh: &mut shell::Shell, pid: i32, gid: i32) {
    let _gid = if gid == 0 {
        unsafe { libc::getpgid(pid) }
    } else { gid };

    if let Some(job) = sh.mark_job_member_continued(pid, gid) {
        if job.all_members_running() {
            mark_job_as_running(sh, gid, true);
        }
    }
}

pub fn mark_job_as_running(sh: &mut shell::Shell, gid: i32, bg: bool) {
    sh.mark_job_as_running(gid, bg);
}

#[allow(unreachable_patterns)]
pub fn waitpidx(wpid: i32, block: bool) -> types::WaitStatus {
    let options = if block {
        Some(WF::WUNTRACED | WF::WCONTINUED)
    } else {
        Some(WF::WUNTRACED | WF::WCONTINUED | WF::WNOHANG)
    };
    match waitpid(Pid::from_raw(wpid), options) {
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

pub fn wait_fg_job(sh: &mut shell:: Shell, gid: i32, pids: &[i32]) -> CommandResult {
    let mut cmd_result = CommandResult::new();
    let mut count_waited = 0;
    let count_child = pids.len();
    if count_child == 0 {
        return cmd_result;
    }
    let pid_last = pids.last().unwrap().clone();

    loop {
        let ws = waitpidx(-1, true);
        // here when we calling waitpidx(), all signals should have
        // been masked. There should no errors (ECHILD/EINTR etc) happen.
        if ws.is_error() {
            let err = ws.get_errno();
            if err == nix::Error::ECHILD {
                break;
            }

            log!("jobc unexpected waitpid error: {}", err);
            cmd_result = CommandResult::from_status(gid, err as i32);
            break;
        }

        let pid = ws.get_pid();
        let is_a_fg_child = pids.contains(&pid);
        if is_a_fg_child && !ws.is_continued() {
            count_waited += 1;
        }

        if ws.is_exited() {
            if is_a_fg_child {
                mark_job_as_done(sh, gid, pid, "Done");
            } else {
                let status = ws.get_status();
                signals::insert_reap_map(pid, status);
            }
        } else if ws.is_stopped() {
            if is_a_fg_child {
                // for stop signal of fg job (current job)
                // i.e. Ctrl-Z is pressed on the fg job
                mark_job_member_stopped(sh, pid, gid, true);
            } else {
                // for stop signal of bg jobs
                signals::insert_stopped_map(pid);
                mark_job_member_stopped(sh, pid, 0, false);
            }
        } else if ws.is_continued() {
            if !is_a_fg_child {
                signals::insert_cont_map(pid);
            }
            continue;
        } else if ws.is_signaled() {
            if is_a_fg_child {
                mark_job_as_done(sh, gid, pid, "Killed");
            } else {
                signals::killed_map_insert(pid, ws.get_signal());
            }
        }

        if is_a_fg_child && pid == pid_last {
            let status = ws.get_status();
            cmd_result.status = status;
        }

        if count_waited >= count_child {
            break;
        }
    }
    cmd_result
}

pub fn try_wait_bg_jobs(sh: &mut shell::Shell, report: bool) {
    if sh.jobs.is_empty() {
        return;
    }

    let jobs = sh.jobs.clone();
    for (_i, job) in jobs.iter() {
        for pid in job.pids.iter() {
            if let Some(_status) = signals::pop_reap_map(*pid) {
                mark_job_as_done(sh, job.gid, *pid, "Done");
                continue;
            }

            if let Some(sig) = signals::killed_map_pop(*pid) {
                let reason = if sig == Signal::SIGQUIT as i32 {
                    format!("Quit: {}", sig)
                } else if sig == Signal::SIGINT as i32 {
                    format!("Interrupt: {}", sig)
                } else if sig == Signal::SIGKILL as i32 {
                    format!("Killed: {}", sig)
                } else if sig == Signal::SIGTERM as i32 {
                    format!("Terminated: {}", sig)
                } else {
                    format!("Killed: {}", sig)
                };
                mark_job_as_done(sh, job.gid, *pid, &reason);
                continue;
            }

            if signals::pop_stopped_map(*pid) {
                mark_job_member_stopped(sh, *pid, job.gid, report);
            } else if signals::pop_cont_map(*pid) {
                mark_job_member_continued(sh, *pid, job.gid);
            }
        }
    }
}
