use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;
use shellexpand;

use tools;
use shell;

pub fn load_rcfile(sh: &mut shell::Shell) {
    let rc_file = tools::get_rc_file();
    if !Path::new(rc_file.as_str()).exists() {
        return;
    }
    let mut file = File::open(rc_file).expect("opening file");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("reading file");
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
    let re = Regex::new(r"^ *export +([a-zA-Z0-9_\.-]+)=(.*)$")
        .expect("cicada: Regex error");
    for cap in re.captures_iter(line) {
        let value = tools::unquote(&cap[2]);
        match shellexpand::env(value.as_str()) {
            Ok(x) => {
                let mut value = x.to_string();
                if tools::needs_extend_home(value.as_str()) {
                    tools::extend_home(&mut value);
                }
                let name = &cap[1];
                env::set_var(name, &value);
            }
            Err(e) => {
                println!("shellexpand err: {:?}", e);
            }
        }
    }
}

fn handle_alias(sh: &mut shell::Shell, line: &str) {
    let re = Regex::new(r"^ *alias +([a-zA-Z0-9_\.-]+)=(.*)$")
        .expect("cicada: Regex error");
    for cap in re.captures_iter(line) {
        let name = tools::unquote(&cap[1]);
        let value = tools::unquote(&cap[2]);
        sh.add_alias(name.as_str(), value.as_str());
    }
}
