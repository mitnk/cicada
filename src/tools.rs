use std::collections::HashSet;
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::Stdio;
use time;

use regex::Regex;

use execute;
use libc;
use parsers;
use shell;

#[derive(Clone, Debug, Default)]
pub struct CommandResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn new() -> CommandResult {
        CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

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
    let now = time::now();
    let s = format!(
        "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}][{}]{}",
        now.tm_year + 1900,
        now.tm_mon + 1,
        now.tm_mday,
        now.tm_hour,
        now.tm_min,
        now.tm_sec,
        pid,
        s,
    );
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
        clog(concat!($fmt, "\n"));
    );
    ($fmt:expr, $($arg:tt)*) => (
        clog(format!(concat!($fmt, "\n"), $($arg)*).as_str());
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
    match execute::run("whoami") {
        Ok(x) => {
            return x.stdout.trim().to_string();
        }
        Err(e) => {
            log!("cicada: run whoami error: {}", e);
        }
    }
    String::from("NOUSER")
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
    let args = parsers::parser_line::line_to_plain_tokens(s);
    if args.is_empty() {
        return String::new();
    }
    args[0].clone()
}

pub fn is_export_env(line: &str) -> bool {
    re_contains(line, r"^ *export +[a-zA-Z0-9_]+=.*$")
}

pub fn is_env(line: &str) -> bool {
    re_contains(line, r"^[a-zA-Z0-9_]+=.*$")
}

