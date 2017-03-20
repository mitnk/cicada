use std::process::{Command, Stdio};
use std::os::unix::io::FromRawFd;

use nix::unistd::pipe;
use nix::sys::signal;

extern fn handle_sigchld(_:i32) {
    // println!("child finished!: {}", x);
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
    let mut cmds: Vec<Vec <&str> > = Vec::new();
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
    // println!("cmds = {:?}", cmds);

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

    // let mut processes: Vec<Command> = Vec::new();
    i = 0;
    let mut status = 0;
    for cmd in &cmds {
        let mut p = Command::new(cmd[0]);
        p.args(&cmd[1..]);
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
        } else {
            println!("child spawn error");
            return 1;
        }

        let ecode;
        if let Ok(x) = child.wait() {
            ecode = x;
        } else {
            println!("child wait error");
            return 1;
        }
        if !ecode.success() {
            status = 1;
        }
        // processes.push(p);
        i += 1;
    }
    status
}
