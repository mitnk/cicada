use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

use regex::Regex;

use linefeed::Reader;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion, Suffix};

pub struct MakeCompleter;

impl<Term: Terminal> Completer<Term> for MakeCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Reader<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_make(word))
    }
}

fn complete_make(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let _current_dir = env::current_dir().expect("cd: get current_dir error");
    let current_dir = _current_dir.to_str().expect("cd: to_str error");
    let make_file = current_dir.to_owned() + "/Makefile";
    if let Ok(f) = File::open(&make_file) {
        let file = BufReader::new(&f);
        let re = Regex::new(r"^ *([^ ]+):").expect("Regex build error");
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
