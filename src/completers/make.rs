use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

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

fn complete_make(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let _current_dir;
    match env::current_dir() {
        Ok(x) => _current_dir = x,
        Err(e) => {
            println!("cd: get current_dir error: {:?}", e);
            return res;
        }
    }
    let current_dir;
    match _current_dir.to_str() {
        Some(x) => current_dir = x,
        None => {
            println!("cd: to_str error");
            return res;
        }
    }
    let make_file = current_dir.to_owned() + "/Makefile";
    if let Ok(f) = File::open(&make_file) {
        let file = BufReader::new(&f);
        let re;
        match Regex::new(r"^ *([^ ]+):") {
            Ok(x) => re = x,
            Err(e) => {
                println!("Regex build error: {:?}", e);
                return res;
            }
        }
        for (_, line) in file.lines().enumerate() {
            if let Ok(line) = line {
                if !re.is_match(&line) {
                    continue;
                }
                for cap in re.captures_iter(&line) {
                    if !cap[1].starts_with(path) {
                        continue;
                    }
                    res.push(Completion {
                        completion: cap[1].to_string(),
                        display: None,
                        suffix: Suffix::Default,
                    });
                }
            }
        }
    }
    res
}
