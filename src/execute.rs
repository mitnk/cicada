use std::io::Error;
// use std::fs::File;
use std::fs::OpenOptions;
use std::process::{Command, Stdio};
use std::os::unix::io::{FromRawFd, IntoRawFd};
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

fn args_to_cmds(args: Vec<String>) -> Vec<Vec<String>> {
    let mut cmd: Vec<String> = Vec::new();
    let mut cmds: Vec<Vec<String>> = Vec::new();
    for token in &args {
        if token != "|" {
            cmd.push(token.trim().to_string());
        } else {
            cmds.push(cmd.clone());
            cmd = Vec::new();
        }
    }
    cmds.push(cmd.clone());
    cmds
}

fn args_to_redirections(args: Vec<String>) -> (Vec<String>, Vec<i32>) {
    let mut vec_redirected = Vec::new();
    let mut args_new = args.clone();
    let mut redirected_to = 0;
    for arg in &args_new {
        if arg == "2>&1" {
            redirected_to = 1;
        }
        if arg == "1>&2" {
            redirected_to = 2;
        }
        if arg == "|" {
            vec_redirected.push(redirected_to);
            redirected_to = 0;
        }
    }
    vec_redirected.push(redirected_to);

    while args_new.iter().position(|x| *x == "2>&1").is_some() {
        let index = args_new.iter().position(|x| *x == "2>&1").unwrap();
        args_new.remove(index);
    }
    while args_new.iter().position(|x| *x == "1>&2").is_some() {
        let index = args_new.iter().position(|x| *x == "1>&2").unwrap();
        args_new.remove(index);
    }
    (args_new, vec_redirected)
}

pub fn run_pipeline(args: Vec<String>, redirect: &str, append: bool) -> i32 {
    let sig_action = signal::SigAction::new(signal::SigHandler::Handler(handle_sigchld),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());
    unsafe {
        signal::sigaction(signal::SIGCHLD, &sig_action).unwrap();
    }

    let (args_new, vec_redirected) = args_to_redirections(args);
    let cmds = args_to_cmds(args_new);
    let length = cmds.len();
    let mut pipes = Vec::new();
    for _ in 0..length - 1 {
        let fds = pipe().unwrap();
        pipes.push(fds);
    }

    let mut i = 0;
    let mut pgid: u32 = 0;
    let mut children: Vec<u32> = Vec::new();
    let mut status = 0;
    for cmd in &cmds {
        let mut p = Command::new(&cmd[0]);
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
            p.stdout(pipe_out);

            if vec_redirected[i] > 0 {
                if vec_redirected[i] == 1 {
                    let fds = pipes[i];
                    let pipe_out = unsafe { Stdio::from_raw_fd(fds.1) };
                    p.stderr(pipe_out);
                }
                /* else if vec_redirected[i] == 2 {
                    unsafe { p.stdout(Stdio::from_raw_fd(2)) };
                }
                */
            }
        }

        if i > 0 {
            // redirect to stderr has some issues now:
            // run commands like `ls | wc 1>&2 | cat` a second time will crash
            /*
            if vec_redirected[i - 1] == 2 {
                match File::open("/dev/null") {
                    Ok(x) => {
                        let dev_null = x.into_raw_fd();
                        let pipe_in = unsafe { Stdio::from_raw_fd(dev_null) };
                        p.stdin(pipe_in);
                    }
                    Err(e) => {
                        println!("open dev null error: {:?}", e);
                    }
                }
            } else {
            */
                let fds_prev = pipes[i - 1];
                let pipe_in = unsafe { Stdio::from_raw_fd(fds_prev.0) };
                p.stdin(pipe_in);
            /*
            }
            */
        }

        // redirect output if needed
        if redirect != "" && i == length - 1 {
            let mut oos = OpenOptions::new();
            if append {
                oos.append(true);
            } else {
                oos.write(true);
                oos.truncate(true);
            }
            let fd = oos.create(true).open(redirect).unwrap().into_raw_fd();
            let file_out = unsafe { Stdio::from_raw_fd(fd) };
            p.stdout(file_out);
        }

        let mut child;
        match p.spawn() {
            Ok(x) => {
                child = x;
                if i != length - 1 {
                    children.push(child.id());
                }
            }
            Err(e) => {
                println!("child spawn error: {:?}", e);
                continue;
            }
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


#[cfg(test)]
mod tests {
    use super::args_to_cmds;

    #[test]
    fn test_args_to_cmd() {
        let s = vec![String::from("ls")];
        let result = args_to_cmds(s);
        let expected = vec![vec!["ls".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            String::from("ls"),
            String::from("|"),
            String::from("wc"),
        ];
        let result = args_to_cmds(s);
        let expected = vec![vec!["ls".to_string()], vec!["wc".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            String::from("  ls   "),
            String::from("-lh"),
            String::from("|"),
            String::from("wc  "),
            String::from("-l"),
            String::from("|"),
            String::from("  less"),
        ];
        let result = args_to_cmds(s);
        let expected = vec![
            vec!["ls".to_string(), "-lh".to_string()],
            vec!["wc".to_string(), "-l".to_string()],
            vec!["less".to_string()],
        ];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

    }
}
