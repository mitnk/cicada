use std::env;

use linefeed::complete::{Completer, Completion, Suffix};
use linefeed::prompter::Prompter;
use linefeed::terminal::Terminal;

pub struct EnvCompleter;

impl<Term: Terminal> Completer<Term> for EnvCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_env(word))
    }
}

fn complete_env(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    if path.trim().is_empty() {
        return res;
    }
    let mut prefix = path.to_string();
    prefix.remove(0);

    for (key, _) in env::vars_os() {
        let env_name = key.to_string_lossy().to_string();
        if env_name.starts_with(&prefix) {
            res.push(Completion {
                completion: format!("${}", env_name),
                display: None,
                suffix: Suffix::Default,
            });
        }
    }
    res
}
