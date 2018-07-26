use parsers;
use shell;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use tools;

pub fn run(sh: &mut shell::Shell, tokens: &Vec<(String, String)>) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    if args.len() > 2 {
        println!("invalid cd command");
        return 1;
    }
    let mut current_dir = PathBuf::new();
    match env::current_dir() {
        Ok(x) => current_dir = x,
        Err(e) => {
            println!("current_dir() failed: {}", e.description());
        }
    }
    let mut str_current_dir = "";
    match current_dir.to_str() {
        Some(x) => str_current_dir = x,
        None => {
            println!("current_dir to str failed.");
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
            println!("no previous dir");
            return 0;
        }
        dir_to = sh.previous_dir.clone();
    } else if !dir_to.starts_with('/') {
        dir_to = format!("{}/{}", str_current_dir, dir_to);
    }
    if str_current_dir != dir_to {
        sh.previous_dir = str_current_dir.to_string();
    }
    match env::set_current_dir(&dir_to) {
        Ok(_) => 0,
        Err(e) => {
            println!("cd: {}", e.description());
            1
        }
    }
}
