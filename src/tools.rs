use std;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use glob;
use regex::Regex;
use shellexpand;

use libc;
use parsers;
use shlex;
use tools;

pub fn rlog(s: String) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/cicada-debug.log")
        .expect("rlog: open /tmp/cicada-debug.log faild");
    let pid = unsafe { libc::getpid() };
    let s = format!("[{}] {}", pid, s);
    file.write_all(s.as_bytes()).expect("rlog: write_all failed");
}

pub fn get_user_home() -> String {
    let home = env::var("HOME").expect("cicada: env HOME error");
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

pub fn extend_env(line: &mut String) {
    let mut result: Vec<String> = Vec::new();
    let _line = line.clone();
    let args = parsers::parser_line::parser_args(_line.as_str());

    for (sep, token) in args {
        if sep == "`" {
            tools::println_stderr("cicada: does not support \"`\" yet");
            result.push(format!("{}{}{}", sep, token, sep));
        } else if sep == "'" {
            result.push(format!("{}{}{}", sep, token, sep));
        } else {
            match shellexpand::env(token.as_str()) {
                Ok(x) => {
                    result.push(format!("{}{}{}", sep, x.into_owned(), sep));
                }
                Err(e) => {
                    println!("cicada: shellexpand error: {:?}", e);
                    result.push(format!("{}{}{}", sep, token, sep));
                }
            }
        }
    }


    /*
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
    */

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
    return re.is_match(line);
}

fn extend_glob(line: &mut String) {
    let _line = line.clone();
    // XXX: spliting needs to consider cases like `echo 'a * b'`
    let _tokens: Vec<&str> = _line.split(" ").collect();
    let mut result: Vec<String> = Vec::new();
    for item in &_tokens {
        if item.trim().starts_with("'") || item.trim().starts_with("\"") {
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

extern {
    fn gethostname(name: *mut libc::c_char, size: libc::size_t) -> libc::c_int;
}

/// via: https://gist.github.com/conradkleinespel/6c8174aee28fa22bfe26
pub fn get_hostname() -> String {
    let len = 255;
    let mut buf = Vec::<u8>::with_capacity(len);

    let ptr = buf.as_mut_slice().as_mut_ptr();

    let err = unsafe {
        gethostname(ptr as *mut libc::c_char, len as libc::size_t)
    } as i32;

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
        },
        _ => {
            String::from("unknown")
        }
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
    return re.is_match(line);
}

pub fn println_stderr(msg: &str) {
    writeln!(&mut std::io::stderr(), "{}", msg).expect("write to stderr failed");
}

#[cfg(test)]
mod tests {
    use super::needs_extend_home;
    use super::needs_globbing;
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
    fn test_extend_env() {
        let mut s = String::from("echo '$PATH'");
        extend_env(&mut s);
        assert_eq!(s, "echo '$PATH'");

        let mut s = String::from("echo 'hi $PATH'");
        extend_env(&mut s);
        assert_eq!(s, "echo 'hi $PATH'");
    }
}
