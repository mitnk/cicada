#![allow(unknown_lints)]
extern crate errno;
extern crate exec;
extern crate glob;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate sqlite;
extern crate time;
extern crate yaml_rust;
#[macro_use]
extern crate nom;

use std::env;
use std::rc::Rc;
use linefeed::{Reader, ReadResult};

#[macro_use]
mod tools;
mod builtins;
mod completers;
mod execute;
mod libs;
mod parsers;
mod history;
mod rcfile;
mod shell;

use tools::clog;

pub use tools::CommandResult;

fn main() {
    let mut sh = shell::Shell::new();
    rcfile::load_rcfile(&mut sh);

    // this section handles `cicada -c 'echo hi && echo yoo'`,
    // e.g. it could be triggered from Vim (`:!ls` etc).
    if env::args().len() > 1 {
        let line = tools::env_args_to_command_line();
        log!("run with -c args: {}", &line);
        execute::run_procs(&mut sh, &line, false);
        return;
    }

    let isatty: bool = unsafe { libc::isatty(0) == 1 };
    if !isatty {
        // cases like open a new MacVim window,
        // (i.e. CMD+N) on an existing one
        execute::handle_non_tty(&mut sh);
        return;
    }

    let mut rl;
    match Reader::new("cicada") {
        Ok(x) => rl = x,
        Err(e) => {
            // non-tty will raise errors here
            println!("Reader Error: {:?}", e);
            return;
        }
    }
    history::init(&mut rl);
    rl.set_completer(Rc::new(completers::CicadaCompleter));

    let mut status = 0;
    loop {
        let prompt = libs::prompt::get_prompt(status);
        rl.set_prompt(&prompt);
        match rl.read_line() {
            Ok(ReadResult::Input(line)) => {
                if line.trim() == "" {
                    continue;
                }

                let tsb_spec = time::get_time();
                let tsb = (tsb_spec.sec as f64) + tsb_spec.nsec as f64 / 1000000000.0;

                let mut line = line.clone();
                tools::extend_bandband(&sh, &mut line);
                status = execute::run_procs(&mut sh, &line, true);

                let tse_spec = time::get_time();
                let tse = (tse_spec.sec as f64) + tse_spec.nsec as f64 / 1000000000.0;
                history::add(&mut sh, &mut rl, &line, status, tsb, tse);
            }
            Ok(ReadResult::Eof) => {
                if let Ok(x) = env::var("NO_EXIT_ON_CTRL_D") {
                    if x == "1" {
                        println!("");
                        continue;
                    }
                }
                println!("exit");
                break;
            }
            Ok(ReadResult::Signal(s)) => {
                println!("readline signal: {:?}", s);
            }
            Err(e) => {
                println!("readline error: {:?}", e);
            }
        }
    }
}
