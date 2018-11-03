use std::io::Write;

use jobc;
use libc;
use shell;
use tools::clog;
use types;

pub fn run(sh: &mut shell::Shell, tokens: &types::Tokens) -> i32 {
    if sh.jobs.is_empty() {
        println_stderr!("cicada: fg: no job found");
        return 0;
    }

    let mut job_id = 0;
    if tokens.len() == 1 {
        for (gid, _) in sh.jobs.iter() {
            job_id = *gid;
            break;
        }
    }
    if tokens.len() >= 2 {
        match tokens[1].1.parse::<i32>() {
            Ok(n) => job_id = n,
            Err(_) => {
                println_stderr!("cicada: fg: invalid job id");
                return 1;
            }
        }
    }
    if job_id == 0 {
        println_stderr!("cicada: not job id found");
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
                let cmd = job.cmd.trim_matches('&').trim();
                println_stderr!("{}", cmd);

                unsafe {
                    if !shell::give_terminal_to(job.gid) {
                        return 1;
                    }

                    libc::killpg(job.gid, libc::SIGCONT);
                    pid_list = job.pids.clone();
                    gid = job.gid;
                }
            }
            None => {
                println_stderr!("cicada: fg: no such job");
                return 1;
            }
        }
    }

    unsafe {
        jobc::mark_job_as_running(sh, gid, false);

        let mut status = 0;
        for pid in pid_list.iter() {
            status = jobc::wait_process(sh, gid, *pid, true);
        }

        if status == types::STOPPED {
            jobc::mark_job_as_stopped(sh, gid);
        }

        let gid_shell = libc::getpgid(0);
        if !shell::give_terminal_to(gid_shell) {
            log!("failed to give term to back to shell : {}", gid_shell);
        }
        return status;
    }
}
