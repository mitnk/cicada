use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::Regex;

use tools;

pub fn load_rcfile() {
    let rc_file = tools::get_rc_file();
    if !Path::new(rc_file.as_str()).exists() {
        return;
    }
    let mut file = File::open(rc_file).expect("opening file");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("reading file");
    for line in text.lines() {
        handle_line(line);
    }
}

fn is_env(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^ *export *[a-zA-Z0-9_\.-]+=.*$") {
        re = x;
    } else {
        return false;
    }
    return re.is_match(line);
}

fn handle_line(line: &str) {
    if is_env(line) {
        handle_env(line);
        return;
    }
}

fn handle_env(line: &str) {
    let re = Regex::new(r"^ *export *([a-zA-Z0-9_\.-]+)=(.*)$").unwrap();
    for cap in re.captures_iter(line) {
        let value = tools::unquote(&cap[2]);
        env::set_var(&cap[1], &value);
    }
}
