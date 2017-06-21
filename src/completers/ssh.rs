use std::fs::File;
use std::io::{BufRead, BufReader};

use regex::Regex;

use linefeed::Reader;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion, Suffix};

use tools;
use completers;

pub struct SshCompleter;

impl<Term: Terminal> Completer<Term> for SshCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Reader<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_ssh(word))
    }
}

fn complete_ssh(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let home = tools::get_user_home();
    let ssh_config = home + "/.ssh/config";
    if let Ok(f) = File::open(&ssh_config) {
        let file = BufReader::new(&f);
        let re = Regex::new(r"^ *(?i)host +([^ ]+)").expect("Regex build error");
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
    if res.is_empty() {
        return completers::path::complete_path(path, false);
    }
    res
}
