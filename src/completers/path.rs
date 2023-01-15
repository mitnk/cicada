use std::collections::HashSet;
use std::env;
use std::fs::read_dir;
use std::io::Write;
use std::iter::FromIterator;
use std::os::unix::fs::PermissionsExt;
use std::path::MAIN_SEPARATOR;
use std::sync::Arc;

use linefeed::complete::{Completer, Completion, Suffix};
use linefeed::terminal::Terminal;
use linefeed::Prompter;

use crate::completers::utils;
use crate::libs;
use crate::parsers;
use crate::shell;
use crate::tools;

pub struct BinCompleter {
    pub sh: Arc<shell::Shell>,
}
pub struct CdCompleter;
pub struct PathCompleter;

fn is_env_prefix(line: &str) -> bool {
    libs::re::re_contains(line, r" *\$[a-zA-Z_][A-Za-z0-9_]*")
}

fn is_pipelined(path: &str) -> bool {
    if !path.contains('|') {
        return false;
    }
    !path.starts_with('"') && !path.starts_with('\'')
}

impl<Term: Terminal> Completer<Term> for BinCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let sh = Arc::try_unwrap(self.sh.clone());
        match sh {
            Ok(x) => Some(complete_bin(&x, word)),
            Err(x) => Some(complete_bin(&x, word)),
        }
    }
}

impl<Term: Terminal> Completer<Term> for PathCompleter {
    fn complete(
        &self,
        word: &str,
        _reader: &Prompter<Term>,
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
        _reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        Some(complete_path(word, true))
    }
}

fn needs_expand_home(line: &str) -> bool {
    libs::re::re_contains(line, r"( +~ +)|( +~/)|(^ *~/)|( +~ *$)")
}

