use std::borrow::Cow;
use std::collections::HashSet;
use std::env;
use std::fs::read_dir;
use std::iter::FromIterator;
use std::path::{is_separator, MAIN_SEPARATOR};
use std::os::unix::fs::PermissionsExt;

use linefeed::Reader;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion};
use linefeed::complete::Suffix;
use linefeed::complete::escape;
use linefeed::complete::unescape;
use linefeed::complete::escaped_word_start;

use tools;


/// Performs completion by searching for filenames matching the word prefix.
pub struct PathCompleter;

impl<Term: Terminal> Completer<Term> for PathCompleter {
    fn complete(&self,
                word: &str,
                _reader: &Reader<Term>,
                _start: usize,
                _end: usize)
                -> Option<Vec<Completion>> {
        Some(complete_path(word))
    }

    fn word_start(&self, line: &str, end: usize, _reader: &Reader<Term>) -> usize {
        escaped_word_start(&line[..end])
    }

    fn quote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        escape(word)
    }

    fn unquote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        unescape(word)
    }
}

/// Returns a sorted list of paths whose prefix matches the given path.
fn complete_path(path: &str) -> Vec<Completion> {
    let mut path_s = String::from(path);
    if tools::needs_extend_home(path_s.as_str()) {
        tools::extend_home(&mut path_s)
    }
    let (base_dir, fname) = split_path(path_s.as_str());
    let mut res = Vec::new();

    let lookup_dir = base_dir.unwrap_or(".");

    if let Ok(list) = read_dir(lookup_dir) {
        for ent in list {
            if let Ok(ent) = ent {
                let ent_name = ent.file_name();

                // TODO: Deal with non-UTF8 paths in some way
                if let Ok(path) = ent_name.into_string() {
                    if path.starts_with(fname) {
                        let (name, display) = if let Some(dir) = base_dir {
                            (format!("{}{}{}", dir, MAIN_SEPARATOR, path), Some(path))
                        } else {
                            (path, None)
                        };
                        let name = str::replace(name.as_str(), "//", "/");
                        let is_dir = ent.metadata().ok().map_or(false, |m| m.is_dir());

                        let suffix = if is_dir {
                            Suffix::Some(MAIN_SEPARATOR)
                        } else {
                            Suffix::Default
                        };

                        res.push(Completion {
                                     completion: name,
                                     display: display,
                                     suffix: suffix,
                                 });
                    }
                }
            }
        }
    }
    res
}

fn split_path(path: &str) -> (Option<&str>, &str) {
    match path.rfind(is_separator) {
        Some(pos) => (Some(&path[..pos + 1]), &path[pos + 1..]),
        None => (None, path),
    }
}

pub struct BinCompleter;

impl<Term: Terminal> Completer<Term> for BinCompleter {
    fn complete(&self,
                word: &str,
                _reader: &Reader<Term>,
                _start: usize,
                _end: usize)
                -> Option<Vec<Completion>> {
        Some(complete_bin(word))
    }
}

/// Returns a sorted list of paths whose prefix matches the given path.
fn complete_bin(path: &str) -> Vec<Completion> {
    let (_, fname) = split_path(path);

    let env_path = env::var("PATH").expect("cicada: env error");
    let vec_path: Vec<&str> = env_path.split(':').collect();
    let path_list: HashSet<&str> = HashSet::from_iter(vec_path.iter().cloned());

    let mut res = Vec::new();
    let mut checker: HashSet<String> = HashSet::new();
    for p in &path_list {
        if let Ok(list) = read_dir(p) {
            for entry in list {
                if let Ok(entry) = entry {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(fname) {
                            let _mode = entry.metadata().expect("cicada: metadata error");
                            let mode = _mode.permissions().mode();
                            if mode & 0o111 == 0 {
                                // not binary
                                continue;
                            }
                            if checker.contains(&name) {
                                continue;
                            }

                            let display = None;
                            let suffix = Suffix::Default;
                            checker.insert(name.clone());
                            res.push(Completion {
                                         completion: name,
                                         display: display,
                                         suffix: suffix,
                                     });
                        }
                    }
                }
            }
        }
    }
    res
}
