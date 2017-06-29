use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;

use builtins;
use tools;
use shell;

pub fn load_rcfile(sh: &mut shell::Shell) {
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
    if tools::is_env(line) {
        handle_env(line);
        return;
    }
    if tools::is_alias(line) {
        handle_alias(sh, line);
        return;
    }
}

fn handle_env(line: &str) {
    builtins::export::run(line);
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
