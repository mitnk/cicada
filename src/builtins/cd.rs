use std::env;
use std::io::Write;
use std::path::Path;

use crate::parsers;
use crate::shell;
use crate::tools;

use crate::types::Tokens;

pub fn run(sh: &mut shell::Shell, tokens: &Tokens) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    if args.len() > 2 {
        println_stderr!("cicada: cd: too many argument");
        return 1;
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
            println_stderr!("no previous dir");
            return 1;
        }
        dir_to = sh.previous_dir.clone();
    } else if !dir_to.starts_with('/') {
        dir_to = format!("{}/{}", str_current_dir, dir_to);
    }

    if !Path::new(&dir_to).exists() {
        println_stderr!(
            "cicada: cd: {}: No such file or directory",
            args[1..].join("")
        );
        return 1;
    }

    match env::set_current_dir(&dir_to) {
        Ok(_) => {
            sh.current_dir = dir_to.clone();
            if str_current_dir != dir_to {
                sh.previous_dir = str_current_dir.clone();
                env::set_var("PWD", &sh.current_dir);
            };
            0
        }
        Err(e) => {
            println_stderr!("cicada: cd: {}", e);
            1
        }
    }
}
