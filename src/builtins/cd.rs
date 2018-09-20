use parsers;
use shell;
use std::env;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use tools;

pub fn run(sh: &mut shell::Shell, tokens: &Vec<(String, String)>) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    if args.len() > 2 {
        println_stderr!("invalid cd command");
        return 1;
    }
    let mut current_dir = PathBuf::new();
    match env::current_dir() {
        Ok(x) => current_dir = x,
        Err(e) => {
            println_stderr!("current_dir() failed: {}", e.description());
        }
    }
    let mut str_current_dir = "";
    match current_dir.to_str() {
        Some(x) => str_current_dir = x,
        None => {
            println_stderr!("current_dir to str failed.");
        }
    }
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
        println_stderr!("cicada: cd: {}: No such file or directory", args[1..].join(""));
        return 1
    }

    match env::set_current_dir(&dir_to) {
        Ok(_) => {
            if str_current_dir != dir_to {
                sh.previous_dir = str_current_dir.to_string();
            };
            0
        },
        Err(e) => {
            println_stderr!("cicada: cd: {}", e.description());
            1
        }
    }
}
