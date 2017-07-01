use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use time;

use glob;
use regex::Regex;

use libc;
use parsers;
use execute;
use shell;

macro_rules! println_stderr {
    ($fmt:expr) => (
        match writeln!(&mut ::std::io::stderr(), concat!($fmt, "\n")) {
            Ok(_) => {}
            Err(e) => println!("write to stderr failed: {:?}", e)
        }
    );
    ($fmt:expr, $($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), concat!($fmt, "\n"), $($arg)*) {
            Ok(_) => {}
            Err(e) => println!("write to stderr failed: {:?}", e)
        }
    );
}

pub fn rlog(s: &str) {
    let mut file;
    match OpenOptions::new().append(true).create(true).open(
        "/tmp/cicada-debug.log",
    ) {
        Ok(x) => file = x,
        Err(e) => {
            println!("rlog: open /tmp/cicada-debug.log faild: {:?}", e);
            return;
        }
    }
    let pid = unsafe { libc::getpid() };
    let now = time::now();
    let s =
        format!(
        "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}][{}] {}",
        now.tm_year + 1900,
        now.tm_mon + 1,
        now.tm_mday,
        now.tm_hour,
        now.tm_min,
        now.tm_sec,
        pid,
        s,
    );
    match file.write_all(s.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            println!("rlog: write_all failed: {:?}", e);
            return;
        }
    }
}

macro_rules! log {
    ($fmt:expr) => (
        rlog(concat!($fmt, "\n"));
    );
    ($fmt:expr, $($arg:tt)*) => (
        rlog(format!(concat!($fmt, "\n"), $($arg)*).as_str());
    );
}

pub fn get_user_home() -> String {
    match env::var("HOME") {
        Ok(x) => x,
        Err(e) => {
            println!("cicada: env HOME error: {:?}", e);
            String::new()
        }
    }
}

pub fn get_user_completer_dir() -> String {
    let home = get_user_home();
    format!("{}/.cicada/completers", home)
}

pub fn get_rc_file() -> String {
    let home = get_user_home();
    format!("{}/{}", home, ".cicadarc")
}

pub fn unquote(s: &str) -> String {
    let args = parsers::parser_line::parse_line(s);
    if args.is_empty() {
        return String::new();
    }
    args[0].clone()
}

pub fn is_env(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^ *export +[a-zA-Z0-9_]+=.*$") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
}

fn should_extend_brace(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"\{.*,.*\}") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
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
    re.is_match(line)
}

pub fn wrap_sep_string(sep: &str, s: &str) -> String {
    let mut _token = String::new();
    for c in s.chars() {
        if c.to_string() == sep {
            _token.push('\\');
        }
        _token.push(c);
    }
    format!("{}{}{}", sep, _token, sep)
}

pub fn do_command_substitution(line: &mut String) {
    let _line = line.clone();
    let args = parsers::parser_line::parse_args(_line.as_str());
    let mut result: Vec<String> = Vec::new();
    for (sep, token) in args {
        if sep == "`" {
            let _args = parsers::parser_line::parse_line(token.as_str());
            let (_, _, output) = execute::run_pipeline(_args, "", "", false, false, false, true);
            if let Some(x) = output {
                match String::from_utf8(x.stdout) {
                    Ok(stdout) => {
                        let _txt = wrap_sep_string(sep.as_str(), stdout.trim());
                        result.push(_txt);
                    }
                    Err(_) => {
                        println_stderr!("cicada: from_utf8 error");
                        result.push(wrap_sep_string(sep.as_str(), token.as_str()));
                    }
                }
            } else {
                println_stderr!("cicada: command error");
                result.push(wrap_sep_string(sep.as_str(), token.as_str()));
            }
        } else if sep == "\"" || sep.is_empty() {
            let re;
            if let Ok(x) = Regex::new(r"^([^`]*)`([^`]+)`(.*)$") {
                re = x;
            } else {
                println_stderr!("cicada: re new error");
                return;
            }
            if !re.is_match(&token) {
                result.push(wrap_sep_string(sep.as_str(), token.as_str()));
                continue;
            }
            let mut _token = token.clone();
            let mut _item = String::new();
            let mut _head = String::new();
            let mut _output = String::new();
            let mut _tail = String::new();
            loop {
                if !re.is_match(&_token) {
                    if !_token.is_empty() {
                        _item = format!("{}{}", _item, _token);
                    }
                    break;
                }
                for cap in re.captures_iter(&_token) {
                    _head = cap[1].to_string();
                    _tail = cap[3].to_string();
                    let _args = parsers::parser_line::parse_line(&cap[2]);
                    let (_, _, output) =
                        execute::run_pipeline(_args, "", "", false, false, false, true);
                    if let Some(x) = output {
                        match String::from_utf8(x.stdout) {
                            Ok(stdout) => {
                                _output = stdout.trim().to_string();
                            }
                            Err(_) => {
                                println_stderr!("cicada: from_utf8 error");
                                result.push(wrap_sep_string(sep.as_str(), token.as_str()));
                                return;
                            }
                        }
                    } else {
                        println_stderr!("cicada: command error: {}", token);
                        result.push(wrap_sep_string(sep.as_str(), token.as_str()));
                        return;
                    }
                }
                _item = format!("{}{}{}", _item, _head, _output);
                if _tail.is_empty() {
                    break;
                }
                _token = _tail.clone();
            }
            result.push(wrap_sep_string(sep.as_str(), &_item));
        } else {
            result.push(wrap_sep_string(sep.as_str(), token.as_str()));
        }
    }
    *line = result.join(" ");
}

