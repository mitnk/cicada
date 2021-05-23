use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::{IntoRawFd, RawFd};
use std::path::{Path, PathBuf};

use nix::unistd::pipe;

use chrono::prelude::{Local, Datelike, Timelike};
use libc;
use regex::Regex;

use crate::execute;
use crate::libs::re::re_contains;
use crate::parsers;
use crate::shell;

macro_rules! println_stderr {
    ($fmt:expr) => (
        match writeln!(&mut ::std::io::stderr(), $fmt) {
            Ok(_) => {}
            Err(e) => println!("write to stderr failed: {:?}", e)
        }
    );
    ($fmt:expr, $($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $fmt, $($arg)*) {
            Ok(_) => {}
            Err(e) => println!("write to stderr failed: {:?}", e)
        }
    );
}

pub fn clog(s: &str) {
    let file;
    if let Ok(x) = env::var("CICADA_LOG_FILE") {
        file = x;
    } else {
        return;
    }
    let mut cfile;
    match OpenOptions::new().append(true).create(true).open(&file) {
        Ok(x) => cfile = x,
        Err(e) => {
            println!("clog: open file {} failed: {:?}", &file, e);
            return;
        }
    }
    let pid = unsafe { libc::getpid() };
    let now = Local::now();
    let s = format!(
        "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}][{}] {}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        pid,
        s,
    );
    let s = if s.ends_with('\n') {
        s
    } else {
        format!("{}\n", s)
    };
    match cfile.write_all(s.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            println!("clog: write_all failed: {:?}", e);
            return;
        }
    }
}

macro_rules! log {
    ($fmt:expr) => (
        clog($fmt);
    );
    ($fmt:expr, $($arg:tt)*) => (
        clog(&format!($fmt, $($arg)*));
    );
}

pub fn get_user_name() -> String {
    match env::var("USER") {
        Ok(x) => {
            return x;
        }
        Err(e) => {
            log!("cicada: env USER error: {:?}", e);
        }
    }
    let cmd_result = execute::run("whoami");
    return cmd_result.stdout.trim().to_string();
}

pub fn get_user_home() -> String {
    match env::var("HOME") {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("cicada: env HOME error: {:?}", e);
            String::new()
        }
    }
}

pub fn get_config_dir() -> String {
    if let Ok(x) = env::var("XDG_CONFIG_HOME") {
        format!("{}/cicada", x)
    } else {
        let home = get_user_home();
        format!("{}/.config/cicada", home)
    }
}

pub fn get_user_completer_dir() -> String {
    let dir_config = get_config_dir();
    let dir_completers = format!("{}/completers", dir_config);
    if Path::new(&dir_completers).exists() {
        return dir_completers;
    }

    // fail back to $HOME/.cicada, will remove after 1.0 release
    let home = get_user_home();
    format!("{}/.cicada/completers", home)
}

pub fn unquote(s: &str) -> String {
    let args = parsers::parser_line::line_to_plain_tokens(s);
    if args.is_empty() {
        return String::new();
    }
    args[0].clone()
}

pub fn is_env(line: &str) -> bool {
    re_contains(line, r"^[a-zA-Z_][a-zA-Z0-9_]*=.*$")
}

// #[allow(clippy::trivial_regex)]
pub fn extend_bangbang(sh: &shell::Shell, line: &mut String) {
    if !re_contains(line, r"!!") {
        return;
    }
    if sh.previous_cmd.is_empty() {
        return;
    }

    let re;
    match Regex::new(r"!!") {
        Ok(x) => {
            re = x;
        }
        Err(e) => {
            println_stderr!("Regex new: {:?}", e);
            return;
        }
    }

    let mut replaced = false;
    let mut new_line = String::new();
    let tokens = parsers::parser_line::cmd_to_tokens(line);
    for (sep, token) in tokens {
        if !sep.is_empty() {
            new_line.push_str(&sep);
        }

        if re_contains(&token, r"!!") && sep != "'" {
            let line2 = token.clone();
            let result = re.replace_all(&line2, sh.previous_cmd.as_str());
            new_line.push_str(&result);
            replaced = true;
        } else {
            new_line.push_str(&token);
        }

        if !sep.is_empty() {
            new_line.push_str(&sep);
        }
        new_line.push(' ');
    }

    *line = new_line.trim_end().to_string();
    // print full line after extending
    if replaced {
        println!("{}", line);
    }
}

