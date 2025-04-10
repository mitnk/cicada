use std::env;
use std::sync::Arc;

use lineread::complete::{Completer, Completion, Suffix};
use lineread::prompter::Prompter;
use lineread::terminal::Terminal;

use crate::shell;

pub struct EnvCompleter {
    pub sh: Arc<shell::Shell>,
}

impl<Term: Terminal> Completer<Term> for EnvCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let sh = Arc::try_unwrap(self.sh.clone());
        match sh {
            Ok(x) => Some(complete_env(&x, word)),
            Err(x) => Some(complete_env(&x, word)),
        }
    }
}

fn complete_env(sh: &shell::Shell, path: &str) -> Vec<Completion> {
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

    // sh.envs is a just clone here; see FIXME in main.rs
    for key in sh.envs.keys() {
        if key.starts_with(&prefix) {
            res.push(Completion {
                completion: format!("${}", key),
                display: None,
                suffix: Suffix::Default,
            });
        }
    }

    res
}
