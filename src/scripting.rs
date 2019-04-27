use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use regex::Regex;

use crate::execute;
use crate::libs;
use crate::parsers;
use crate::shell;
use crate::types;

pub fn run_script(sh: &mut shell::Shell, args: &Vec<String>) -> i32 {
    let mut status = 0;

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
    for line in text.lines() {
        if line.trim().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let line_new = expand_args(line, &args[1..]);
        status = execute::run_procs(sh, &line_new, true);
        if status != 0 {
            return status;
        }
    }

    status
}

fn expand_args(line: &str, args: &[String]) -> String {
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
    expand_args_in_tokens(&mut tokens, args);
    return parsers::parser_line::tokens_to_line(&tokens);
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
