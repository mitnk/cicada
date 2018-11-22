use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use regex::Regex;

use linefeed::complete::{Completer, Completion, Suffix};
use linefeed::prompter::Prompter;
use linefeed::terminal::Terminal;

pub struct MakeCompleter;

impl<Term: Terminal> Completer<Term> for MakeCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_make(word))
    }
}

fn handle_file(ci: &mut Vec<Completion>, path: &str, file_path: &str, current_dir: &str) {
    if let Ok(f) = File::open(file_path) {
        let file = BufReader::new(&f);
        let re_cmd;
        match Regex::new(r"^ *([^ ]+):") {
            Ok(x) => re_cmd = x,
            Err(e) => {
                println_stderr!("cicada: regex build error: {:?}", e);
                return;
            }
        }
        let re_include;
        match Regex::new(r"^ *include  *([^ ]+) *$") {
            Ok(x) => re_include = x,
            Err(e) => {
                println_stderr!("cicada: regex build error: {:?}", e);
                return;
            }
        }

        for (_, line) in file.lines().enumerate() {
            if let Ok(line) = line {
                if re_cmd.is_match(&line) {
                    for cap in re_cmd.captures_iter(&line) {
                        if !cap[1].starts_with(path) {
                            continue;
                        }
                        ci.push(Completion {
                            completion: cap[1].to_string(),
                            display: None,
                            suffix: Suffix::Default,
                        });
                    }
                }
                if re_include.is_match(&line) {
                    for cap in re_include.captures_iter(&line) {
                        let _file = &cap[1];
                        if _file.contains('/') {
                            handle_file(ci, path, &_file, current_dir);
                        } else {
                            let make_file = current_dir.to_owned() + "/" + _file;
                            handle_file(ci, path, &make_file, current_dir);
                        }
                    }
                }
            }
        }
    }
}

fn complete_make(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let _current_dir;
    match env::current_dir() {
        Ok(x) => _current_dir = x,
        Err(e) => {
            println!("cicada: get current_dir error: {:?}", e);
            return res;
        }
    }
    let current_dir;
    match _current_dir.to_str() {
        Some(x) => current_dir = x,
        None => {
            println!("cicada: to_str error");
            return res;
        }
    }
    let make_file = current_dir.to_owned() + "/Makefile";
    handle_file(&mut res, path, &make_file, &current_dir);
    res
}
