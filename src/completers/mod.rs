use std::path::Path;
use std::sync::Arc;

use linefeed::prompter::Prompter;
use linefeed::complete::{Completer, Completion};
use linefeed::terminal::Terminal;
use regex::Regex;

pub mod dots;
pub mod make;
pub mod path;
pub mod ssh;

use parsers;
use shell;
use tools;

pub struct CicadaCompleter {
    pub sh: Arc<shell::Shell>,
}

fn for_make(line: &str) -> bool {
    tools::re_contains(line, r"^ *make ")
}

fn for_ssh(line: &str) -> bool {
    tools::re_contains(line, r"^ *(ssh|scp).* +[^ \./]+ *$")
}

fn for_cd(line: &str) -> bool {
    tools::re_contains(line, r"^ *cd +")
}

fn for_bin(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"(^ *[a-zA-Z0-9_\.-]+$)|(^.+\| +[a-zA-Z0-9_\.-]+$)") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
}

fn for_dots(line: &str) -> bool {
    let args = parsers::parser_line::line_to_plain_tokens(line);
    let len = args.len();
    if len == 0 {
        return false;
    }
    let dir = tools::get_user_completer_dir();
    let dot_file = format!("{}/{}.yaml", dir, args[0]);
    Path::new(dot_file.as_str()).exists()
}

impl<Term: Terminal> Completer<Term> for CicadaCompleter {
    fn complete(
        &self,
        word: &str,
        reader: &Prompter<Term>,
        start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let line = reader.buffer();

        // these completions should not fail back to path completion.
        if for_bin(line) {
            let cpl = Arc::new(path::BinCompleter{sh: self.sh.clone()});
            return cpl.complete(word, reader, start, _end);
        }
        if for_cd(line) {
            let cpl = Arc::new(path::CdCompleter);
            return cpl.complete(word, reader, start, _end);
        }

        // the following completions needs fail back to use path completion,
        // so that `$ make generate /path/to/fi<Tab>` still works.
        if for_ssh(line) {
            let cpl = Arc::new(ssh::SshCompleter);
            if let Some(x) = cpl.complete(word, reader, start, _end) {
                if !x.is_empty() {
                    return Some(x);
                }
            }
        }
        if for_make(line) {
            let cpl = Arc::new(make::MakeCompleter);
            if let Some(x) = cpl.complete(word, reader, start, _end) {
                if !x.is_empty() {
                    return Some(x);
                }
            }
        }
        if for_dots(line) {
            let cpl = Arc::new(dots::DotsCompleter);
            if let Some(x) = cpl.complete(word, reader, start, _end) {
                if !x.is_empty() {
                    return Some(x);
                }
            }
        }

        let cpl = Arc::new(path::PathCompleter);
        cpl.complete(word, reader, start, _end)
    }
}
