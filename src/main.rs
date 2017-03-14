extern crate ansi_term;
extern crate rustyline;
extern crate shlex;
extern crate libc;
extern crate errno;

use std::env;
use std::os::unix::process::CommandExt;
use std::process::Command;
// use std::thread;
// use std::time::Duration;

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
    let home = env::var("HOME").unwrap();
    let env_path = env::var("PATH").unwrap();
    let dir_bin_cargo = format!("{}/.cargo/bin", home);
    let env_path_new = [
        "/usr/local/bin".to_string(),
        env_path,
        dir_bin_cargo,
    ].join(":");
    env::set_var("PATH", &env_path_new);

    let mut previous_dir = String::new();
    let mut proc_status_ok = true;
    let mut painter;
    let mut rl = Editor::<()>::new();
    loop {
        if proc_status_ok {painter = Green;} else {painter = Red;}

        // TODO: clean these mess up
        let current_dir = env::current_dir().unwrap();
        let current_dir = current_dir.to_string_lossy();
        let tokens: Vec<&str> = current_dir.split("/").collect();
        let last = tokens.last().unwrap();
        let pwd: String;
        if last.to_string() == "" {
            pwd = String::from("/");
        } else if current_dir == home {
            pwd = String::from("~");
        } else {
            pwd = last.to_string();
        }
        let prompt = format!("{}@{}: {}$ ",
                         painter.paint(user.to_string()),
                         painter.paint("RUSH"),
                         painter.paint(pwd));
        let cmd = rl.readline(&prompt);
        match cmd {
            Ok(line) => {
                let cmd: String;
                if line.trim() == "exit" {
                    break;
                } else if line.trim() == "" {
                    continue;
                } else if line.trim() == "bash" {
                    cmd = String::from("bash --rcfile ~/.bash_profile");
                } else {
                    cmd = line.to_string();
                }

                rl.add_history_entry(&cmd);
                let args = shlex::split(cmd.trim()).unwrap();
                if args[0] == "cd" {
                    if args.len() > 2 {
                        println!("invalid cd command");
                        proc_status_ok = false;
                        continue;
                    } else {
                        let mut path: String;
                        if args.len() == 1 {
                            path = home.to_string();
                        } else {
                            path = args[1..].join("");
                        }
                        if path == "-"{
                            if previous_dir == "" {
                                println!("no previous dir");
                                continue;
                            }
                            path = previous_dir.clone();
                        } else {
                            if !path.starts_with("/") {
                                path = format!("{}/{}", tokens.join("/"), path);
                            }
                        }
                        if current_dir != path {
                            previous_dir = current_dir.to_string();
                        }
                        match env::set_current_dir(&path) {
                            Ok(_) => {
                                proc_status_ok = true;
                                continue;
                            },
                            Err(e) => {
                                proc_status_ok = false;
                                println!("{:?}", e);
                                continue;
                            }
                        }
                    }
                }

                tools::rlog(format!("run {:?}\n", args));
                let mut child;
                match Command::new(&args[0]).args(&(args[1..]))
                    .before_exec(|| {
                        unsafe {
                            let pid = libc::getpid();
                            libc::setpgid(0, pid);
                        }
                        Ok(())
                    })
                    .spawn() {
                    Ok(x) => child = x,
                    Err(e) => {
                        proc_status_ok = false;
                        println!("{:?}", e);
                        continue;
                    }
                }
                unsafe {
                    let pid = child.id() as i32;
                    let gid = libc::getpgid(pid);
                    // thread::sleep(Duration::from_millis(1));
                    tools::rlog(format!("try give term to {}\n", gid));
                    jobs::give_terminal_to(gid);
                    tools::rlog(format!("waiting pid {}\n", gid));
                }
                let ecode = child.wait().unwrap();
                proc_status_ok = ecode.success();
                tools::rlog(format!("done. ok: {}\n", proc_status_ok));
                unsafe {
                    let gid = libc::getpgid(0);
                    tools::rlog(format!("try give term to {}\n", gid));
                    jobs::give_terminal_to(gid);
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
