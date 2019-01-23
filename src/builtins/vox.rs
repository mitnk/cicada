use std::env;
use std::fs::{self, read_dir};
use std::io::Write;
use std::path::Path;

use crate::parsers;
use crate::shell;
use crate::types;

fn in_env() -> bool {
    if let Ok(x) = env::var("VIRTUAL_ENV") {
        if x != "" {
            return true;
        }
    }
    false
}

fn get_envs_home() -> String {
    let home_envs;
    match env::var("VIRTUALENV_HOME") {
        Ok(x) => {
            home_envs = x;
        }
        Err(_) => {
            home_envs = String::new();
        }
    }
    home_envs
}

fn list_envs() -> i32 {
    let home_envs = get_envs_home();
    if home_envs == "" {
        println_stderr!("you need to set VIRTUALENV_HOME to use vox");
        return 1;
    }
    if !Path::new(home_envs.as_str()).exists() {
        match fs::create_dir_all(home_envs.as_str()) {
            Ok(_) => {}
            Err(e) => println_stderr!("fs create_dir_all failed: {:?}", e),
        }
    }

    let pdir = home_envs.clone();
    if let Ok(list) = read_dir(home_envs) {
        for ent in list {
            if let Ok(ent) = ent {
                let ent_name = ent.file_name();
                if let Ok(path) = ent_name.into_string() {
                    let full_path = format!("{}/{}/bin/activate", pdir, path);
                    if !Path::new(full_path.as_str()).exists() {
                        continue;
                    }
                    println!("{}", path);
                }
            }
        }
    }
    0
}

fn enter_env(sh: &shell::Shell, path: &str) -> i32 {
    if in_env() {
        println_stderr!("vox: already in env");
        return 1;
    }
    let home_envs = get_envs_home();
    let full_path = format!("{}/{}/bin/activate", home_envs, path);
    if !Path::new(full_path.as_str()).exists() {
        println_stderr!("no such env: {}", full_path);
        return 1;
    }
    let path_env = format!("{}/{}", home_envs, path);
    env::set_var("VIRTUAL_ENV", &path_env);
    let path_new = String::from("${VIRTUAL_ENV}/bin:$PATH");
    let mut tokens: types::Tokens = Vec::new();
    tokens.push((String::new(), path_new));
    shell::expand_env(sh, &mut tokens);
    env::set_var("PATH", &tokens[0].1);
    0
}

fn exit_env(sh: &shell::Shell) -> i32 {
    if !in_env() {
        println_stderr!("vox: not in an env");
        return 0;
    }
    let env_path;
    match env::var("PATH") {
        Ok(x) => env_path = x,
        Err(_) => {
            println_stderr!("vox: cannot read PATH env");
            return 1;
        }
    }
    let mut _tokens: Vec<&str> = env_path.split(':').collect();
    let mut path_virtual_env = String::from("${VIRTUAL_ENV}/bin");
    // shell::extend_env(sh, &mut path_virtual_env);
    let mut tokens: types::Tokens = Vec::new();
    tokens.push((String::new(), path_virtual_env));
    shell::expand_env(sh, &mut tokens);
    path_virtual_env = tokens[0].1.clone();
    _tokens
        .iter()
        .position(|&n| n == path_virtual_env)
        .map(|e| _tokens.remove(e));
    let env_path_new = _tokens.join(":");
    env::set_var("PATH", &env_path_new);
    env::set_var("VIRTUAL_ENV", "");
    0
}

pub fn run(sh: &shell::Shell, tokens: &types::Tokens) -> i32 {
    let args = parsers::parser_line::tokens_to_args(tokens);
    let len = args.len();
    if len == 1 {
        return list_envs();
    }

    let subcmd = &args[1];
    if len == 2 && subcmd == "ls" {
        list_envs()
    } else if len == 3 && subcmd == "enter" {
        enter_env(sh, args[2].as_str())
    } else if len == 2 && subcmd == "exit" {
        exit_env(sh)
    } else {
        println_stderr!("vox: invalid command");
        println_stderr!("usage: vox (ls | enter <env-name> | exit)");
        1
    }
}
