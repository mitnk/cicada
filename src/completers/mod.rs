use std::path::Path;
use std::sync::Arc;

use lineread::complete::{Completer, Completion};
use lineread::prompter::Prompter;
use lineread::terminal::Terminal;

pub mod dots;
pub mod env;
pub mod make;
pub mod path;
pub mod ssh;
pub mod utils;

use crate::libs;
use crate::libs::prefix;
use crate::parsers;
use crate::shell;
use crate::tools;

pub struct CicadaCompleter {
    pub sh: Arc<shell::Shell>,
}

fn for_make(line: &str) -> bool {
    match prefix::get_effective_command(line) {
        Some(cmd) => cmd == "make",
        None => false,
    }
}

fn for_env(line: &str) -> bool {
    libs::re::re_contains(line, r" *\$[_a-zA-Z0-9]*$")
}

fn for_ssh(line: &str) -> bool {
    match prefix::get_effective_command(line) {
        Some(cmd) => {
            if cmd != "ssh" && cmd != "scp" {
                return false;
            }
            // Also need to check we're completing a hostname (not a path)
            let segment = prefix::get_current_segment(line);
            libs::re::re_contains(segment, r".* +[^ \./]+ *$")
        }
        None => false,
    }
}

fn for_cd(line: &str) -> bool {
    match prefix::get_effective_command(line) {
        Some(cmd) => {
            if cmd != "cd" {
                return false;
            }
            // Make sure there's a space after cd (we're completing an argument)
            let segment = prefix::get_current_segment(line);
            libs::re::re_contains(segment, r"cd +")
        }
        None => false,
    }
}

fn for_bin(line: &str) -> bool {
    // Check if we're in command position (completing a command name)
    let segment = prefix::get_current_segment(line).trim_start();
    if segment.is_empty() {
        return true; // Empty segment means we're at command position
    }

    let tokens = parsers::parser_line::line_to_plain_tokens(segment);
    if tokens.is_empty() {
        return true;
    }

    // Check each token - we're in command position if all tokens so far
    // are command wrappers or env assignments
    for token in &tokens {
        if prefix::is_env_assignment(token) {
            continue;
        }
        if prefix::is_wrapper_command(token) {
            continue;
        }
        // Found a non-wrapper, non-env-assignment token
        // If the line ends with space, we're past command position
        // If not, we might be completing this token as a command
        if segment.ends_with(' ') || segment.ends_with('\t') {
            return false;
        }
        // We're completing the last token - check if it looks like a command name
        return tokens.len() == 1 || (tokens.len() > 1 && token == tokens.last().unwrap());
    }

    // All tokens were wrappers/env - still in command position
    true
}

fn for_dots(line: &str) -> bool {
    match prefix::get_effective_command(line) {
        Some(cmd) => {
            let dir = tools::get_user_completer_dir();
            let dot_file = format!("{}/{}.yaml", dir, cmd);
            Path::new(dot_file.as_str()).exists()
        }
        None => false,
    }
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

        let completions: Option<Vec<Completion>>;
        if for_dots(line) {
            let cpl = Arc::new(dots::DotsCompleter);
            completions = cpl.complete(word, reader, start, _end);
        } else if for_ssh(line) {
            let cpl = Arc::new(ssh::SshCompleter);
            completions = cpl.complete(word, reader, start, _end);
        } else if for_make(line) {
            let cpl = Arc::new(make::MakeCompleter);
            completions = cpl.complete(word, reader, start, _end);
        } else if for_bin(line) {
            let cpl = Arc::new(path::BinCompleter {
                sh: self.sh.clone(),
            });
            completions = cpl.complete(word, reader, start, _end);
        } else if for_env(line) {
            let cpl = Arc::new(env::EnvCompleter {
                sh: self.sh.clone(),
            });
            completions = cpl.complete(word, reader, start, _end);
        } else if for_cd(line) {
            // `for_cd` should be put a bottom position, so that
            // `cd $SOME_ENV_<TAB>` works as expected.
            let cpl = Arc::new(path::CdCompleter);
            // completions for `cd` should not fail back to path-completion
            return cpl.complete(word, reader, start, _end);
        } else {
            completions = None;
        }

        if let Some(x) = completions {
            if !x.is_empty() {
                return Some(x);
            }
        }

        // empty completions should fail back to path-completion,
        // so that `$ make generate /path/to/fi<Tab>` still works.
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
