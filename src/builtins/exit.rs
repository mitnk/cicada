#![allow(unreachable_code)]
use std::process;

use crate::builtins::utils::print_stderr_with_capture;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(sh: &Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = cmd.tokens.clone();
    if tokens.len() > 2 {
        let info = "cicada: exit: too many arguments";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }

    if tokens.len() == 2 {
        let _code = &tokens[1].1;
        match _code.parse::<i32>() {
            Ok(x) => {
                process::exit(x);
            }
            Err(_) => {
                let info = format!("cicada: exit: {}: numeric argument required", _code);
                print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                process::exit(255);
            }
        }
    }

    for (_i, job) in sh.jobs.iter() {
        if !job.cmd.starts_with("nohup ") {
            let mut info = String::new();
            info.push_str("There are background jobs.");
            info.push_str("Run `jobs` to see details; `exit 1` to force quit.");
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
        }
    }

    process::exit(0);
    cr
}
