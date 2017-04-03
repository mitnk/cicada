extern crate ansi_term;
extern crate errno;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate shlex;
extern crate sqlite;
extern crate time;
extern crate yaml_rust;

#[macro_use]
extern crate nom;

use std::env;
use std::rc::Rc;

// use std::thread;
// use std::time::Duration;

use ansi_term::Colour::{Red, Green};

use nom::IResult;
use regex::Regex;

use linefeed::Command;
use linefeed::{Reader, ReadResult};

mod builtins;
mod completers;
mod execute;
mod jobs;
mod parsers;
mod tools;
mod binds;
mod rcfile;
mod history;


fn main() {
    if env::args().len() > 1 {
        println!("does not support args yet.");
        return;
    }

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    let user = env::var("USER").unwrap();
    let home = tools::get_user_home();
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
    rcfile::load_rcfile();

    let mut previous_dir = String::new();
    let mut proc_status_ok = true;

    let mut rl = Reader::new("cicada").unwrap();
    history::init(&mut rl);
    rl.set_completer(Rc::new(completers::CCDCompleter));

    rl.define_function("up-key-function", Rc::new(binds::UpKeyFunction));
    rl.bind_sequence(binds::SEQ_UP_KEY, Command::from_str("up-key-function"));

    let mut painter;
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

        let prompt = tools::wrap_seq_chars(
            format!("\x01{}@{}: {}$ \x02",
                    painter.paint(user.to_string()),
                    painter.paint("cicada"),
                    painter.paint(pwd)));
        rl.set_prompt(prompt.as_str());

        if let Ok(ReadResult::Input(line)) = rl.read_line() {
            let cmd: String;
            if line.trim() == "exit" {
                break;
            } else if line.trim() == "" {
                continue;
            } else if line.trim() == "version" {
                println!("Cicada v{} by @mitnk", VERSION);
                continue;
            } else if line.trim() == "bash" {
                cmd = String::from("bash --rcfile ~/.bash_profile");
            } else {
                cmd = line.clone();
            }

            let tss_spec = time::get_time();
            let tss = (tss_spec.sec as f64) + tss_spec.nsec as f64 / 1000000000.0;
            history::add(&mut rl, line.as_str(), tss);

            let re;
            if let Ok(x) = Regex::new(r"^[ 0-9\.\(\)\+\-\*/]+$") {
                re = x;
            } else {
                println!("regex error for arithmetic");
                continue;
            }
            if re.is_match(line.as_str()) {
                if line.contains(".") {
                    match parsers::parser_float::expr_float(line.as_bytes()) {
                        IResult::Done(_, x) => {
                            println!("{:?}", x);
                        }
                        IResult::Error(x) => println!("Error: {:?}", x),
                        IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
                    }
                } else {
                    match parsers::parser_int::expr_int(line.as_bytes()) {
                        IResult::Done(_, x) => {
                            println!("{:?}", x);
                        }
                        IResult::Error(x) => println!("Error: {:?}", x),
                        IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
                    }
                }
                continue;
            }

            let mut args;
            if let Some(x) = shlex::split(cmd.trim()) {
                args = x;
            } else {
                println!("shlex split error: does not support multiple line");
                proc_status_ok = false;
                continue;
            }

            if args.len() == 0 {
                continue;
            }

            if args[0] == "cd" {
                let result = builtins::cd::run(args.clone(),
                                               home.as_str(),
                                               current_dir,
                                               &mut previous_dir);
                proc_status_ok = result == 0;
                continue;
            } else if args[0] == "export" {
                let result = builtins::export::run(line.as_str());
                proc_status_ok = result == 0;
                continue;
            } else {
                let mut background = false;
                let mut len = args.len();
                if len > 1 {
                    if args[len - 1] == "&" {
                        args.pop().expect("args pop error");
                        background = true;
                        len -= 1;
                    }
                }

                let result;
                if len > 2 && (args[len - 2] == ">" || args[len - 2] == ">>") {
                    let append = args[len - 2] == ">>";
                    let mut args_new = args.clone();
                    let redirect = args_new.pop().unwrap();
                    args_new.pop();
                    result = execute::run_pipeline(
                        args_new, redirect.as_str(), append, background);
                } else {
                    result = execute::run_pipeline(args.clone(), "", false, background);
                }
                proc_status_ok = result == 0;
                unsafe {
                    let gid = libc::getpgid(0);
                    tools::rlog(format!("try return term to {}\n", gid));
                    jobs::give_terminal_to(gid);
                }
                continue;
            }
        } else {
            println!("rl.read_line() error");
        }
    }
}
