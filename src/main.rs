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
extern crate yaml_rust;

extern crate clap;

#[macro_use]
extern crate lazy_static;
extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::env;
use std::io::Write;
use std::sync::Arc;

use linefeed::{Command, Interface, ReadResult};

#[macro_use]
mod tlog;
#[macro_use]
mod tools;

mod builtins;
mod calculator;
mod completers;
mod core;
mod ctime;
mod execute;
mod history;
mod jobc;
mod libs;
mod parsers;
mod prompt;
mod rcfile;
mod scripting;
mod shell;
mod signals;
mod types;

// #[allow(clippy::cast_lossless)]
fn main() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);

        // ignore SIGTSTP (ctrl-Z) for the shell itself
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
    }

    tools::init_path_env();

    let mut sh = shell::Shell::new();
    let args: Vec<String> = env::args().collect();
    // only load RC in a login shell
    if args.len() > 0 && args[0].starts_with("-") {
        rcfile::load_rc_files(&mut sh);
        sh.is_login = true;
    }

    if args.len() > 1 {
        if !args[1].starts_with("-") {
            log!("run script: {:?} ", &args);
            scripting::run_script(&mut sh, &args);
            return;
        }

        // this section handles `cicada -c 'echo hi && echo yoo'`,
        // e.g. it could be triggered from Vim (`:!ls` etc).
        if args[1] == "-c" {
            let line = tools::env_args_to_command_line();
            log!("run with -c args: {}", &line);
            execute::run_command_line(&mut sh, &line, false, false);
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
            println!("cicada: linefeed error: {}", e);
            return;
        }
    }

    rl.define_function("enter-function", Arc::new(prompt::EnterFunction));
    rl.bind_sequence("\r", Command::from_str("enter-function"));

    history::init(&mut rl);
    rl.set_completer(Arc::new(completers::CicadaCompleter {
        sh: Arc::new(sh.clone()),
    }));

    let sig_handler_enabled = tools::is_signal_handler_enabled();
    if sig_handler_enabled {
        signals::setup_sigchld_handler();
        // block the signals at most of time, since Rust is not "async-signal-safe"
        // yet. see https://github.com/rust-lang/rfcs/issues/1368
        // we'll unblock them when necessary only.
        signals::block_signals();
    }

    loop {
        let prompt = prompt::get_prompt(&sh);
        match rl.set_prompt(&prompt) {
            Ok(_) => {}
            Err(e) => {
                println_stderr!("cicada: prompt error: {}", e);
            }
        }

        if sig_handler_enabled {
            // FIXME: in `rl.read_line()` below, there is lots of Rust code,
            // which may not be async-signal-safe. see follow links for details:
            // - https://ldpreload.com/blog/signalfd-is-useless
            // - https://man7.org/linux/man-pages/man7/signal-safety.7.html
            signals::unblock_signals();
        }
        match rl.read_line() {
            Ok(ReadResult::Input(line)) => {
                if sig_handler_enabled {
                    signals::block_signals();
                }

                let line = shell::trim_multiline_prompts(&line);
                if line.trim() == "" {
                    jobc::try_wait_bg_jobs(&mut sh, true, sig_handler_enabled);
                    continue;
                }
                sh.cmd = line.clone();

                let tsb = ctime::DateTime::now().unix_timestamp();
                let mut line = line.clone();

                // since `!!` expansion is only meaningful in an interactive
                // shell we extend it here, instead of in `run_command_line()`.
                tools::extend_bangbang(&sh, &mut line);

                let mut status = 0;
                let cr_list = execute::run_command_line(&mut sh, &line, true, false);
                if let Some(last) = cr_list.last() {
                    status = last.status;
                }
                let tse = ctime::DateTime::now().unix_timestamp();

                if !sh.cmd.starts_with(' ') && line != sh.previous_cmd {
                    history::add(&sh, &mut rl, &line, status, tsb, tse);
                    sh.previous_cmd = line.clone();
                }

                if tools::is_shell_altering_command(&line) {
                    // since our shell object need to be passed into
                    // `linefeed::Completer` with an Arc.
                    // I currently do not know how to share the same sh
                    // instance at hand with it.

                    // update the Arc clone when alias/function/env changes
                    rl.set_completer(Arc::new(completers::CicadaCompleter {
                        sh: Arc::new(sh.clone()),
                    }));
                }

                jobc::try_wait_bg_jobs(&mut sh, true, sig_handler_enabled);
                continue;
            }
            Ok(ReadResult::Eof) => {
                if let Ok(x) = env::var("NO_EXIT_ON_CTRL_D") {
                    if x == "1" {
                        println!();
                    }
                } else {
                    println!("exit");
                    break;
                }
            }
            Ok(ReadResult::Signal(s)) => {
                println_stderr!("readline signal: {:?}", s);
            }
            Err(e) => {
                println_stderr!("readline error: {}", e);
            }
        }
        if sig_handler_enabled {
            signals::block_signals();
        }
    }
}
