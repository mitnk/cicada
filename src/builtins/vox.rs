use std::env;
use std::fs::{self, read_dir};
use std::path::Path;

use parsers;
use shell;

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
        println!("you need to set VIRTUALENV_HOME to use vox");
        return 1;
    }
    if !Path::new(home_envs.as_str()).exists() {
        match fs::create_dir_all(home_envs.as_str()) {
            Ok(_) => {}
            Err(e) => println!("fs create_dir_all failed: {:?}", e),
        }
    }

    println!("Envs under: {}", home_envs);
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
        println!("vox: already in env");
        return 1;
    }
    let home_envs = get_envs_home();
    let full_path = format!("{}/{}/bin/activate", home_envs, path);
    if !Path::new(full_path.as_str()).exists() {
        println!("no such env: {}", full_path);
        return 1;
    }
    let path_env = format!("{}/{}", home_envs, path);
    env::set_var("VIRTUAL_ENV", &path_env);
    let path_new = String::from("${VIRTUAL_ENV}/bin:$PATH");
    let mut tokens: Vec<(String, String)> = Vec::new();
    tokens.push((String::new(), path_new));
    shell::expand_env(sh, &mut tokens);
    env::set_var("PATH", &tokens[0].1);
    0
}

fn exit_env(sh: &shell::Shell) -> i32 {
    if !in_env() {
        println!("vox: not in an env");
        return 0;
    }
    let env_path;
    match env::var("PATH") {
        Ok(x) => env_path = x,
        Err(_) => {
            println!("vox: cannot read PATH env");
            return 1;
        }
    }
    let mut _tokens: Vec<&str> = env_path.split(':').collect();
    let mut path_virtual_env = String::from("${VIRTUAL_ENV}/bin");
    // shell::extend_env(sh, &mut path_virtual_env);
    let mut tokens: Vec<(String, String)> = Vec::new();
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

pub fn run(sh: &shell::Shell, tokens: &Vec<(String, String)>) -> i32 {
    let args = parsers::parser_line::tokens_to_args(&tokens);
    if args.len() == 2 && args[1] == "ls" {
        list_envs()
    } else if args.len() == 3 && args[1] == "enter" {
        enter_env(sh, args[2].as_str())
    } else if args.len() == 2 && args[1] == "exit" {
        exit_env(sh)
    } else {
        println!("vox: invalid command");
        println!("usage: vox (ls | enter <env-name> | exit)");
        1
    }
}
