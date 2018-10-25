use std::io::Write;

use libc;
use shell;
use types;
use tools::clog;
use jobc;

pub fn run(sh: &mut shell::Shell, tokens: &types::Tokens) -> i32 {
    if tokens.len() == 2 {
        let pgid;
        match tokens[1].1.parse::<i32>() {
            Ok(n) => pgid = n,
            Err(_) => {
                println_stderr!("cicada: fg: invalid parameter");
                return 1;
            }
        }

        if let None = sh.jobs.get(&pgid) {
            println_stderr!("cicada: fg: no such process group");
            return 1;
        }

        unsafe {
            if !shell::give_terminal_to(pgid) {
                return 1;
            }
            log!("gave term to pgid: {}", pgid);

            libc::killpg(pgid, libc::SIGCONT);

            let mut pid_list = Vec::new();
            if let Some(v) = sh.jobs.get(&pgid) {
                pid_list = v.clone();
            }

            for pid in pid_list.iter() {
                log!("waitpid: {}", *pid);
                match jobc::wait_process(sh, pgid, *pid, true) {
                    148 => {
                        return 148;
                    }
                    _ => {}
                }
            }

            let gid = libc::getpgid(0);
            if shell::give_terminal_to(gid) {
                log!("gave term to back to shell : {}", gid);
            }
        }
        return 0;
    }
    return 0;
}
