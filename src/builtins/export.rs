use regex::Regex;
use std::env;

use crate::libs;
use crate::parsers;
use crate::tools;

use crate::builtins::utils::print_stderr_with_capture;
use crate::shell::Shell;
use crate::types::{Command, CommandLine, CommandResult};

pub fn run(_sh: &Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = cmd.tokens.clone();

    let re_name_ptn = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_]*)=(.*)$").unwrap();
    for (_, text) in tokens.iter() {
        if text == "export" {
            continue;
        }

        if !tools::is_env(text) {
            let mut info = String::new();
            info.push_str("export: invalid command\n");
            info.push_str("usage: export XXX=YYY");
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
        }

        if !re_name_ptn.is_match(text) {
            let mut info = String::new();
            info.push_str("export: invalid command\n");
            info.push_str("usage: export XXX=YYY ZZ=123");
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
        }

        for cap in re_name_ptn.captures_iter(text) {
            let name = cap[1].to_string();
            let token = parsers::parser_line::unquote(&cap[2]);
            let value = libs::path::expand_home(&token);
            env::set_var(name, &value);
        }
    }
    cr
}
