use std::env;
use std::io::Write;
use regex::Regex;

use shell;
use tools;
use parsers;

pub fn run(sh: &shell::Shell, line: &str) -> i32 {
    if !tools::is_env(line) {
        println!("export: expected syntax like XXX=YYY");
        return 1;
    }

    let _line;
    if let Ok(re) = Regex::new(r"^ *export +") {
        if !re.is_match(line) {
            println_stderr!("export: invalid command?");
            return 1;
        }
        _line = re.replace_all(line, "");
    } else {
        println_stderr!("cicada: re new error");
        return 2;
    }

    let args = parsers::parser_line::cmd_to_tokens(&_line);
    for (sep, token) in args {
        if sep == "`" {
            continue;
        }
        if let Ok(re) = Regex::new(r" *([a-zA-Z0-9_]+)=(.*) *") {
            if !re.is_match(&token) {
                continue;
            }
            for cap in re.captures_iter(&token) {
                let mut _value = tools::unquote(&cap[2]);
                if tools::needs_extend_home(&_value) {
                    tools::extend_home(&mut _value);
                }
                let value = shell::extend_env_blindly(sh, &_value);
                env::set_var(&cap[1], &value);
            }
        } else {
            println_stderr!("cicada: re new error");
            return 2;
        }
    }
    0
}
