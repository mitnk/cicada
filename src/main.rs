extern crate ansi_term;
extern crate rustyline;
extern crate shlex;
extern crate libc;
extern crate errno;

use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::fs::OpenOptions;
use std::io::Write;

use ansi_term::Colour::Red;
use ansi_term::Colour::Green;
use errno::errno;
use rustyline::Editor;
use rustyline::error::ReadlineError;


fn rlog(s: String) {
    let mut file =
        OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/rlog.log")
        .unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

unsafe fn give_terminal_to(pid: i32) {
    let mut mask: libc::sigset_t = 0;
    let mut old_mask: libc::sigset_t = 0;

    libc::sigemptyset(&mut mask);
    libc::sigaddset(&mut mask, libc::SIGTSTP);
    libc::sigaddset(&mut mask, libc::SIGTTIN);
    libc::sigaddset(&mut mask, libc::SIGTTOU);
    libc::sigaddset(&mut mask, libc::SIGCHLD);

    let rcode = libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut old_mask);
    if rcode != 0 {
        rlog(format!("failed to call pthread_sigmask\n"));
    }
    let rcode = libc::tcsetpgrp(1, pid);
    if rcode == -1 {
        let e = errno();
        let code = e.0;
        rlog(format!("Error {}: {}\n", code, e));
    } else {
        rlog(format!("return term back to {} rcode: {}\n", pid, rcode));
    }
    let rcode = libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, &mut mask);
    if rcode != 0 {
        rlog(format!("failed to call pthread_sigmask\n"));
    }
}


fn main() {
    println!("RUSH v0.1.1");
    let user = env::var("USER").unwrap();
    let mut proc_status_ok = true;
    let mut prompt;
    let mut painter;
    let mut rl = Editor::<()>::new();
    loop {
        if proc_status_ok {painter = Green;} else {painter = Red;}
        prompt = format!("{}@{} ",
                         painter.paint(user.to_string()),
                         painter.paint("RUSH: ~$"));

        let cmd = rl.readline(&prompt);
        match cmd {
            Ok(line) => {
                if line.trim() == "exit" {
                    println!("Bye.");
                    break;
                } else if line.trim() == "" {
                    continue;
                }
                rl.add_history_entry(&line);

                let args = shlex::split(line.trim()).unwrap();
                let mut child;
                match Command::new(&args[0]).args(&(args[1..]))
                    .before_exec(
                        || {
                            unsafe {
                                let pid = libc::getpid();
                                libc::setpgid(0, pid);
                            }
                            Ok(())
                        }
                    )
                    .spawn() {
                    Ok(x) => child = x,
                    Err(e) => {
                        proc_status_ok = false;
                        println!("{:?}", e);
                        continue
                    }
                }
                unsafe {
                    let pid = child.id() as i32;
                    rlog(format!("try give term to {}\n", pid));
                    give_terminal_to(pid);
                }
                rlog("waiting\n".to_string());
                let ecode = child.wait().unwrap();
                rlog("done\n".to_string());
                proc_status_ok = ecode.success();
                rlog("before unsafe\n".to_string());
                unsafe {
                    let pid = libc::getpid();
                    rlog(format!("try give term to {}\n", pid));
                    give_terminal_to(pid);
                }
            },
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                continue;
            },
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                continue;
            },
            Err(err) => {
                println!("RL Error: {:?}", err);
                continue;
            }
        }
    }
}
