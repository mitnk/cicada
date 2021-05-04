use std::io::{self, Write};

use crate::shell;
use crate::libs::re::re_contains;
use crate::types::CommandLine;
use crate::tools;

fn _find_invalid_identifier(name_list: &Vec<String>) -> Option<String> {
    for id_ in name_list {
        if !re_contains(id_, r"^[a-zA-Z_][a-zA-Z0-9_]*$") {
            return Some(id_.to_string());
        }
    }
    None
}

pub fn run(sh: &mut shell::Shell, cl: &CommandLine) -> i32 {
    let cmd = &cl.commands[0];
    let tokens = cmd.tokens.clone();

    let name_list: Vec<String>;
    if tokens.len() <= 1 {
        name_list = vec!["REPLY".to_string()];
    } else {
        name_list = tokens[1..].iter().map(|x| x.1.clone()).collect();
        if let Some(id_) = _find_invalid_identifier(&name_list) {
            println_stderr!("cicada: read: `{}': not a valid identifier", id_);
            return 1;
        }
    }

    let mut buffer = String::new();

    if cmd.has_here_string() {
        if let Some(redirect_from) = &cmd.redirect_from {
            buffer.push_str(&redirect_from.1);
            buffer.push('\n');
        }
    } else {
        match io::stdin().read_line(&mut buffer) {
            Ok(_) => {}
            Err(e) => {
                println_stderr!("cicada: read: error in reading stdin: {:?}", e);
                return 1;
            }
        }
    }

    let envs = cl.envs.clone();
    let value_list = tools::split_into_fields(sh, buffer.trim(), &envs);

    let idx_2rd_last = name_list.len() - 1;
    for i in 0..idx_2rd_last {
        let name = name_list.get(i);
        if name.is_none() {
            println_stderr!("cicada: read: name index error");
            return 1;
        }
        let name = name.unwrap();

        let value = value_list.get(i).unwrap_or(&String::new()).clone();
        sh.set_env(name, &value);
    }

    let name_last = &name_list[idx_2rd_last];
    let value_left: String = if value_list.len() > idx_2rd_last {
        value_list[idx_2rd_last..].join(" ")
    } else {
        String::new()
    };
    sh.set_env(name_last, &value_left);
    0
}
