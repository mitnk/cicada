use std::borrow::Cow;
use std::env;
use std::fs::read_dir;
use std::io::{ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;

use regex::Regex;

use crate::tools;

pub fn basename(path: &str) -> Cow<'_, str> {
    let mut pieces = path.rsplit('/');
    match pieces.next() {
        Some(p) => p.into(),
        None => path.into(),
    }
}

pub fn expand_home(text: &str) -> String {
    let mut s: String = text.to_string();
    let v = vec![
        r"(?P<head> +)~(?P<tail> +)",
        r"(?P<head> +)~(?P<tail>/)",
        r"^(?P<head> *)~(?P<tail>/)",
        r"(?P<head> +)~(?P<tail> *$)",
    ];
    for item in &v {
        let re;
        if let Ok(x) = Regex::new(item) {
            re = x;
        } else {
            return String::new();
        }
        let home = tools::get_user_home();
        let ss = s.clone();
        let to = format!("$head{}$tail", home);
        let result = re.replace_all(ss.as_str(), to.as_str());
        s = result.to_string();
    }
    s
}

pub fn find_file_in_path(filename: &str, exec: bool) -> String {
    let env_path = match env::var("PATH") {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: error with env PATH: {:?}", e);
            return String::new();
        }
    };
    let vec_path = env::split_paths(&env_path);
    for p in vec_path {
        match read_dir(&p) {
            Ok(list) => {
                for entry in list.flatten() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name != filename {
                            continue;
                        }

                        if exec {
                            let _mode = match entry.metadata() {
                                Ok(x) => x,
                                Err(e) => {
                                    println_stderr!("cicada: metadata error: {:?}", e);
                                    continue;
                                }
                            };
                            let mode = _mode.permissions().mode();
                            if mode & 0o111 == 0 {
                                // not binary
                                continue;
                            }
                        }

                        return entry.path().to_string_lossy().to_string();
                    }
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    continue;
                }
                log!("cicada: fs read_dir error: {}: {}", p.display(), e);
            }
        }
    }
    String::new()
}

pub fn current_dir() -> String {
    let _current_dir = match env::current_dir() {
        Ok(x) => x,
        Err(e) => {
            log!("cicada: PROMPT: env current_dir error: {}", e);
            return String::new();
        }
    };
    let current_dir = match _current_dir.to_str() {
        Some(x) => x,
        None => {
            log!("cicada: PROMPT: to_str error");
            return String::new();
        }
    };

    current_dir.to_string()
}
