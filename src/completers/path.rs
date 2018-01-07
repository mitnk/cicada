use std::collections::HashSet;
use std::env;
use std::fs::read_dir;
use std::iter::FromIterator;
use std::path::{is_separator, MAIN_SEPARATOR};
use std::os::unix::fs::PermissionsExt;

use linefeed::Reader;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion, Suffix};

use tools;

pub struct BinCompleter;
pub struct CdCompleter;
pub struct PathCompleter;

impl<Term: Terminal> Completer<Term> for BinCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Reader<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_bin(word))
    }
}


impl<Term: Terminal> Completer<Term> for PathCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Reader<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_path(word, false))
    }
}

impl<Term: Terminal> Completer<Term> for CdCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Reader<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_path(word, true))
    }
}

/// Returns a sorted list of paths whose prefix matches the given path.
pub fn complete_path(path: &str, for_dir: bool) -> Vec<Completion> {
    let (_dir_orig, _) = split_path(path);
    let dir_orig = if let Some(_dir) = _dir_orig { _dir } else { "" };
    let mut path_extended = String::from(path);
    if tools::needs_extend_home(path_extended.as_str()) {
        tools::extend_home(&mut path_extended)
    }
    let (_dir_lookup, file_name) = split_path(path_extended.as_str());
    let mut res = Vec::new();
    let dir_lookup = _dir_lookup.unwrap_or(".");
    if let Ok(entries) = read_dir(dir_lookup) {
        for entry in entries {
            let mut is_dir;
            if let Ok(entry) = entry {
                let pathbuf = entry.path();
                is_dir = pathbuf.is_dir();
                if for_dir && !is_dir {
                    continue;
                }

                let ent_name = entry.file_name();
                // TODO: Deal with non-UTF8 paths in some way
                if let Ok(_path) = ent_name.into_string() {
                    if _path.starts_with(file_name) {
                        let (name, display) = if dir_orig != "" {
                            (
                                format!("{}{}{}", dir_orig, MAIN_SEPARATOR, _path),
                                Some(_path),
                            )
                        } else {
                            (_path, None)
                        };
                        let name = str::replace(name.as_str(), "//", "/");
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

/// Returns a sorted list of paths whose prefix matches the given path.
fn complete_bin(path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let (_, fname) = split_path(path);
    let env_path;
    match env::var("PATH") {
        Ok(x) => env_path = x,
        Err(e) => {
            println!("cicada: env error when complete_bin: {:?}", e);
            return res;
        }
    }
    let vec_path: Vec<&str> = env_path.split(':').collect();
    let path_list: HashSet<&str> = HashSet::from_iter(vec_path.iter().cloned());

    let mut checker: HashSet<String> = HashSet::new();
    for p in &path_list {
        if let Ok(list) = read_dir(p) {
            for entry in list {
                if let Ok(entry) = entry {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(fname) {
                            let _mode;
                            match entry.metadata() {
                                Ok(x) => _mode = x,
                                Err(e) => {
                                    println!("cicada: metadata error: {:?}", e);
                                    continue;
                                }
                            }
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
