use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use pest::iterators::Pair;
use regex::{Regex, RegexBuilder};

use crate::execute;
use crate::libs;
use crate::parsers;
use crate::shell;
use crate::types;
use crate::types::CommandResult;

pub fn run_script(sh: &mut shell::Shell, args: &Vec<String>) -> i32 {
    let src_file = &args[1];
    let full_src_file: String;
    if src_file.contains('/') {
        full_src_file = src_file.clone();
    } else {
        let full_path = libs::path::find_file_in_path(src_file, false);
        if full_path.is_empty() {
            // not in PATH and not in current work directory
            if !Path::new(src_file).exists() {
                println_stderr!("cicada: {}: no such file", src_file);
                return 1;
            }
            full_src_file = format!("./{}", src_file);
        } else {
            full_src_file = full_path.clone();
        }
    }

    if !Path::new(&full_src_file).exists() {
        println_stderr!("cicada: {}: no such file", src_file);
        return 1;
    }
    if Path::new(&full_src_file).is_dir() {
        println_stderr!("cicada: {}: is a directory", src_file);
        return 1;
    }

    let mut file;
    match File::open(&full_src_file) {
        Ok(x) => file = x,
        Err(e) => {
            println_stderr!("cicada: open script file err: {:?}", e);
            return 1;
        }
    }
    let mut text = String::new();
    match file.read_to_string(&mut text) {
        Ok(_) => {}
        Err(e) => {
            println_stderr!("cicada: read_to_string error: {:?}", e);
            return 1;
        }
    }

    if text.contains("\\\n") {
        let re = RegexBuilder::new(r#"([ \t]*\\\n[ \t]+)|([ \t]+\\\n[ \t]*)"#)
            .multi_line(true).build().unwrap();
        text = re.replace_all(&text, " ").to_string();

        let re = RegexBuilder::new(r#"\\\n"#).multi_line(true).build().unwrap();
        text = re.replace_all(&text, "").to_string();
    }

    let re_func_head = Regex::new(r"^function ([a-zA-Z_-][a-zA-Z0-9_-]*) *(?:\(\))? *\{$").unwrap();
    let re_func_tail = Regex::new(r"^\}$").unwrap();
    let mut text_new = String::new();
    let mut enter_func = false;
    let mut func_name = String::new();
    let mut func_body = String::new();
    for line in text.clone().lines() {
        if re_func_head.is_match(line.trim()) {
            enter_func = true;
            let cap = re_func_head.captures(line.trim()).unwrap();
            func_name = cap[1].to_string();
            func_body = String::new();
            continue;
        }
        if re_func_tail.is_match(line.trim()) {
            sh.set_func(&func_name, &func_body);
            enter_func = false;
            continue;
        }
        if enter_func {
            func_body.push_str(line);
            func_body.push('\n');
        } else {
            text_new.push_str(line);
            text_new.push('\n');
        }
    }

    let mut status = 0;
    let cr_list = run_lines(sh, &text_new, args, false);
    if let Some(last) = cr_list.last() {
        status = last.status;
    }
    status
}

pub fn run_lines(sh: &mut shell::Shell,
                 lines: &str,
                 args: &Vec<String>,
                 capture: bool) -> Vec<CommandResult> {
    let mut cr_list = Vec::new();
    match parsers::locust::parse_lines(&lines) {
        Ok(pairs_exp) => {
            for pair in pairs_exp {
                let (mut _cr_list, _cont, _brk) = run_exp(sh, pair, args, false, capture);
                cr_list.append(&mut _cr_list);
            }
        }
        Err(e) => {
            println_stderr!("syntax error: {:?}", e);
            return cr_list;
        }
    }
    cr_list
}

fn expand_args(line: &str, args: &[String]) -> String {
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
    expand_args_in_tokens(&mut tokens, args);
    return parsers::parser_line::tokens_to_line(&tokens);
}

fn expand_line_to_toknes(line: &str,
                         args: &[String],
                         sh: &mut shell::Shell) -> types::Tokens {
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
    expand_args_in_tokens(&mut tokens, args);
    shell::do_expansion(sh, &mut tokens);
    tokens
}

fn is_args_in_token(token: &str) -> bool {
    libs::re::re_contains(token, r"\$\{?[0-9@]+\}?")
}

fn expand_args_for_single_token(token: &str, args: &[String]) -> String {
    let re;
    if let Ok(x) = Regex::new(r"^(.*?)\$\{?([0-9]+|@)\}?(.*)$") {
        re = x;
    } else {
        println_stderr!("cicada: re new error");
        return String::new();
    }
    if !re.is_match(token) {
        return token.to_string();
    }

    let mut result = String::new();
    let mut _token = token.to_string();
    let mut _head = String::new();
    let mut _output = String::new();
    let mut _tail = String::new();
    loop {
        if !re.is_match(&_token) {
            if !_token.is_empty() {
                result.push_str(&_token);
            }
            break;
        }
        for cap in re.captures_iter(&_token) {
            _head = cap[1].to_string();
            _tail = cap[3].to_string();
            let _key = cap[2].to_string();
            if _key == "@" {
                result.push_str(format!("{}{}", _head, args[1..].join(" ")).as_str());
            } else if let Ok(arg_idx) = _key.parse::<usize>() {
                if arg_idx < args.len() {
                    result.push_str(format!("{}{}", _head, args[arg_idx]).as_str());
                } else {
                    result.push_str(&_head);
                }
            } else {
                result.push_str(&_head);
            }
        }

        if _tail.is_empty() {
            break;
        }
        _token = _tail.clone();
    }
    result
}

fn expand_args_in_tokens(tokens: &mut types::Tokens, args: &[String]) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();

    for (sep, token) in tokens.iter() {
        if sep == "`" || sep == "'" || !is_args_in_token(token) {
            idx += 1;
            continue;
        }

        let _token = expand_args_for_single_token(token, args);
        buff.push((idx, _token));
        idx += 1;
    }

    for (i, text) in buff.iter().rev() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn run_exp_test_br(sh: &mut shell::Shell,
                   pair_br: Pair<parsers::locust::Rule>,
                   args: &Vec<String>,
                   in_loop: bool,
                   capture: bool) -> (Vec<CommandResult>, bool, bool, bool) {
    let mut cr_list = Vec::new();
    let pairs = pair_br.into_inner();
    let mut test_pass = false;
    for pair in pairs {
        let rule = pair.as_rule();
        if rule == parsers::locust::Rule::IF_HEAD ||
                rule == parsers::locust::Rule::IF_ELSEIF_HEAD ||
                rule == parsers::locust::Rule::WHILE_HEAD {
            let pairs_test: Vec<Pair<parsers::locust::Rule>> =
                pair.into_inner().collect();
            let pair_test = &pairs_test[0];
            let line = pair_test.as_str().trim();
            let line_new = expand_args(line, &args[1..]);
            let mut _cr_list = execute::run_procs(sh, &line_new, true, capture);
            if let Some(last) = _cr_list.last() {
                if last.status == 0 {
                    test_pass = true;
                }
            }
            cr_list.append(&mut _cr_list);
            continue;
        }

        if rule == parsers::locust::Rule::KW_ELSE {
            test_pass = true;
            continue;
        }

        if rule == parsers::locust::Rule::EXP_BODY {
            if !test_pass {
                return (cr_list, false, false, false);
            }
            let (mut _cr_list, _cont, _brk) = run_exp(sh, pair, args, in_loop, capture);
            cr_list.append(&mut _cr_list);
            // branch executed successfully
            return (cr_list, true, _cont, _brk);
        }

        unreachable!();
    }
    (cr_list, test_pass, false, false)
}

fn run_exp_if(sh: &mut shell::Shell,
              pair_if: Pair<parsers::locust::Rule>,
              args: &Vec<String>,
              in_loop: bool,
              capture: bool) -> (Vec<CommandResult>, bool, bool) {
    let mut cr_list = Vec::new();
    let pairs = pair_if.into_inner();
    let mut met_continue = false;
    let mut met_break = false;
    for pair in pairs {
        let (mut _cr_list, passed, _cont, _brk) = run_exp_test_br(sh, pair, args, in_loop, capture);
        met_continue = _cont;
        met_break = _brk;
        cr_list.append(&mut _cr_list);
        // break at first successful branch
        if passed {
            break;
        }
    }
    (cr_list, met_continue, met_break)
}

fn get_for_result_from_init(sh: &mut shell::Shell,
                            pair_init: Pair<parsers::locust::Rule>,
                            args: &Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let pairs = pair_init.into_inner();
    for pair in pairs {
        let rule = pair.as_rule();
        if rule == parsers::locust::Rule::TEST {
            let line = pair.as_str().trim();
            let tokens = expand_line_to_toknes(line, &args[1..], sh);
            for (sep, token) in tokens {
                if sep.is_empty() {
                    for x in token.split_whitespace() {
                        result.push(x.to_string());
                    }
                } else {
                    result.push(token.clone());
                }
            }
        }
    }
    result
}

fn get_for_result_list(sh: &mut shell::Shell,
                       pair_head: Pair<parsers::locust::Rule>,
                       args: &Vec<String>) -> Vec<String> {
    let pairs = pair_head.into_inner();
    for pair in pairs {
        let rule = pair.as_rule();
        if rule == parsers::locust::Rule::FOR_INIT {
            return get_for_result_from_init(sh, pair, args);
        }
    }
    return Vec::new();
}

fn get_for_var_name(pair_head: Pair<parsers::locust::Rule>) -> String {
    let pairs = pair_head.into_inner();
    for pair in pairs {
        let rule = pair.as_rule();
        if rule == parsers::locust::Rule::FOR_INIT {
            let pairs_init = pair.into_inner();
            for pair_init in pairs_init {
                let rule_init = pair_init.as_rule();
                if rule_init == parsers::locust::Rule::FOR_VAR {
                    let line = pair_init.as_str().trim();
                    return line.to_string();
                }
            }
        }
    }
    String::new()
}

fn run_exp_for(sh: &mut shell::Shell,
               pair_for: Pair<parsers::locust::Rule>,
               args: &Vec<String>,
               capture: bool) -> Vec<CommandResult> {
    let mut cr_list = Vec::new();
    let pairs = pair_for.into_inner();
    let mut result_list: Vec<String> = Vec::new();
    let mut var_name: String = String::new();
    for pair in pairs {
        let rule = pair.as_rule();
        if rule == parsers::locust::Rule::FOR_HEAD {
            var_name = get_for_var_name(pair.clone());
            result_list = get_for_result_list(sh, pair.clone(), args);
            continue;
        }
        if rule == parsers::locust::Rule::EXP_BODY {
            for value in &result_list {
                sh.set_env(&var_name, &value);
                let (mut _cr_list, _cont, _brk) = run_exp(
                    sh, pair.clone(), args, true, capture);
                cr_list.append(&mut _cr_list);
                if _brk {
                    break;
                }
            }
        }
    }
    cr_list
}

fn run_exp_while(sh: &mut shell::Shell,
                 pair_while: Pair<parsers::locust::Rule>,
                 args: &Vec<String>,
                 capture: bool) -> Vec<CommandResult> {
    let mut cr_list = Vec::new();
    loop {
        let (mut _cr_list, passed, _cont, _brk) = run_exp_test_br(sh, pair_while.clone(), args, true, capture);
        cr_list.append(&mut _cr_list);
        if !passed || _brk {
            break;
        }
    }
    cr_list
}

fn run_exp(sh: &mut shell::Shell,
           pair_in: Pair<parsers::locust::Rule>,
           args: &Vec<String>,
           in_loop: bool,
           capture: bool) -> (Vec<CommandResult>, bool, bool) {
    let mut cr_list = Vec::new();
    let pairs = pair_in.into_inner();
    for pair in pairs {
        let line = pair.as_str().trim();
        if line.is_empty() {
            continue;
        }

        let rule = pair.as_rule();
        if rule == parsers::locust::Rule::CMD {
            if line == "continue" {
                if in_loop {
                    return (cr_list, true, false);
                } else {
                    println_stderr!("cicada: continue: only meaningful in loops");
                    continue;
                }
            }
            if line == "break" {
                if in_loop {
                    return (cr_list, false, true);
                } else {
                    println_stderr!("cicada: break: only meaningful in loops");
                    continue;
                }
            }

            let line_new = expand_args(line, &args[1..]);
            let mut _cr_list = execute::run_procs(sh, &line_new, true, capture);
            cr_list.append(&mut _cr_list);
            if let Some(last) = cr_list.last() {
                let status = last.status;
                if status != 0 {
                    return (cr_list, false, false);
                }
            }
        } else if rule == parsers::locust::Rule::EXP_IF {
            let (mut _cr_list, _cont, _brk) = run_exp_if(sh, pair, args, in_loop, capture);
            cr_list.append(&mut _cr_list);
            if _cont {
                return (cr_list, true, false);
            }
            if _brk {
                return (cr_list, false, true);
            }
        } else if rule == parsers::locust::Rule::EXP_FOR {
            let mut _cr_list = run_exp_for(sh, pair, args, capture);
            cr_list.append(&mut _cr_list);
        } else if rule == parsers::locust::Rule::EXP_WHILE {
            let mut _cr_list = run_exp_while(sh, pair, args, capture);
            cr_list.append(&mut _cr_list);
        }
    }
    (cr_list, false, false)
}

#[cfg(test)]
mod tests {
    use super::expand_args;
    use super::libs;

    #[test]
    fn test_expand_args() {
        let args = vec!["./demo.sh".to_string(), "foo".to_string(), "bar".to_string(), "baz".to_string()];

        let line = "echo $@";
        let line_new = expand_args(line, &args);
        assert_eq!(line_new, "echo foo bar baz");

        let line = "echo \"$@\"";
        let line_new = expand_args(line, &args);
        assert_eq!(line_new, "echo \"foo bar baz\"");

        let line = "echo $1";
        let line_new = expand_args(line, &args);
        assert_eq!(line_new, "echo foo");

        let line = "echo $2 $1";
        let line_new = expand_args(line, &args);
        assert_eq!(line_new, "echo bar foo");

        let line = "echo $3 $1 $2";
        let line_new = expand_args(line, &args);
        assert_eq!(line_new, "echo baz foo bar");

        let line = "echo $3 $1 $2 $4 $5";
        let ptn_expected = r"^echo baz foo bar *$";
        let line_new = expand_args(line, &args);
        if !libs::re::re_contains(&line_new, ptn_expected) {
            println!("expect RE: {:?}", ptn_expected);
            println!("real: {:?}", line_new);
            assert!(false);
        }

        let line = "echo \"==$3--$$==$1--$2==$4--$5==$$--$2==\"";
        let line_new = expand_args(line, &args);
        let ptn_expected = r"^echo .==baz--\$\$==foo--bar==--==\$\$--bar==.$";
        if !libs::re::re_contains(&line_new, ptn_expected) {
            println!("expect RE: {:?}", ptn_expected);
            println!("real: {:?}", line_new);
            assert!(false);
        }
    }
}
