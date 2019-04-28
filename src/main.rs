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
extern crate time;
extern crate yaml_rust;
#[macro_use]
extern crate nom;

use std::env;
use std::sync::Arc;

use linefeed::{Interface, ReadResult};

#[macro_use]
mod tools;

mod builtins;
mod completers;
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
        // to make cicada a job-control shell
        libc::signal(libc::SIGTSTP, libc::SIG_DFL);
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
            scripting::run_script(&mut sh, &args);
            return;
        }

        // this section handles `cicada -c 'echo hi && echo yoo'`,
        // e.g. it could be triggered from Vim (`:!ls` etc).
        if args[1] == "-c" {
            let line = tools::env_args_to_command_line();
            log!("run with -c args: {}", &line);
            execute::run_procs(&mut sh, &line, false);
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
        execute::handle_non_tty(&mut sh);
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

                let tsb_spec = time::get_time();
                let tsb = (tsb_spec.sec as f64) + tsb_spec.nsec as f64 / 1_000_000_000.0;

                let mut line = line.clone();
                tools::extend_bandband(&sh, &mut line);
                let status = execute::run_procs(&mut sh, &line, true);

                let tse_spec = time::get_time();
                let tse = (tse_spec.sec as f64) + tse_spec.nsec as f64 / 1_000_000_000.0;

                if !sh.cmd.starts_with(' ') && line != sh.previous_cmd {
                    history::add(&mut rl, &line, status, tsb, tse);
                    sh.previous_cmd = line.clone();
                }

                if line.trim().starts_with("alias ") || line.trim().starts_with("unalias ") {
                    // temporary solution for completion when sh alias changes
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
