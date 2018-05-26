use std::fs::File;
use std::io::{BufRead, BufReader};

use regex::Regex;

use linefeed::Prompter;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion, Suffix};

use tools;

pub struct SshCompleter;

impl<Term: Terminal> Completer<Term> for SshCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
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
        let re;
        match Regex::new(r"^ *(?i)host +([^ ]+)") {
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
