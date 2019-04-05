use std::io::Write;

use regex::Regex;

use crate::shell;
use crate::tools;
use crate::types::Tokens;

pub fn run(sh: &mut shell::Shell, tokens: &Tokens) -> i32 {
    if tokens.len() == 1 {
        return show_alias_list(sh, "");
    }
    if tokens.len() > 2 {
        println_stderr!("alias syntax error");
        println_stderr!("alias usage example: alias foo='echo foo'");
        return 1;
    }

    let input = &tokens[1].1;

    let re_single_read;
    match Regex::new(r"^[a-zA-Z0-9_\.-]+$") {
        Ok(x) => re_single_read = x,
        Err(e) => {
            println!("cicada: Regex error: {:?}", e);
            return 1;
        }
    }
    if re_single_read.is_match(input) {
        return show_alias_list(sh, input);
    }

    let re_to_add;
    match Regex::new(r"^([a-zA-Z0-9_\.-]+)=(.*)$") {
        Ok(x) => re_to_add = x,
        Err(e) => {
            println!("cicada: Regex error: {:?}", e);
            return 1;
        }
    }

    for cap in re_to_add.captures_iter(input) {
        let name = tools::unquote(&cap[1]);
        let value = tools::unquote(&cap[2]);
        sh.add_alias(name.as_str(), value.as_str());
    }
    0
}

fn show_alias_list(sh: &shell::Shell, name_to_find: &str) -> i32 {
    let mut single_and_not_found = true;
    for (name, value) in sh.get_alias_list() {
        if name_to_find.is_empty() {
            single_and_not_found = false;
            println!("alias {}='{}'", name, value);
        } else if name_to_find == name {
            println!("alias {}='{}'", name, value);
            single_and_not_found = false;
        }
    }
    if single_and_not_found {
        println_stderr!("cicada: alias: {}: not found", name_to_find);
        return 1;
    }
    0
}
