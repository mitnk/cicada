#![allow(unreachable_code)]
use std::io::Write;
use std::process;

use types::Tokens;
use shell;

pub fn run(sh: &shell::Shell, tokens: &Tokens) -> i32 {
    if tokens.len() > 2 {
        println_stderr!("cicada: exit: too many arguments");
        return 1;
    }

    if tokens.len() == 2 {
        let _code = &tokens[1].1;
        match _code.parse::<i32>() {
            Ok(x) => {
                process::exit(x);
            }
            Err(_) => {
                println_stderr!("cicada: exit: {}: numeric argument required", _code);
                process::exit(255);
            }
        }
    }

    for (_i, job) in sh.jobs.iter() {
        if !job.cmd.starts_with("nohup ") {
            println_stderr!("There are background jobs.");
            println_stderr!("Use command `jobs` to see more details.");
            println_stderr!("Use `exit 1` to force quit.");
            return 0;
        }
    }

    process::exit(0);
}
