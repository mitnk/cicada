extern crate ansi_term;
extern crate rustyline;
extern crate shlex;
extern crate libc;
extern crate errno;

use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;

use ansi_term::Colour::Red;
use ansi_term::Colour::Green;
use rustyline::Editor;
use rustyline::error::ReadlineError;

mod jobs;
mod tools;


fn main() {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("##### Welcome to RUSH v{} #####", VERSION);

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
                    tools::rlog(format!("try give term to {}\n", pid));
                    jobs::give_terminal_to(pid);
                    tools::rlog(format!("waiting pid {}\n", pid));
                }
                let ecode = child.wait().unwrap();
                proc_status_ok = ecode.success();
                tools::rlog(format!("done. ok: {}\n", proc_status_ok));
                unsafe {
                    let pid = libc::getpid();
                    tools::rlog(format!("try give term to {}\n", pid));
                    jobs::give_terminal_to(pid);
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