pub fn should_extend_brace(line: &str) -> bool {
    re_contains(line, r#"\{[^ "']+,[^ "']+,?[^ "']*\}"#)
}

#[allow(trivial_regex)]
pub fn extend_bandband(sh: &shell::Shell, line: &mut String) {
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

    *line = new_line.trim_right().to_string();
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

pub fn is_alias(line: &str) -> bool {
    re_contains(line, r"^ *alias +[a-zA-Z0-9_\.-]+=.*$")
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
    re_contains(line, r"^[ 0-9\.\(\)\+\-\*/]+$")
}

pub fn re_contains(line: &str, ptn: &str) -> bool {
    let re;
    match Regex::new(ptn) {
        Ok(x) => {
            re = x;
        }
        Err(e) => {
            println!("Regex new: {:?}", e);
            return false;
        }
    }
    re.is_match(line)
}

pub fn extend_alias(sh: &shell::Shell, line: &str) -> String {
    let cmds = parsers::parser_line::line_to_cmds(line);
    let mut seps_cmd: HashSet<&str> = HashSet::new();
    seps_cmd.insert(";");
    seps_cmd.insert("&&");
    seps_cmd.insert("||");

    let mut result = String::new();
    for (_, cmd) in cmds.iter().enumerate() {
        if seps_cmd.contains(cmd.as_str()) {
            result.push(' ');
            result.push_str(cmd);
            result.push(' ');
            continue;
        }

        let tokens = parsers::parser_line::cmd_to_tokens(cmd);
        let mut is_cmd = false;
        for (i, token) in tokens.iter().enumerate() {
            let sep = &token.0;
            let arg = &token.1;

            if !sep.is_empty() {
                is_cmd = false;
                result.push(' ');
                result.push_str(&wrap_sep_string(&sep, &arg));
                continue;
            }

            if i == 0 {
                is_cmd = true;
            } else if arg == "|" {
                result.push(' ');
                result.push_str(&wrap_sep_string(&sep, &arg));
                is_cmd = true;
                continue;
            }
            if !is_cmd {
                result.push(' ');
                result.push_str(&wrap_sep_string(&sep, &arg));
                continue;
            }

            let extended;
            match sh.get_alias_content(arg) {
                Some(_extended) => {
                    extended = _extended;
                }
                None => {
                    extended = arg.clone();
                }
            }
            if i > 0 {
                result.push(' ');
            }
            result.push_str(&extended);
            is_cmd = false;
        }
    }
    result
}

pub fn create_fd_from_file(file_name: &str, append: bool) -> Result<Stdio, String> {
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
            let file_out = unsafe { Stdio::from_raw_fd(fd) };
            Ok(file_out)
        }
        Err(e) => Err(format!("failed to create fd from file: {:?}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::extend_alias;
    use super::extend_bandband;
    use super::is_alias;
    use shell;

    #[test]
    fn test_is_alias() {
        assert!(is_alias("alias ls='ls -lh'"));
    }

    #[test]
    fn test_extend_alias() {
        let mut sh = shell::Shell::new();
        sh.add_alias("ls", "ls -G");
        sh.add_alias("wc", "wc -l");
        sh.add_alias("grep", "grep -I --color=auto --exclude-dir=.git");
        sh.add_alias("tx", "tmux");

        assert_eq!(extend_alias(&sh, "echo"), "echo");
        assert_eq!(extend_alias(&sh, "echo a\\ b xy"), "echo a\\ b xy");

        assert_eq!(extend_alias(&sh, "ls"), "ls -G");
        assert_eq!(extend_alias(&sh, "ls a\\ b xy"), "ls -G a\\ b xy");

        assert_eq!(extend_alias(&sh, "ls -lh"), "ls -G -lh");
        assert_eq!(extend_alias(&sh, "ls | wc"), "ls -G | wc -l");
        assert_eq!(
            extend_alias(&sh, "ps ax | grep foo"),
            "ps ax | grep -I --color=auto --exclude-dir=.git foo"
        );
        assert_eq!(extend_alias(&sh, "ls | wc | cat"), "ls -G | wc -l | cat");
        assert_eq!(extend_alias(&sh, "echo foo | wc"), "echo foo | wc -l");
        assert_eq!(
            extend_alias(&sh, "echo foo | cat | wc"),
            "echo foo | cat | wc -l"
        );
        assert_eq!(
            extend_alias(&sh, "echo foo | wc | cat"),
            "echo foo | wc -l | cat"
        );
        assert_eq!(extend_alias(&sh, "ls || wc"), "ls -G || wc -l");
        assert_eq!(extend_alias(&sh, "ls && wc"), "ls -G && wc -l");
        assert_eq!(extend_alias(&sh, "ls&&wc"), "ls -G && wc -l");
        assert_eq!(extend_alias(&sh, "ls ; wc"), "ls -G ; wc -l");
        assert_eq!(extend_alias(&sh, "ls; wc"), "ls -G ; wc -l");
        assert_eq!(extend_alias(&sh, "ls;wc"), "ls -G ; wc -l");
        assert_eq!(
            extend_alias(&sh, "ls&&wc; foo || bar"),
            "ls -G && wc -l ; foo || bar"
        );
        assert_eq!(extend_alias(&sh, "echo 'ls | wc'"), "echo 'ls | wc'");
        assert_eq!(extend_alias(&sh, "echo \"ls | wc\""), "echo \"ls | wc\"");
        assert_eq!(extend_alias(&sh, "echo `ls | wc`"), "echo `ls | wc`");

        assert_eq!(extend_alias(&sh, "tx ls"), "tmux ls");
        assert_eq!(
            extend_alias(&sh, "awk -F \"[ ,.\\\"]+\""),
            "awk -F \"[ ,.\\\"]+\""
        );
        assert_eq!(extend_alias(&sh, "ls a\\.b"), "ls -G a.b");
    }

    #[test]
    fn test_extend_bandband() {
        let mut sh = shell::Shell::new();
        sh.previous_cmd = "foo".to_string();

        let mut line = "echo !!".to_string();
        extend_bandband(&sh, &mut line);
        assert_eq!(line, "echo foo");

        line = "echo \"!!\"".to_string();
        extend_bandband(&sh, &mut line);
        assert_eq!(line, "echo \"foo\"");

        line = "echo '!!'".to_string();
        extend_bandband(&sh, &mut line);
        assert_eq!(line, "echo '!!'");

        line = "echo '!!' && echo !!".to_string();
        extend_bandband(&sh, &mut line);
        assert_eq!(line, "echo '!!' && echo foo");
    }
}
