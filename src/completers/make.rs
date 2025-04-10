use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

use regex::Regex;

use lineread::complete::{Completer, Completion, Suffix};
use lineread::prompter::Prompter;
use lineread::terminal::Terminal;

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
        let re_cmd = match Regex::new(r"^ *([^ ]+):") {
            Ok(x) => x,
            Err(e) => {
                println_stderr!("cicada: regex build error: {:?}", e);
                return;
            }
        };

        let re_include = match Regex::new(r"^ *include  *([^ ]+) *$") {
            Ok(x) => x,
            Err(e) => {
                println_stderr!("cicada: regex build error: {:?}", e);
                return;
            }
        };

        for line in file.lines().map_while(Result::ok) {
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
                        handle_file(ci, path, _file, current_dir);
                    } else {
                        let make_file = current_dir.to_owned() + "/" + _file;
                        handle_file(ci, path, &make_file, current_dir);
                    }
                }
            }
        }
    }
}

fn complete_make(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let current_dir = match env::current_dir() {
        Ok(dir) => match dir.to_str() {
            Some(s) => s.to_string(),
            None => {
                println!("cicada: to_str error");
                return res;
            }
        },
        Err(e) => {
            println!("cicada: get current_dir error: {:?}", e);
            return res;
        }
    };

    let make_file = format!("{}/Makefile", current_dir);
    handle_file(&mut res, path, &make_file, &current_dir);
    res
}