pub fn do_brace_expansion(line: &mut String) {
    let _line = line.clone();
    let args = parsers::parser_line::parse_args(_line.as_str());
    let mut result: Vec<String> = Vec::new();
    for (sep, token) in args {
        if sep.is_empty() && should_extend_brace(token.as_str()) {
            let mut _prefix = String::new();
            let mut _token = String::new();
            let mut _result = Vec::new();
            let mut only_tail_left = false;
            let mut start_sign_found = false;
            for c in token.chars() {
                if c == '{' {
                    start_sign_found = true;
                    continue;
                }
                if !start_sign_found {
                    _prefix.push(c);
                    continue;
                }
                if only_tail_left {
                    _token.push(c);
                    continue;
                }
                if c == '}' {
                    if !_token.is_empty() {
                        _result.push(_token);
                        _token = String::new();
                    }
                    only_tail_left = true;
                    continue;
                }
                if c == ',' {
                    if !_token.is_empty() {
                        _result.push(_token);
                        _token = String::new();
                    }
                } else {
                    _token.push(c);
                }
            }
            for item in &mut _result {
                *item = format!("{}{}{}", _prefix, item, _token);
            }
            result.push(wrap_sep_string(sep.as_str(), _result.join(" ").as_str()));
        } else {
            result.push(wrap_sep_string(sep.as_str(), token.as_str()));
        }
    }
    *line = result.join(" ");
}

fn needs_globbing(line: &str) -> bool {
    if is_arithmetic(line) {
        return false;
    }
    let re;
    if let Ok(x) = Regex::new(r"[\*]+") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
}

fn extend_glob(line: &mut String) {
    let _line = line.clone();
    // XXX: spliting needs to consider cases like `echo 'a * b'`
    let _tokens: Vec<&str> = _line.split(' ').collect();
    let mut result: Vec<String> = Vec::new();
    for item in &_tokens {
        if item.trim().starts_with('\'') || item.trim().starts_with('"') {
            result.push(item.to_string());
        } else {
            match glob::glob(item) {
                Ok(paths) => {
                    let mut is_empty = true;
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                let s = path.to_string_lossy();
                                if s.starts_with('.') {
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

pub fn pre_handle_cmd_line(sh: &shell::Shell, line: &mut String) {
    if needs_extend_home(line.as_str()) {
        extend_home(line);
    }
    if needs_globbing(line.as_str()) {
        extend_glob(line);
    }
    shell::extend_env(sh, line);
    do_command_substitution(line);
    do_brace_expansion(line);
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
    result
}

pub fn is_alias(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^ *alias +[a-zA-Z0-9_\.-]+=.*$") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
}

extern "C" {
    fn gethostname(name: *mut libc::c_char, size: libc::size_t) -> libc::c_int;
}

/// via: https://gist.github.com/conradkleinespel/6c8174aee28fa22bfe26
pub fn get_hostname() -> String {
    let len = 255;
    let mut buf = Vec::<u8>::with_capacity(len);

    let ptr = buf.as_mut_slice().as_mut_ptr();

    let err = unsafe { gethostname(ptr as *mut libc::c_char, len as libc::size_t) } as i32;

    match err {
        0 => {
            let real_len;
            let mut i = 0;
            loop {
                let byte = unsafe { *(((ptr as u64) + (i as u64)) as *const u8) };
                if byte == 0 {
                    real_len = i;
                    break;
                }

                i += 1;
            }
            unsafe { buf.set_len(real_len) }
            String::from_utf8_lossy(buf.as_slice()).into_owned()
        }
        _ => String::from("unknown"),
    }
}

pub fn is_arithmetic(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^[ 0-9\.\(\)\+\-\*/]+$") {
        re = x;
    } else {
        println!("regex error for arithmetic");
        return false;
    }
    re.is_match(line)
}

#[cfg(test)]
mod tests {
    use super::needs_extend_home;
    use super::needs_globbing;
    use super::is_alias;
    use super::do_brace_expansion;

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
    fn test_needs_globbing() {
        assert!(needs_globbing("ls *"));
        assert!(needs_globbing("ls  *.txt"));
        assert!(!needs_globbing("2 * 3"));
    }

    #[test]
    fn test_is_alias() {
        assert!(is_alias("alias ls='ls -lh'"));
    }

    #[test]
    fn test_do_brace_expansion() {
        let mut s = String::from("echo {foo,bar,baz}.txt");
        do_brace_expansion(&mut s);
        assert_eq!(s, "echo foo.txt bar.txt baz.txt");

        let mut s = String::from("echo foo.{txt,py}");
        do_brace_expansion(&mut s);
        assert_eq!(s, "echo foo.txt foo.py");

        let mut s = String::from("echo foo.{cpp,py}.txt");
        do_brace_expansion(&mut s);
        assert_eq!(s, "echo foo.cpp.txt foo.py.txt");
    }
}
