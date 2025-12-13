//! bg - background builtin

use crate::builtins::utils::print_stderr_with_capture;
use crate::jobc;
use crate::shell::Shell;
use crate::types::{Command, CommandLine, CommandResult};

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let tokens = cmd.tokens.clone();
    let mut cr = CommandResult::new();

    if sh.jobs.is_empty() {
        let info = "cicada: bg: no job found";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let mut job_id = -1;
    if tokens.len() == 1 {
        if let Some((gid, _)) = sh.jobs.iter().next() {
            job_id = *gid;
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
                let info = "cicada: bg: invalid job id";
                print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
    }
    if job_id == -1 {
        let info = "cicada: bg: not such job";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let gid: i32;

    {
        let mut result = sh.get_job_by_id(job_id);
        // fall back to find job by using prcess group id
        if result.is_none() {
            result = sh.get_job_by_gid(job_id);
        }

        match result {
            Some(job) => {
                unsafe {
                    nix::libc::killpg(job.gid, nix::libc::SIGCONT);
                    gid = job.gid;
                    if job.status == "Running" {
                        let info = format!("cicada: bg: job {} already in background", job.id);
                        print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                        return cr;
                    }
                }

                let info_cmd = format!("[{}]  {} &", job.id, job.cmd);
                print_stderr_with_capture(&info_cmd, &mut cr, cl, cmd, capture);
                cr.status = 0;
            }
            None => {
                let info = "cicada: bg: not such job";
                print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
    }

    jobc::mark_job_as_running(sh, gid, true);
    cr
}
