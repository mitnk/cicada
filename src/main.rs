extern crate ansi_term;
extern crate rustyline;
extern crate shlex;
extern crate sqlite;
extern crate libc;
extern crate errno;
extern crate regex;
extern crate nix;

#[macro_use]
extern crate nom;

use std::env;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

// use std::thread;
// use std::time::Duration;

use ansi_term::Colour::Red;
use ansi_term::Colour::Green;

use rustyline::completion::FilenameCompleter;
use rustyline::error::ReadlineError;
use rustyline::{Config, CompletionType, Editor};

// use rustyline::Editor;
// use rustyline::error::ReadlineError;
use nom::IResult;
use regex::Regex;


mod jobs;
mod tools;
mod parsers;
mod builtins;
mod execute;


fn main() {
    if env::args().len() > 1 {
        println!("does not support args yet.");
        return;
    }

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("##### Welcome to RUSH v{} #####", VERSION);

    let user = env::var("USER").unwrap();
    let home = env::var("HOME").unwrap();
    let env_path = env::var("PATH").unwrap();
    let dir_bin_cargo = format!("{}/.cargo/bin", home);
    let env_path_new = ["/usr/local/bin".to_string(),
                        env_path,
                        dir_bin_cargo,
                        "/Library/Frameworks/Python.framework/Versions/3.6/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/3.5/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/3.4/bin".to_string(),
                        "/Library/Frameworks/Python.framework/Versions/2.7/bin".to_string()]
            .join(":");
    env::set_var("PATH", &env_path_new);

    let mut previous_dir = String::new();
    let mut proc_status_ok = true;
    let mut painter;

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();
    let mut rl = Editor::with_config(config);
    let c = FilenameCompleter::new();
    rl.set_completer(Some(c));
    rl.get_history().set_max_len(9999);  // make bigger, but not huge

    let file_db = format!("{}/{}", home, ".local/share/xonsh/xonsh-history.sqlite");
    if Path::new(file_db.as_str()).exists() {
        let conn = sqlite::open(file_db).unwrap();
        conn.iterate("SELECT DISTINCT inp FROM xonsh_history ORDER BY ROWID;", |pairs| {
            for &(_, value) in pairs.iter() {
                let inp = value.unwrap();
                rl.add_history_entry(inp.as_ref());
            }
            true
        }).unwrap();
    }

    loop {
        if proc_status_ok {
            painter = Green;
        } else {
            painter = Red;
        }

        let _current_dir = env::current_dir().unwrap();
        let current_dir = _current_dir.to_str().unwrap();
        let _tokens: Vec<&str> = current_dir.split("/").collect();

        let last = _tokens.last().unwrap();
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
                rl.add_history_entry(cmd.as_ref());
                let re;
                if let Ok(x) = Regex::new(r"^ *\(* *[0-9\.]+") {
                    re = x;
                } else {
                    println!("regex error");
                    continue;
                }
                if re.is_match(line.as_str()) {
                    match parsers::expr(line.as_bytes()) {
                        IResult::Done(_, x) => {
                            println!("{:?}", x);
                        }
                        IResult::Error(x) => println!("Error: {:?}", x),
                        IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
                    }
                    continue;
                }

                let args;
                if let Some(x) = shlex::split(cmd.trim()) {
                    args = x;
                } else {
                    println!("shlex split error: does not support multiple line");
                    proc_status_ok = false;
                    continue;
                }
                if args[0] == "cd" {
                    let result = builtins::cd::run(args.clone(),
                                                   home.as_str(),
                                                   current_dir,
                                                   &mut previous_dir);
                    proc_status_ok = result == 0;
                    continue;
                } else if args.iter().any(|x| x == "|") {
                    let result = execute::run_pipeline(args.clone());
                    proc_status_ok = result == 0;
                    continue;
                }

                tools::rlog(format!("run {:?}\n", args));
                let mut child;
                match Command::new(&args[0])
                          .args(&(args[1..]))
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
                    tools::rlog(format!("try give term to {}\n", gid));
                    jobs::give_terminal_to(gid);
                    tools::rlog(format!("waiting pid {}\n", gid));
                }

                let ecode;
                if let Ok(x) = child.wait() {
                    ecode = x;
                } else {
                    println!("child wait error.");
                    proc_status_ok = false;
                    continue;
                }

                proc_status_ok = ecode.success();
                tools::rlog(format!("done. ok: {}\n", proc_status_ok));
                unsafe {
                    let gid = libc::getpgid(0);
                    tools::rlog(format!("try give term to {}\n", gid));
                    jobs::give_terminal_to(gid);
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                continue;
            }
            Err(err) => {
                println!("RL Error: {:?}", err);
                continue;
            }
        }
    }
}
