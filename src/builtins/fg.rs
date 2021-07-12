use libc;

use crate::builtins::utils::print_stderr_with_capture;
use crate::jobc;
use crate::shell::{self, Shell};
use crate::tools::clog;
use crate::types::{self, CommandResult, CommandLine, Command};

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let tokens = cmd.tokens.clone();
    let mut cr = CommandResult::new();

    if sh.jobs.is_empty() {
        let info = "cicada: fg: no job found";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let mut job_id = -1;
    if tokens.len() == 1 {
        for (gid, _) in sh.jobs.iter() {
            job_id = *gid;
            break;
        }
    }

    if tokens.len() >= 2 {
        let mut job_str = tokens[1].1.clone();
        if job_str.starts_with("%") {
            job_str = job_str.trim_start_matches('%').to_string();
        }

        match job_str.parse::<i32>() {
            Ok(n) => job_id = n,
            Err(_) => {
                let info = "cicada: fg: invalid job id";
                print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
    }

    if job_id == -1 {
        let info = "cicada: not job id found";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let gid: i32;
    let pid_list: Vec<i32>;

    {
        let mut result = sh.get_job_by_id(job_id);
        // fall back to find job by using prcess group id
        if let None = result {
            result = sh.get_job_by_gid(job_id);
        }

        match result {
            Some(job) => {
                let _cmd = job.cmd.trim_matches('&').trim();
                print_stderr_with_capture(&_cmd, &mut cr, cl, cmd, capture);

                unsafe {
                    if !shell::give_terminal_to(job.gid) {
                        return CommandResult::error();
                    }

                    libc::killpg(job.gid, libc::SIGCONT);
                    pid_list = job.pids.clone();
                    gid = job.gid;
                }
            }
            None => {
                let info = "cicada: fg: no such job";
                print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
    }

    unsafe {
        jobc::mark_job_as_running(sh, gid, false);

        let mut status = 0;
        for pid in pid_list.iter() {
            status = jobc::wait_process(sh, gid, *pid, true);
        }

        if status == types::WS_STOPPED {
            jobc::mark_job_as_stopped(sh, gid);
        }

        let gid_shell = libc::getpgid(0);
        if !shell::give_terminal_to(gid_shell) {
            log!("failed to give term to back to shell : {}", gid_shell);
        }
        return CommandResult::from_status(0, status);
    }
}
