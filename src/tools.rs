use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use glob;
use regex::Regex;
use shellexpand;

use libc;
use shlex;


pub fn rlog(s: String) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/cicada-debug.log")
        .unwrap();
    let pid = unsafe { libc::getpid() };
    let s = format!("[{}] {}", pid, s);
    file.write_all(s.as_bytes()).unwrap();
}

pub fn get_user_home() -> String {
    let home = env::var("HOME").unwrap();
    return home;
}

pub fn get_user_completer_dir() -> String {
    let home = get_user_home();
    return format!("{}/.cicada/completers", home);
}

pub fn get_rc_file() -> String {
    let home = get_user_home();
    return format!("{}/{}", home, ".cicadarc");
}

pub fn unquote(s: &str) -> String {
    let args;
    if let Some(x) = shlex::split(s.trim()) {
        args = x;
    } else {
        return String::new();
    }
    if args.len() == 0 {
        return String::new();
    }
    return args[0].clone();
}

pub fn is_env(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^ *export +[a-zA-Z0-9_\.-]+=.*$") {
        re = x;
    } else {
        return false;
    }
    return re.is_match(line);
}

pub fn extend_home(s: &mut String) {
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
            return;
        }
        let home = get_user_home();
        let ss = s.clone();
        let to = format!("$head{}$tail", home);
        let result = re.replace_all(ss.as_str(), to.as_str());
        *s = result.to_string();
    }
}

pub fn needs_extend_home(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"( +~ +)|( +~/)|(^ *~/)|( +~ *$)") {
        re = x;
    } else {
        return false;
    }
    return re.is_match(line);
}

fn extend_env(line: &mut String) {
    let mut result: Vec<String> = Vec::new();
    let _line = line.clone();
    let _tokens: Vec<&str> = _line.split(" ").collect();
    for item in &_tokens {
        if item.trim().starts_with("'") {
            result.push(item.to_string());
        } else {
            match shellexpand::env(item) {
                Ok(x) => {
                    result.push(x.into_owned());
                }
                Err(e) => {
                    println!("shellexpand error: {:?}", e);
                    result.push(item.to_string());
                }
            }
        }
    }
    *line = result.join(" ");
}

fn needs_globbing(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"[\*]+") {
        re = x;
    } else {
        return false;
    }
    return re.is_match(line);
}

fn extend_glob(line: &mut String) {
    let _line = line.clone();
    // XXX: spliting needs to consider cases like `echo 'a * b'`
    let _tokens: Vec<&str> = _line.split(" ").collect();
    let mut result: Vec<String> = Vec::new();
    for item in &_tokens {
        if item.trim().starts_with("'") || item.trim().starts_with("\"")
                || !needs_globbing(item) {
            result.push(item.to_string());
        } else {
            match glob::glob(item) {
                Ok(paths) => {
                    let mut is_empty = true;
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                let s = path.to_string_lossy();
                                if s.starts_with(".") {
                                    continue;
                                }
                                result.push(s.into_owned());
                                is_empty = false;
                            }
                            Err(e) => println!("{:?}", e),
                        }
                    }
                    if is_empty {
                        result.push(item.to_string());
                    }
                }
                Err(e) => {
                    println!("glob error: {:?}", e);
                    result.push(item.to_string());
                    return;
                }
            }
        }
    }
    *line = result.join(" ");
}

pub fn pre_handle_cmd_line(line: &mut String) {
    // TODO maybe replace extend_home() with shellexpand::full()
    if needs_extend_home(line.as_str()) {
        extend_home(line);
    }
    if needs_globbing(line.as_str()) {
        extend_glob(line);
    }
    extend_env(line);
}

pub fn env_args_to_command_line() -> String {
    let mut result = String::new();
    let env_args = env::args();
    if env_args.len() <= 1 {
        return result;
    }
    for (i, arg) in env_args.enumerate() {
        if i == 0 || arg == "-c" {
            continue;
        }
        result.push_str(arg.as_str());
    }
    return result;
}

pub fn is_alias(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^ *alias +[a-zA-Z0-9_\.-]+=.*$") {
        re = x;
    } else {
        return false;
    }
    return re.is_match(line);
}

#[cfg(test)]
mod tests {
    use super::needs_extend_home;
    use super::is_alias;
    use super::extend_env;

    #[test]
    fn dots_test() {
        assert!(needs_extend_home("ls ~"));
        assert!(needs_extend_home("ls  ~  "));
        assert!(needs_extend_home("cat ~/a.py"));
        assert!(needs_extend_home("echo ~"));
        assert!(needs_extend_home("echo ~ ~~"));
        assert!(needs_extend_home("~/bin/py"));
        assert!(!needs_extend_home("echo '~'"));
        assert!(!needs_extend_home("echo \"~\""));
        assert!(!needs_extend_home("echo ~~"));
    }

    #[test]
    fn test_is_alias() {
        assert!(is_alias("alias ls='ls -lh'"));
    }

    #[test]
    fn test_extend_env() {
        let mut s = String::from("echo '$PATH'");
        extend_env(&mut s);
        assert_eq!(s, "echo '$PATH'");
    }
}
