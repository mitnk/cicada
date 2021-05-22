use std::env;
use std::path::Path;

use crate::builtins::utils::print_stderr_with_capture;
use crate::parsers;
use crate::shell;
use crate::tools;
use crate::types::{Command, CommandLine, CommandResult};

pub fn run(sh: &mut shell::Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let tokens = cmd.tokens.clone();
    let mut cr = CommandResult::new();
    let args = parsers::parser_line::tokens_to_args(&tokens);

    if args.len() > 2 {
        let info = "cicada: cd: too many argument";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        return cr;
    }

    let str_current_dir = tools::get_current_dir();

    let mut dir_to = if args.len() == 1 {
        let home = tools::get_user_home();
        home.to_string()
    } else {
        args[1..].join("")
    };

    if dir_to == "-" {
        if sh.previous_dir == "" {
            let info = "no previous dir";
            print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
            return cr;
        }
        dir_to = sh.previous_dir.clone();
    } else if !dir_to.starts_with('/') {
        dir_to = format!("{}/{}", str_current_dir, dir_to);
    }

    if !Path::new(&dir_to).exists() {
        let info = format!("cicada: cd: {}: No such file or directory", &args[1]);
        print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
        return cr;
    }

    match Path::new(&dir_to).canonicalize() {
        Ok(p) => {
            dir_to = p.as_path().to_string_lossy().to_string();
        }
        Err(e) => {
            let info = format!("cicada: cd: error: {}", e);
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
        }
    }

    match env::set_current_dir(&dir_to) {
        Ok(_) => {
            sh.current_dir = dir_to.clone();
            if str_current_dir != dir_to {
                sh.previous_dir = str_current_dir.clone();
                env::set_var("PWD", &sh.current_dir);
            };
            cr.status = 0;
            cr
        }
        Err(e) => {
            let info = format!("cicada: cd: {}", e);
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            cr
        }
    }
}
