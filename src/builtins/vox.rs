use std::env;
use std::fs::{self, read_dir};
use std::path::Path;

use tools;

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
        fs::create_dir_all(home_envs.as_str()).expect("dirs env create failed");
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

fn enter_env(path: &str) -> i32 {
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
    let mut path_new = String::from("${VIRTUAL_ENV}/bin:$PATH");
    tools::extend_env(&mut path_new);
    env::set_var("PATH", &path_new);
    0
}

fn exit_env() -> i32 {
    if !in_env() {
        println!("vox: not in an env");
        return 0;
    }
    let env_path = env::var("PATH").expect("vox: env error");
    let mut _tokens: Vec<&str> = env_path.split(':').collect();
    let mut path_virtual_env = String::from("${VIRTUAL_ENV}/bin");
    tools::extend_env(&mut path_virtual_env);
    _tokens
        .iter()
        .position(|&n| n == path_virtual_env)
        .map(|e| _tokens.remove(e));
    let env_path_new = _tokens.join(":");
    env::set_var("PATH", &env_path_new);
    env::set_var("VIRTUAL_ENV", "");
    0
}

#[allow(needless_pass_by_value)]
pub fn run(args: Vec<String>) -> i32 {
    if args.len() == 2 && args[1] == "ls" {
        list_envs()
    } else if args.len() == 3 && args[1] == "enter" {
        enter_env(args[2].as_str())
    } else if args.len() == 2 && args[1] == "exit" {
        exit_env()
    } else {
        println!("vox: invalid command");
        println!("vox (ls | enter <env-name> | exit)");
        1
    }
}
