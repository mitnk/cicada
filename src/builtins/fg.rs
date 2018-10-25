use std::io::Write;

use nix::sys::signal;
use nix::sys::wait::WaitStatus;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;

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

        if let None = sh.pgs.get(&pgid) {
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
            if let Some(v) = sh.pgs.get(&pgid) {
                pid_list = v.clone();
            }

            for pid in pid_list.iter() {
                log!("waitpid: {}", *pid);
                match waitpid(Pid::from_raw(*pid), Some(nix::sys::wait::WaitPidFlag::WUNTRACED)) {
                    Ok(WaitStatus::Stopped(_pid, _)) => {
                        return 148;
                    }
                    Ok(WaitStatus::Exited(npid, _status)) => {
                        jobc::cleanup_process_groups(sh, pgid, npid.into());
                    }
                    Ok(WaitStatus::Signaled(npid, signal::SIGINT, _)) => {
                        jobc::cleanup_process_groups(sh, pgid, npid.into());
                    }
                    Ok(_info) => {
                        log!("fg waitpid ok: {:?}", _info);
                    }
                    Err(_e) => {
                        log!("fg waitpid error: {:?}", _e);
                    }
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
