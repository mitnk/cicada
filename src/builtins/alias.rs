use regex::Regex;

use crate::shell;
use crate::tools;
use crate::types::{Command, CommandLine};
use crate::builtins::utils::{print_stderr, print_stdout};

pub fn run(sh: &mut shell::Shell, cl: &CommandLine, idx_cmd: usize) -> i32 {
    let cmd = &cl.commands[idx_cmd];
    let tokens = cmd.tokens.clone();

    if tokens.len() == 1 {
        return show_alias_list(sh, cmd, cl);
    }
    if tokens.len() > 2 {
        let info = "alias syntax error: usage: alias foo='echo foo'";
        print_stderr(info, cmd, cl);
        return 1;
    }

    let input = &tokens[1].1;
    let re_single_read = Regex::new(r"^[a-zA-Z0-9_\.-]+$").unwrap();
    if re_single_read.is_match(input) {
        return show_single_alias(sh, input, cmd, cl);
    }

    let re_to_add = Regex::new(r"^([a-zA-Z0-9_\.-]+)=(.*)$").unwrap();
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

fn show_alias_list(sh: &shell::Shell, cmd: &Command, cl: &CommandLine) -> i32 {
    let mut lines = Vec::new();
    for (name, value) in sh.get_alias_list() {
        let line = format!("alias {}='{}'", name, value);
        lines.push(line);
    }
    let buffer = lines.join("\n");
    print_stdout(&buffer, cmd, cl);
    0
}

fn show_single_alias(sh: &shell::Shell, name_to_find: &str, cmd: &Command,
                     cl: &CommandLine) -> i32 {
    if let Some(content) = sh.get_alias_content(name_to_find) {
        let info = format!("alias {}='{}'", name_to_find, content);
        print_stdout(&info, cmd, cl);
        0
    } else {
        let info = format!("cicada: alias: {}: not found", name_to_find);
        print_stderr(&info, cmd, cl);
        1
    }
}