/// Returns a sorted list of paths whose prefix matches the given path.
pub fn complete_path(word: &str, for_dir: bool) -> Vec<Completion> {
    let is_env = is_env_prefix(word);
    let mut res = Vec::new();
    let linfo = parsers::parser_line::parse_line(word);
    let tokens = linfo.tokens;
    let (path, path_sep) = if tokens.is_empty() {
        (String::new(), String::new())
    } else {
        let (ref _path_sep, ref _path) = tokens[tokens.len() - 1];
        (_path.clone(), _path_sep.clone())
    };

    let (_, _dir_orig, _f) = split_pathname(&path, "");
    let dir_orig = if _dir_orig.is_empty() {
        String::new()
    } else {
        _dir_orig.clone()
    };
    let mut path_extended = path.clone();
    if needs_expand_home(&path_extended) {
        utils::expand_home_string(&mut path_extended)
    }
    utils::expand_env_string(&mut path_extended);

    let (_, _dir_lookup, file_name) = split_pathname(&path_extended, "");
    let dir_lookup = if _dir_lookup.is_empty() {
        ".".to_string()
    } else {
        _dir_lookup.clone()
    };
    // let dir_lookup = _dir_lookup.unwrap_or(".");
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
                    if _path.starts_with(&file_name) {
                        let (name, display) = if dir_orig != "" {
                            (
                                format!("{}{}{}", dir_orig, MAIN_SEPARATOR, _path),
                                Some(_path),
                            )
                        } else {
                            (_path, None)
                        };
                        let mut name = str::replace(name.as_str(), "//", "/");
                        if path_sep.is_empty() && !is_env {
                            name = tools::escape_path(&name);
                        }
                        let mut quoted = false;
                        if !path_sep.is_empty() {
                            name = tools::wrap_sep_string(&path_sep, &name);
                            quoted = true;
                        }
                        let suffix = if is_dir {
                            if quoted {
                                name.pop();
                            }
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
    res.sort_by(|a, b| a.completion.cmp(&b.completion));
    res
}

// Split optional directory and prefix. (see its test cases for more details)
fn split_pathname(path: &str, prefix: &str) -> (String, String, String) {
    if is_pipelined(path) {
        let tokens: Vec<&str> = path.rsplitn(2, '|').collect();
        let prefix = format!("{}|", tokens[1]);
        return split_pathname(tokens[0], &prefix);
    }
    match path.rfind('/') {
        Some(pos) => (
            prefix.to_string(),
            (&path[..=pos]).to_string(),
            (&path[pos + 1..]).to_string(),
        ),
        None => (prefix.to_string(), String::new(), path.to_string()),
    }
}

/// Returns a sorted list of paths whose prefix matches the given path.
fn complete_bin(sh: &shell::Shell, path: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let (prefix, _, fname) = split_pathname(path, "");
    let env_path;

    match env::var("PATH") {
        Ok(x) => env_path = x,
        Err(e) => {
            println_stderr!("cicada: env error when complete_bin: {:?}", e);
            return res;
        }
    }

    let mut checker: HashSet<String> = HashSet::new();

    // handle alias, builtins, and functions
    for func in sh.funcs.keys() {
        if !func.starts_with(&fname) {
            continue;
        }
        if checker.contains(func) {
            continue;
        }
        checker.insert(func.clone());
        res.push(Completion {
            completion: func.to_owned(),
            display: None,
            suffix: Suffix::Default,
        });
    }
    for alias in sh.alias.keys() {
        if !alias.starts_with(&fname) {
            continue;
        }
        if checker.contains(alias) {
            continue;
        }
        checker.insert(alias.clone());
        res.push(Completion {
            completion: alias.to_owned(),
            display: None,
            suffix: Suffix::Default,
        });
    }

    let builtins = vec![
        "alias", "bg", "cd", "cinfo", "exec", "exit", "export", "fg",
        "history", "jobs", "read", "source", "ulimit", "unalias", "vox",
        "minfd", "set", "unset", "unpath",
    ];
    for item in &builtins {
        if !item.starts_with(&fname) {
            continue;
        }
        if checker.contains(item.clone()) {
            continue;
        }
        checker.insert(item.to_string());
        res.push(Completion {
            completion: item.to_string(),
            display: None,
            suffix: Suffix::Default,
        });
    }

    let vec_path: Vec<&str> = env_path.split(':').collect();
    let path_list: HashSet<&str> = HashSet::from_iter(vec_path.iter().cloned());

    for p in &path_list {
        if let Ok(list) = read_dir(p) {
            for entry in list {
                if let Ok(entry) = entry {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with(&fname) {
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
                            // TODO: need to handle quoted: `$ "foo#bar"`
                            let name_e = tools::escape_path(&name);
                            let name_e = format!("{}{}", prefix, name_e);
                            res.push(Completion {
                                completion: name_e,
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

#[cfg(test)]
mod tests {
    use super::needs_expand_home;
    use super::split_pathname;

    #[test]
    fn test_split_pathname() {
        assert_eq!(
            split_pathname("", ""),
            (String::new(), String::new(), String::new(),)
        );
        assert_eq!(
            split_pathname("hi|ech", ""),
            ("hi|".to_string(), String::new(), "ech".to_string())
        );
        assert_eq!(
            split_pathname("hi|/bin/ech", ""),
            ("hi|".to_string(), "/bin/".to_string(), "ech".to_string())
        );
        assert_eq!(
            split_pathname("foo", "aprefix"),
            ("aprefix".to_string(), String::new(), "foo".to_string())
        );
    }

    #[test]
    fn test_need_expand_home() {
        assert!(needs_expand_home("ls ~"));
        assert!(needs_expand_home("ls  ~  "));
        assert!(needs_expand_home("cat ~/a.py"));
        assert!(needs_expand_home("echo ~"));
        assert!(needs_expand_home("echo ~ ~~"));
        assert!(needs_expand_home("~/bin/py"));
        assert!(!needs_expand_home("echo '~'"));
        assert!(!needs_expand_home("echo \"~\""));
        assert!(!needs_expand_home("echo ~~"));
    }
}
