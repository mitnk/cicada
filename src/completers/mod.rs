use std::path::Path;
use std::sync::Arc;

use linefeed::complete::{Completer, Completion};
use linefeed::prompter::Prompter;
use linefeed::terminal::Terminal;

pub mod dots;
pub mod env;
pub mod make;
pub mod path;
pub mod ssh;

use crate::libs;
use crate::parsers;
use crate::shell;
use crate::tools;

pub struct CicadaCompleter {
    pub sh: Arc<shell::Shell>,
}

fn for_make(line: &str) -> bool {
    libs::re::re_contains(line, r"^ *make ")
}

fn for_env(line: &str) -> bool {
    libs::re::re_contains(line, r" *\$[_a-zA-Z0-9]*$")
}

fn for_ssh(line: &str) -> bool {
    libs::re::re_contains(line, r"^ *(ssh|scp).* +[^ \./]+ *$")
}

fn for_cd(line: &str) -> bool {
    libs::re::re_contains(line, r"^ *cd +")
}

fn for_bin(line: &str) -> bool {
    // TODO: why 'echo hi|ech<TAB>' doesn't complete in real?
    // but passes in test cases?
    let ptn = r"(^ *(sudo|which)? *[a-zA-Z0-9_\.-]+$)|(^.+\| *(sudo|which)? *[a-zA-Z0-9_\.-]+$)";
    libs::re::re_contains(line, ptn)
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
            let cpl = Arc::new(path::BinCompleter {
                sh: self.sh.clone(),
            });
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
        if for_env(line) {
            let cpl = Arc::new(env::EnvCompleter);
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

    fn word_start(&self, line: &str, end: usize, _reader: &Prompter<Term>) -> usize {
        escaped_word_start(&line[..end])
    }
}

pub fn escaped_word_start(line: &str) -> usize {
    let mut start_position: usize = 0;
    let mut found_bs = false;
    let mut found_space = false;
    let mut with_quote = false;
    let mut ch_quote = '\0';
    let mut extra_bytes = 0;
    for (i, c) in line.chars().enumerate() {
        if found_space {
            found_space = false;
            start_position = i + extra_bytes;
        }

        if c == '\\' {
            found_bs = true;
            continue;
        }
        if c == ' ' && !found_bs && !with_quote {
            found_space = true;
            continue;
        }

        if !with_quote && !found_bs && (c == '"' || c == '\'') {
            with_quote = true;
            ch_quote = c;
        } else if with_quote && !found_bs && ch_quote == c {
            with_quote = false;
        }

        let bytes_c = c.len_utf8();
        if bytes_c > 1 {
            extra_bytes += bytes_c - 1;
        }
        found_bs = false;
    }
    if found_space {
        start_position = line.len();
    }
    start_position
}

#[cfg(test)]
mod tests {
    use super::escaped_word_start;
    use super::for_bin;

    #[test]
    fn test_escaped_word_start() {
        assert_eq!(escaped_word_start("ls a"), 3);
        assert_eq!(escaped_word_start("ls abc"), 3);
        assert_eq!(escaped_word_start("ll 中文yoo"), 3);
        assert_eq!(escaped_word_start("ll yoo中文"), 3);

        assert_eq!(escaped_word_start("  ls   foo"), 7);
        assert_eq!(escaped_word_start("ls foo bar"), 7);
        assert_eq!(escaped_word_start("ls føo bar"), 8);

        assert_eq!(escaped_word_start("ls a\\ "), 3);
        assert_eq!(escaped_word_start("ls a\\ b"), 3);
        assert_eq!(escaped_word_start("ls a\\ b\\ c"), 3);
        assert_eq!(escaped_word_start("  ls   a\\ b\\ c"), 7);
        assert_eq!(escaped_word_start("mv foo\\ bar abc"), 12);
        assert_eq!(escaped_word_start("mv føo\\ bar abc"), 13);

        assert_eq!(escaped_word_start("ls a\\'"), 3);
        assert_eq!(escaped_word_start("ls a\\'b"), 3);
        assert_eq!(escaped_word_start("ls a\\'b\\'c"), 3);
        assert_eq!(escaped_word_start("  ls   a\\'b\\'c"), 7);

        assert_eq!(escaped_word_start("ls a\\\""), 3);
        assert_eq!(escaped_word_start("ls a\\\"b"), 3);
        assert_eq!(escaped_word_start("ls a\\\"b\\\"c"), 3);
        assert_eq!(escaped_word_start("  ls   a\\\"b\\\"c"), 7);

        assert_eq!(escaped_word_start("ls \"a'b'c"), 3);
        assert_eq!(escaped_word_start("ls \'a\"b\"c"), 3);

        assert_eq!(escaped_word_start("rm "), 3);
        assert_eq!(escaped_word_start("ls a "), 5);
        assert_eq!(escaped_word_start("  ls   foo "), 11);

        assert_eq!(escaped_word_start("ls \"a b"), 3);
        assert_eq!(escaped_word_start("ls \"a "), 3);
        assert_eq!(escaped_word_start("ls \"a b "), 3);
        assert_eq!(escaped_word_start("ls \'a b"), 3);
        assert_eq!(escaped_word_start("ls \'a "), 3);
        assert_eq!(escaped_word_start("ls \'a b "), 3);
        assert_eq!(escaped_word_start("\"ls\" \"a b"), 5);

        assert_eq!(escaped_word_start("echo føo b"), 10);
        assert_eq!(escaped_word_start("echo føo "), 10);

        assert_eq!(escaped_word_start("echo \\["), 5);
    }

    #[test]
    fn test_for_bin() {
        assert!(for_bin("foo"));
        assert!(for_bin("foo|bar"));
        assert!(for_bin("foo|bar|baz"));
        assert!(for_bin("foo | bar"));
        assert!(for_bin("foo | bar | baz"));
        assert!(!for_bin("foo bar"));
        assert!(!for_bin("foo bar | foo bar"));
    }
}
