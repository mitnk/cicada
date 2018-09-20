use std::collections::HashSet;
use std::env;
use std::fs::read_dir;
use std::io::Write;
use std::iter::FromIterator;
use std::os::unix::fs::PermissionsExt;
use std::path::{is_separator, MAIN_SEPARATOR};
use std::sync::Arc;

use linefeed::complete::{Completer, Completion, Suffix};
use linefeed::terminal::Terminal;
use linefeed::Prompter;

use parsers;
use shell;

pub struct BinCompleter {
    pub sh: Arc<shell::Shell>,
}
pub struct CdCompleter;
pub struct PathCompleter;

impl<Term: Terminal> Completer<Term> for BinCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        // TODO: use RC::into_raw() instead
        let sh = Arc::try_unwrap(self.sh.clone());
        match sh {
            Ok(x) => Some(complete_bin(&x, word)),
            Err(e) => Some(complete_bin(&e, word)),
        }
    }
}

impl<Term: Terminal> Completer<Term> for PathCompleter {
    fn complete(
        &self,
        _word: &str,
        reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let buffer = reader.buffer();
        Some(complete_path(buffer, false))
    }
}

impl<Term: Terminal> Completer<Term> for CdCompleter {
    fn complete(
        &self,
        _word: &str,
        reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let buffer = reader.buffer();
        Some(complete_path(buffer, true))
    }
}

/// Returns a sorted list of paths whose prefix matches the given path.
pub fn complete_path(buffer: &str, for_dir: bool) -> Vec<Completion> {
    let mut res = Vec::new();
    let mut path_sep = String::new();
    let mut path = String::new();
    if !buffer.ends_with(' ') {
        let tokens = parsers::parser_line::cmd_to_tokens(buffer);
        if tokens.is_empty() {
            return res;
        }

        let (ref _path_sep, ref _path) = tokens[tokens.len() - 1];
        path = _path.clone();
        path_sep = _path_sep.clone();
    }
    let (_dir_orig, _) = split_path(&path);
    let dir_orig = if let Some(_dir) = _dir_orig { _dir } else { "" };
    // let mut path_extended = String::from(path);
    let mut path_extended = path.clone();
    if shell::needs_expand_home(path_extended.as_str()) {
        shell::expand_home_string(&mut path_extended)
    }
    let (_dir_lookup, file_name) = split_path(path_extended.as_str());
    let dir_lookup = _dir_lookup.unwrap_or(".");
    if let Ok(entries) = read_dir(dir_lookup) {
        for entry in entries {
            if let Ok(entry) = entry {
                let pathbuf = entry.path();
                let is_dir = pathbuf.is_dir();
                if for_dir && !is_dir {
                    continue;
                }

                let entry_name = entry.file_name();
                // TODO: Deal with non-UTF8 paths in some way
                if let Ok(_path) = entry_name.into_string() {
                    if _path.starts_with(file_name) {
                        let (name, display) = if dir_orig != "" {
                            (
                                format!("{}{}{}", dir_orig, MAIN_SEPARATOR, _path),
                                Some(_path),
                            )
                        } else {
                            (_path, None)
                        };
                        let mut name = str::replace(name.as_str(), "//", "/");
                        if path_sep.is_empty() {
                            name = str::replace(name.as_str(), " ", "\\ ");
                        }
                        let suffix = if is_dir {
                            Suffix::Some(MAIN_SEPARATOR)
                        } else {
                            Suffix::Default
                        };
                        res.push(Completion {
                            completion: name,
                            display,
                            suffix,
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
fn complete_bin(sh: &shell::Shell, path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let (_, fname) = split_path(path);
    let env_path;
    match env::var("PATH") {
        Ok(x) => env_path = x,
        Err(e) => {
            println_stderr!("cicada: env error when complete_bin: {:?}", e);
            return res;
        }
    }

    // handle alias and builtins
    for alias in sh.alias.keys() {
        if !alias.starts_with(fname) {
            continue;
        }
        res.push(Completion {
            completion: alias.to_owned(),
            display: None,
            suffix: Suffix::Default,
        });
    }
    let builtins = vec!["cd", "cinfo", "exec", "exit", "export", "history", "vox"];
    for item in &builtins {
        if !item.starts_with(fname) {
            continue;
        }
        res.push(Completion {
            completion: item.to_string(),
            display: None,
            suffix: Suffix::Default,
        });
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
                                    println_stderr!("cicada: metadata error: {:?}", e);
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
                                display,
                                suffix,
                            });
                        }
                    }
                }
            }
        }
    }
    res
}
