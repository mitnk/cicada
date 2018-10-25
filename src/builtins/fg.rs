use std::io::Write;

use libc;
use shell;
use types;
use tools::clog;
use jobc;

pub fn run(sh: &mut shell::Shell, tokens: &types::Tokens) -> i32 {
    if sh.jobs.is_empty() {
        println_stderr!("cicada: fg: no such job");
        return 0;
    }

    let mut pgid = 0;
    if tokens.len() == 1 {
        for (gid, _) in sh.jobs.iter() {
            pgid = *gid;
            break;
        }
    }
    if tokens.len() >= 2 {
        match tokens[1].1.parse::<i32>() {
            Ok(n) => pgid = n,
            Err(_) => {
                println_stderr!("cicada: fg: invalid job id");
                return 1;
            }
        }
    }
    if pgid == 0 {
        println_stderr!("cicada: not job id found");
    }

    if let None = sh.jobs.get(&pgid) {
        println_stderr!("cicada: fg: {}: no such job", pgid);
        return 1;
    }

    unsafe {
        if !shell::give_terminal_to(pgid) {
            return 1;
        }

        libc::killpg(pgid, libc::SIGCONT);

        let mut pid_list = Vec::new();
        if let Some(v) = sh.jobs.get(&pgid) {
            pid_list = v.clone();
        }

        let mut status = 0;
        for pid in pid_list.iter() {
            status = jobc::wait_process(sh, pgid, *pid, true);
        }

        let gid = libc::getpgid(0);
        if !shell::give_terminal_to(gid) {
            log!("failed to give term to back to shell : {}", gid);
        }
        return status;
    }
}
