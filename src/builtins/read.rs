use std::io;

use crate::builtins::utils::print_stderr_with_capture;
use crate::libs::re::re_contains;
use crate::shell::Shell;
use crate::tools;
use crate::types::{Command, CommandLine, CommandResult};

fn _find_invalid_identifier(name_list: &Vec<String>) -> Option<String> {
    for id_ in name_list {
        if !re_contains(id_, r"^[a-zA-Z_][a-zA-Z0-9_]*$") {
            return Some(id_.to_string());
        }
    }
    None
}

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = cmd.tokens.clone();

    let name_list: Vec<String>;
    if tokens.len() <= 1 {
        name_list = vec!["REPLY".to_string()];
    } else {
        name_list = tokens[1..].iter().map(|x| x.1.clone()).collect();
        if let Some(id_) = _find_invalid_identifier(&name_list) {
            let info = format!("cicada: read: `{}': not a valid identifier", id_);
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
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
                let info = format!("cicada: read: error in reading stdin: {:?}", e);
                print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                return cr;
            }
        }
    }

    let envs = cl.envs.clone();
    let value_list = tools::split_into_fields(sh, buffer.trim(), &envs);

    let idx_2rd_last = name_list.len() - 1;
    for i in 0..idx_2rd_last {
        let name = name_list.get(i);
        if name.is_none() {
            let info = "cicada: read: name index error";
            print_stderr_with_capture(info, &mut cr, cl, cmd, capture);
            return cr;
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
    cr
}
