use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::builtins::utils::print_stderr_with_capture;
use crate::builtins::utils::print_stdout_with_capture;
use crate::execute;
use crate::parsers;
use crate::shell::{self, Shell};
use crate::types::{self, Command, CommandLine, CommandResult};

fn in_env() -> bool {
    env::var("VIRTUAL_ENV").is_ok_and(|x| !x.is_empty())
}

fn get_envs_home() -> String {
    env::var("VIRTUALENV_HOME").unwrap_or_default()
}

fn get_all_venvs() -> Result<Vec<String>, String> {
    let home_envs = get_envs_home();
    if home_envs.is_empty() {
        let info = String::from("you need to set VIRTUALENV_HOME to use vox");
        return Err(info);
    }
    if !Path::new(home_envs.as_str()).exists() {
        match fs::create_dir_all(home_envs.as_str()) {
            Ok(_) => {}
            Err(e) => {
                let info = format!("fs create_dir_all failed: {:?}", e);
                return Err(info);
            }
        }
    }

    let mut venvs = Vec::new();
    let pdir = home_envs.clone();
    if let Ok(list) = fs::read_dir(home_envs) {
        for ent in list.flatten() {
            let ent_name = ent.file_name();
            if let Ok(path) = ent_name.into_string() {
                let full_path = format!("{}/{}/bin/activate", pdir, path);
                if !Path::new(full_path.as_str()).exists() {
                    continue;
                }
                venvs.push(path);
            }
        }
    }

    Ok(venvs)
}

fn enter_env(sh: &Shell, path: &str) -> String {
    if in_env() {
        return "vox: already in env".to_string();
    }

    let home_envs = get_envs_home();
    let full_path = format!("{}/{}/bin/activate", home_envs, path);
    if !Path::new(full_path.as_str()).exists() {
        return format!("no such env: {}", full_path);
    }

    let path_env = format!("{}/{}", home_envs, path);
    env::set_var("VIRTUAL_ENV", &path_env);
    let path_new = String::from("${VIRTUAL_ENV}/bin:$PATH");
    let mut tokens: types::Tokens = Vec::new();
    tokens.push((String::new(), path_new));
    shell::expand_env(sh, &mut tokens);
    env::set_var("PATH", &tokens[0].1);
    String::new()
}

fn exit_env(sh: &Shell) -> String {
    if !in_env() {
        return String::from("vox: not in an env");
    }

    let env_path = match env::var("PATH") {
        Ok(x) => x,
        Err(_) => {
            return String::from("vox: cannot read PATH env");
        }
    };

    let mut all_tokens_path: Vec<String> = env::split_paths(&env_path)
        .map(|p: PathBuf| p.to_string_lossy().into_owned())
        .collect(); // only keep valid UTF-8 paths
    let mut path_virtual_env = String::from("${VIRTUAL_ENV}/bin");
    // shell::extend_env(sh, &mut path_virtual_env);
    let mut tokens: types::Tokens = Vec::new();
    tokens.push((String::new(), path_virtual_env));
    shell::expand_env(sh, &mut tokens);
    path_virtual_env = tokens[0].1.clone();
    all_tokens_path
        .iter()
        .position(|n| n.as_ref() == path_virtual_env)
        .map(|e| all_tokens_path.remove(e));
    let env_path_new = all_tokens_path.join(":");
    env::set_var("PATH", &env_path_new);
    env::set_var("VIRTUAL_ENV", "");

    String::new()
}

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = cmd.tokens.clone();
    let args = parsers::parser_line::tokens_to_args(&tokens);
    let len = args.len();
    let subcmd = if len > 1 { &args[1] } else { "" };

    if len == 1 || (len == 2 && subcmd == "ls") {
        match get_all_venvs() {
            Ok(venvs) => {
                let info = venvs.join("\n");
                print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
                return cr;
            }
            Err(reason) => {
                print_stderr_with_capture(&reason, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
    }

    if len == 3 && subcmd == "create" {
        let pybin = match env::var("VIRTUALENV_PYBIN") {
            Ok(x) => x,
            Err(_) => "python3".to_string(),
        };
        let dir_venv = get_envs_home();
        let venv_name = args[2].to_string();
        let line = format!("{} -m venv \"{}/{}\"", pybin, dir_venv, venv_name);
        print_stderr_with_capture(&line, &mut cr, cl, cmd, capture);
        let cr_list = execute::run_command_line(sh, &line, false, false);
        return cr_list[0].clone();
    }

    if len == 3 && subcmd == "enter" {
        let _err = enter_env(sh, args[2].as_str());
        if !_err.is_empty() {
            print_stderr_with_capture(&_err, &mut cr, cl, cmd, capture);
        }
        cr
    } else if len == 2 && subcmd == "exit" {
        let _err = exit_env(sh);
        if !_err.is_empty() {
            print_stderr_with_capture(&_err, &mut cr, cl, cmd, capture);
        }
        cr
    } else {
        let info = "cicada: vox: invalid option";
        print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
        cr
    }
}
