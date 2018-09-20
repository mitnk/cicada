use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;

use builtins;
use parsers;
use shell;
use tools;

pub fn load_rcfile(sh: &mut shell::Shell) {
    // make "/usr/local/bin" as the first item in PATH
    if let Ok(env_path) = env::var("PATH") {
        if !env_path.contains("/usr/local/bin:") {
            let env_path_new = format!("/usr/local/bin:{}", env_path);
            env::set_var("PATH", &env_path_new);
        }
    }

    let rc_file = tools::get_rc_file();
    if !Path::new(rc_file.as_str()).exists() {
        return;
    }
    let mut file;
    match File::open(rc_file) {
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
        handle_line(sh, line);
    }
}

fn handle_line(sh: &mut shell::Shell, line: &str) {
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
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

fn handle_env(sh: &shell::Shell, tokens: &Vec<(String, String)>) {
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
