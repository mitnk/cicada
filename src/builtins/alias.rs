use std::io::Write;

use regex::Regex;

use crate::shell;
use crate::tools;
use crate::types::Tokens;

pub fn run(sh: &mut shell::Shell, tokens: &Tokens) -> i32 {
    if tokens.len() == 1 {
        return show_alias_list(sh);
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
        return show_single_alias(sh, input);
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
        // due to limitation of `parses::parser_line`,
        // `alias foo-bar='foo bar'` will become 'foo-bar=foo bar'
        // while `alias foo_bar='foo bar'` keeps foo_bar='foo bar'
        let value = if cap[2].starts_with('"') || cap[2].starts_with('\'') {
            tools::unquote(&cap[2])
        } else {
            cap[2].to_string()
        };
        sh.add_alias(name.as_str(), value.as_str());
    }
    0
}

fn show_alias_list(sh: &shell::Shell) -> i32 {
    for (name, value) in sh.get_alias_list() {
        println!("alias {}='{}'", name, value);
    }
    0
}

fn show_single_alias(sh: &shell::Shell, name_to_find: &str) -> i32 {
    let mut found = false;
    for (name, value) in sh.get_alias_list() {
        if name_to_find == name {
            println!("alias {}='{}'", name, value);
            found = true;
        }
    }
    if !found {
        println_stderr!("cicada: alias: {}: not found", name_to_find);
        return 1;
    }
    0
}
