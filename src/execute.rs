use std::io::Error;
use std::process::{Command, Stdio};
use std::os::unix::io::FromRawFd;
use std::os::unix::process::CommandExt;

use nix::unistd::pipe;
use nix::sys::signal;
use libc;
use tools;
use jobs;


extern "C" fn handle_sigchld(_: i32) {
    // When handle waitpid here & for commands like `ls | cmd-not-exist`
    // will got panic: "wait() should either return Ok or panic"
    // which I currently don't know how to fix.

    /*
    unsafe {
        let mut stat: i32 = 0;
        let ptr: *mut i32 = &mut stat;
        tools::rlog(format!("waiting...\n"));
        let pid = libc::waitpid(-1, ptr, libc::WNOHANG);
        tools::rlog(format!("child {} exit\n", pid));
    }
    */
}


pub fn run_pipeline(args: Vec<String>) -> i32 {
    let sig_action = signal::SigAction::new(signal::SigHandler::Handler(handle_sigchld),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());
    unsafe {
        signal::sigaction(signal::SIGCHLD, &sig_action).unwrap();
    }

    let length = args.len();
    let mut i = 0;

    let mut cmd: Vec<&str> = Vec::new();
    let mut cmds: Vec<Vec<&str>> = Vec::new();
    loop {
        let token = &args[i];
        if token != "|" {
            cmd.push(token.as_str());
        } else {
            cmds.push(cmd.clone());
            cmd = Vec::new();
        }
        i += 1;
        if i >= length {
            cmds.push(cmd.clone());
            break;
        }
    }

    let length = cmds.len();
    let mut pipes = Vec::new();
    i = 0;
    loop {
        let fds = pipe().unwrap();
        pipes.push(fds);
        i += 1;
        if i + 1 >= length {
            break;
        }
    }

    i = 0;
    let mut pgid: u32 = 0;
    let mut children: Vec<u32> = Vec::new();
    let mut status = 0;
    for cmd in &cmds {

        let mut p = Command::new(cmd[0]);
        p.args(&cmd[1..]);
        p.before_exec(move || {
            unsafe {
                if i == 0 {
                    // set the first process as progress group leader
                    let pid = libc::getpid();
                    libc::setpgid(0, pid);
                    tools::rlog(format!("set self gid to {}\n", pid));
                } else {
                    libc::setpgid(0, pgid as i32);
                    tools::rlog(format!("set other gid to {}\n", pgid));
                }
            }
            Ok(())
        });
        if i < length - 1 {
            let fds = pipes[i];
            let pipe_out = unsafe { Stdio::from_raw_fd(fds.1) };
            if i + 1 < length {
                p.stdout(pipe_out);
            }
        }
        if i > 0 {
            let fds_prev = pipes[i - 1];
            let pipe_in = unsafe { Stdio::from_raw_fd(fds_prev.0) };
            p.stdin(pipe_in);
        }

        let mut child;
        if let Ok(x) = p.spawn() {
            child = x;
            if i != length - 1 {
                children.push(child.id());
            }
        } else {
            println!("child spawn error");
            return 1;
        }

        if i == 0 {
            pgid = child.id();
            tools::rlog(format!("try give term to {}\n", pgid));
            unsafe {
                jobs::give_terminal_to(pgid as i32);
            }
        }

        if i == length - 1 {
            let pid = child.id();
            tools::rlog(format!("wait pid {}\n", pid));
            match child.wait() {
                Ok(ecode) => {
                    if ecode.success() {
                        status = 0;
                    } else {
                        status = 1;
                    }
                }
                Err(_) => {
                    match Error::last_os_error().raw_os_error() {
                        Some(10) => {
                            // no such process; it's already done
                            status = 0;
                        }
                        Some(e) => {
                            status = e;
                        }
                        None => {
                            status = 1;
                        }
                    }
                }
            }

            // ack of the zombies
            // TODO: better wait in signal handlers, but.. see above.
            for pid in &children {
                unsafe {
                    let mut stat: i32 = 0;
                    let ptr: *mut i32 = &mut stat;
                    tools::rlog(format!("wait zombie pid {}\n", pid));
                    libc::waitpid(*pid as i32, ptr, 0);
                }
            }
        }
        i += 1;
    }
    return status;
}
