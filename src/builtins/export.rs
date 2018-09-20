use regex::Regex;
use std::env;
use std::io::Write;

use parsers;
use shell;
use tools;

pub fn run(_sh: &shell::Shell, tokens: &Vec<(String, String)>) -> i32 {
    let mut i = 0;
    for (_, text) in tokens.iter() {
        if i == 0 {
            i += 1;
            continue
        }
        if !tools::is_env(text) {
            println!("export: invalid command");
            println!("usage: export XXX=YYY");
            return 1;
        }

        if let Ok(re) = Regex::new(r"^([a-zA-Z0-9_]+)=(.*)$") {
            if !re.is_match(text) {
                println!("export: invalid command");
                println!("usage: export XXX=YYY ZZ=123");
                return 1;
            }

            for cap in re.captures_iter(text) {
                let name = cap[1].to_string();
                let value = parsers::parser_line::unquote(&cap[2]);
                env::set_var(name, value);
            }
        } else {
            println_stderr!("cicada: re new error");
            return 2;
        }
        i += 1;
    }
    0
}