pub fn wrap_sep_string(sep: &str, s: &str) -> String {
    let mut _token = String::new();
    let mut met_subsep = false;
    // let set previous_subsep to any char except '`' or '"'
    let mut previous_subsep = 'N';
    for c in s.chars() {
        // handle cmds like: export DIR=`brew --prefix openssl`/include
        // or like: export foo="hello world"
        if sep.is_empty() && (c == '`' || c == '"') {
            if !met_subsep {
                met_subsep = true;
                previous_subsep = c;
            } else if c == previous_subsep {
                met_subsep = false;
                previous_subsep = 'N';
            }
        }
        if c.to_string() == sep {
            _token.push('\\');
        }
        if c == ' ' && sep.is_empty() && !met_subsep {
            _token.push('\\');
        }
        _token.push(c);
    }
    format!("{}{}{}", sep, _token, sep)
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
    if !re_contains(line, r"[0-9]+") {
        return false;
    }
    if !re_contains(line, r"\+|\-|\*|/|\^") {
        return false;
    }
    re_contains(line, r"^[ 0-9\.\(\)\+\-\*/\^]+[\.0-9 \)]$")
}

pub fn create_raw_fd_from_file(file_name: &str, append: bool) -> Result<i32, String> {
    let mut oos = OpenOptions::new();
    if append {
        oos.append(true);
    } else {
        oos.write(true);
        oos.truncate(true);
    }
    match oos.create(true).open(file_name) {
        Ok(x) => {
            let fd = x.into_raw_fd();
            Ok(fd)
        }
        Err(e) => Err(format!("failed to create fd from file: {:?}", e)),
    }
}

pub fn get_fd_from_file(file_name: &str) -> i32 {
    let path = Path::new(file_name);
    let display = path.display();
    let file = match File::open(&path) {
        Err(why) => {
            println_stderr!("cicada: could not open {}: {}", display, why);
            return 0;
        }
        Ok(file) => file,
    };
    file.into_raw_fd()
}

pub fn escape_path(path: &str) -> String {
    let re;
    match Regex::new(r##"(?P<c>[!\(\)<>,\?\]\[\{\} \\'"`*\^#|$&;])"##) {
        Ok(x) => {
            re = x;
        }
        Err(_) => {
            return path.to_string();
        }
    }
    return re.replace_all(path, "\\$c").to_string();
}

pub fn get_current_dir() -> String {
    let mut current_dir = PathBuf::new();
    match env::current_dir() {
        Ok(x) => current_dir = x,
        Err(e) => {
            println_stderr!("env current_dir() failed: {}", e);
        }
    }
    let mut str_current_dir = "";
    match current_dir.to_str() {
        Some(x) => str_current_dir = x,
        None => {
            println_stderr!("current_dir to str failed.");
        }
    }
    str_current_dir.to_string()
}

pub fn split_into_fields(sh: &shell::Shell, line: &str, envs: &HashMap<String, String>) -> Vec<String> {
    let ifs_chars;
    if envs.contains_key("IFS") {
        ifs_chars = envs[&"IFS".to_string()].chars().collect();
    } else if let Some(x) = sh.get_env("IFS") {
        ifs_chars = x.chars().collect();
    } else if let Ok(x) = env::var("IFS") {
        ifs_chars = x.chars().collect();
    } else {
        ifs_chars = vec![];
    }

    if ifs_chars.is_empty() {
        return line.split(&[' ', '\t', '\n'][..]).map(|x| x.to_string()).collect();
    } else {
        return line.split(&ifs_chars[..]).map(|x| x.to_string()).collect();
    }
}

pub fn create_fds() -> Option<(RawFd, RawFd)> {
    match pipe() {
        Ok(x) => {
            return Some(x);
        }
        Err(e) => {
            println_stderr!("cicada: pipe error: {:?}", e);
            return None;
        }
    }
}

pub fn is_builtin(s: &str) -> bool {
    let builtins = vec![
        "alias", "bg", "cd", "cinfo", "exec", "exit", "export", "fg",
        "history", "jobs", "read", "source", "ulimit", "unalias", "vox",
        "minfd", "set",
    ];
    builtins.contains(&s)
}

#[cfg(test)]
mod tests {
    use super::escape_path;
    use super::extend_bangbang;
    use crate::shell;

    #[test]
    fn test_extend_bangbang() {
        let mut sh = shell::Shell::new();
        sh.previous_cmd = "foo".to_string();

        let mut line = "echo !!".to_string();
        extend_bangbang(&sh, &mut line);
        assert_eq!(line, "echo foo");

        line = "echo \"!!\"".to_string();
        extend_bangbang(&sh, &mut line);
        assert_eq!(line, "echo \"foo\"");

        line = "echo '!!'".to_string();
        extend_bangbang(&sh, &mut line);
        assert_eq!(line, "echo '!!'");

        line = "echo '!!' && echo !!".to_string();
        extend_bangbang(&sh, &mut line);
        assert_eq!(line, "echo '!!' && echo foo");
    }

    #[test]
    fn test_escape_path() {
        assert_eq!(
            escape_path("a b!c\"d\'#$&e(f)g*h,i;j<k>l?m\\n[]o`p{}q|^z.txt"),
            "a\\ b\\!c\\\"d\\\'\\#\\$\\&e\\(f\\)g\\*h\\,i\\;j\\<k\\>l\\?m\\\\n\\[\\]o\\`p\\{\\}q\\|\\^z.txt",
        );
    }
}
