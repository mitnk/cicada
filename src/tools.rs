use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use regex::Regex;

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

/// in docs of `linefeed::reader::Reader.set_prompt()`:
/// If prompt contains any terminal escape sequences, such escape sequences
/// should be immediately preceded by the character '\x01' and immediately
/// followed by the character '\x02'.
pub fn wrap_seq_chars(s: String) -> String {
    return format!("\x01{}\x02", s);
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

pub fn pre_handle_cmd_line(s: &mut String) {
    if needs_extend_home(s.as_str()) {
        extend_home(s);
    }
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
}
