use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use builtins;
use parsers;
use shell;
use tools;
use types;

fn load_file(sh: &mut shell::Shell, file_path: &str, count: i32) {
    if count > 99 {
        // to prevent dead include loop
        println_stderr!("loaded too many rc files");
        return;
    }

    let rc_file;
    if file_path.starts_with('/') {
        rc_file = file_path.to_string();
    } else {
        let home = tools::get_user_home();
        rc_file = format!("{}/{}", home, file_path);
    }
    if !Path::new(&rc_file).exists() {
        return;
    }

    let mut file;
    match File::open(&rc_file) {
        Ok(x) => file = x,
        Err(e) => {
            println!("cicada: open rcfile err: {:?}", e);
            return;
        }
    }
    let mut text = String::new();
    match file.read_to_string(&mut text) {
        Ok(_) => {}
        Err(e) => {
            println!("cicada: read_to_string error: {:?}", e);
            return;
        }
    }
    for line in text.lines() {
        handle_line(sh, line, count);
    }
}

pub fn load_rc_files(sh: &mut shell::Shell) {
    // make "/usr/local/bin" as the first item in PATH
    if let Ok(env_path) = env::var("PATH") {
        if !env_path.contains("/usr/local/bin:") {
            let env_path_new = format!("/usr/local/bin:{}", env_path);
            env::set_var("PATH", &env_path_new);
        }
    }

    let rc_file = tools::get_rc_file();
    load_file(sh, &rc_file, 1);
}

fn handle_line(sh: &mut shell::Shell, line: &str, count: i32) {
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
    if tokens.len() == 2 && tokens[0].1 == "include" {
        load_file(sh, &tokens[1].1, count + 1);
        return;
    }

    shell::do_expansion(sh, &mut tokens);
    if tools::is_export_env(line) {
        handle_env(sh, &tokens);
        return;
    }
    if tools::is_alias(line) {
        handle_alias(sh, line);
        return;
    }
}

fn handle_env(sh: &shell::Shell, tokens: &types::Tokens) {
    builtins::export::run(sh, tokens);
}

fn handle_alias(sh: &mut shell::Shell, line: &str) {
    let re;
    match Regex::new(r"^ *alias +([a-zA-Z0-9_\.-]+)=(.*)$") {
        Ok(x) => re = x,
        Err(e) => {
            println!("cicada: Regex error: {:?}", e);
            return;
        }
    }
    for cap in re.captures_iter(line) {
        let name = tools::unquote(&cap[1]);
        let value = tools::unquote(&cap[2]);
        sh.add_alias(name.as_str(), value.as_str());
    }
}
