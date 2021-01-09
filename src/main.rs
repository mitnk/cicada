#![allow(unknown_lints)]
// #![feature(tool_lints)]
extern crate errno;
extern crate exec;
extern crate glob;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate rusqlite;
extern crate chrono;
extern crate yaml_rust;

extern crate clap;

#[macro_use]
extern crate lazy_static;
extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::env;
use std::sync::Arc;

use chrono::prelude::Local;
use linefeed::{Interface, ReadResult};

#[macro_use]
mod tools;

mod builtins;
mod calculator;
mod completers;
mod core;
mod execute;
mod history;
mod jobc;
mod libs;
mod parsers;
mod prompt;
mod rcfile;
mod scripting;
mod shell;
mod types;

use crate::tools::clog;

// #[allow(clippy::cast_lossless)]
fn main() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);

        // ignore SIGTSTP (ctrl-Z) for the shell itself
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
    }

    let mut sh = shell::Shell::new();

    let args: Vec<String> = env::args().collect();

    // only load RC in a login shell
    if args.len() > 0 && args[0].starts_with("-") {
        rcfile::load_rc_files(&mut sh);
        sh.is_login = true;
    }

    if args.len() > 1 {
        if !args[1].starts_with("-") {
            log!("run script: {:?}", &args);
            scripting::run_script(&mut sh, &args);
            return;
        }

        // this section handles `cicada -c 'echo hi && echo yoo'`,
        // e.g. it could be triggered from Vim (`:!ls` etc).
        if args[1] == "-c" {
            let line = tools::env_args_to_command_line();
            log!("run with -c args: {}", &line);
            execute::run_procs(&mut sh, &line, false, false);
            return;
        }

        if args[1] == "--login" || args[1] == "-l" {
            rcfile::load_rc_files(&mut sh);
            sh.is_login = true;
        }
    }

    let isatty: bool = unsafe { libc::isatty(0) == 1 };
    if !isatty {
        // cases like open a new MacVim window,
        // (i.e. CMD+N) on an existing one
        execute::run_procs_for_non_tty(&mut sh);
        return;
    }

    let mut rl;
    match Interface::new("cicada") {
        Ok(x) => rl = x,
        Err(e) => {
            // non-tty will raise errors here
            println!("linefeed Interface Error: {:?}", e);
            return;
        }
    }
    history::init(&mut rl);
    rl.set_completer(Arc::new(completers::CicadaCompleter {
        sh: Arc::new(sh.clone()),
    }));

    loop {
        let prompt = prompt::get_prompt(&sh);
        match rl.set_prompt(&prompt) {
            Ok(_) => {}
            Err(e) => {
                println!("error when setting prompt: {:?}\n", e);
            }
        }
        match rl.read_line() {
            Ok(ReadResult::Input(line)) => {
                jobc::try_wait_bg_jobs(&mut sh);

                if line.trim() == "" {
                    continue;
                }
                sh.cmd = line.clone();

                let tsb = Local::now().timestamp_nanos() as f64 / 1000000000.0;
                let mut line = line.clone();

                // since `!!` expansion is only meaningful in an interactive
                // shell we extend it here, instead of in `run_procs()`.
                tools::extend_bangbang(&sh, &mut line);

                let mut status = 0;
                let cr_list = execute::run_procs(&mut sh, &line, true, false);
                if let Some(last) = cr_list.last() {
                    status = last.status;
                }
                let tse = Local::now().timestamp_nanos() as f64 / 1000000000.0;

                if !sh.cmd.starts_with(' ') && line != sh.previous_cmd {
                    history::add(&sh, &mut rl, &line, status, tsb, tse);
                    sh.previous_cmd = line.clone();
                }

                // temporary solution for completion when alias/funcs changes
                if line.trim().starts_with("alias ") ||
                        line.trim().starts_with("unalias ") ||
                        line.trim().starts_with("source ") {
                    rl.set_completer(Arc::new(completers::CicadaCompleter {
                        sh: Arc::new(sh.clone()),
                    }));
                }
            }
            Ok(ReadResult::Eof) => {
                if let Ok(x) = env::var("NO_EXIT_ON_CTRL_D") {
                    if x == "1" {
                        println!();
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
