use std::env;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Stdio;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use time;

use glob;
use regex::Regex;

use libc;
use parsers;
use execute;
use shell;
use libs;

#[derive(Clone, Debug)]
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
    let s =
        format!(
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

pub fn is_env(line: &str) -> bool {
    re_contains(line, r"^ *export +[a-zA-Z0-9_]+=.*$")
}

fn should_extend_brace(line: &str) -> bool {
    re_contains(line, r"\{.*,.*\}")
}

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

pub fn extend_home(s: &mut String) {
    if !needs_extend_home(s) {
        return;
    }
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
    re_contains(line, r"( +~ +)|( +~/)|(^ *~/)|( +~ *$)")
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

fn do_command_substitution(line: &mut String) {
    do_command_substitution_for_dot(line);
    do_command_substitution_for_dollar(line);
}

fn should_do_brace_command_extension(line: &str) -> bool {
    re_contains(line, r"\$\([^\)]+\)")
}

fn do_command_substitution_for_dollar(line: &mut String) {
    loop {
        if !should_do_brace_command_extension(&line) {
            break;
        }
        let ptn_cmd = r"\$\(([^\(]+)\)";
        let cmd;
        match libs::re::find_first_group(ptn_cmd, &line) {
            Some(x) => {
                cmd = x;
            }
            None => {
                println_stderr!("cicada: no first group");
                return;
            }
        }

        let _args = parsers::parser_line::cmd_to_tokens(&cmd);
        let (_, _, output) = execute::run_pipeline(_args, "", false, false, true, None);
        let _stdout;
        let output_txt;
        if let Some(x) = output {
            match String::from_utf8(x.stdout) {
                Ok(stdout) => {
                    _stdout = stdout.clone();
                    output_txt = _stdout.trim();
                }
                Err(_) => {
                    println_stderr!("cicada: from_utf8 error");
                    return;
                }
            }
        } else {
            println_stderr!("cicada: command error");
            return;
        }

        let ptn = r"(?P<head>[^\$]*)\$\([^\(]+\)(?P<tail>.*)";
        let re;
        if let Ok(x) = Regex::new(ptn) {
            re = x;
        } else {
            return;
        }

        let to = format!("${{head}}{}${{tail}}", output_txt);
        let line_ = line.clone();
        let result = re.replace(&line_, to.as_str());
        *line = result.to_string();
    }
}

fn do_command_substitution_for_dot(line: &mut String) {
    let tokens = parsers::parser_line::cmd_to_tokens(&line);
    let mut result: Vec<String> = Vec::new();
    for (sep, token) in tokens {
        if sep == "`" {
            let _args = parsers::parser_line::cmd_to_tokens(token.as_str());
            let (_, _, output) =
                execute::run_pipeline(_args, "", false, false, true, None);
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
                    let _args = parsers::parser_line::cmd_to_tokens(&cap[2]);
                    let (_, _, output) =
                        execute::run_pipeline(_args, "", false, false, true, None);
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
    let args = parsers::parser_line::cmd_to_tokens(_line.as_str());
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
    if let Ok(x) = Regex::new(r"\*+") {
        re = x;
    } else {
        return false;
    }

    let tokens = parsers::parser_line::cmd_to_tokens(line);
    for (sep, token) in tokens {
        if !sep.is_empty() {
            continue;
        }
        if re.is_match(&token) {
            return true;
        }
    }
    false
}

fn extend_glob(line: &mut String) {
    if !needs_globbing(&line) {
        return;
    }
    let _line = line.clone();
    // XXX: spliting needs to consider cases like `echo 'a * b'`
    let _tokens: Vec<&str> = _line.split(' ').collect();
    let mut result: Vec<String> = Vec::new();
    for item in &_tokens {
        if !item.contains('*') {
            result.push(item.to_string());
        } else if item.trim().starts_with('\'') || item.trim().starts_with('"') {
            result.push(item.to_string());
        } else {
            match glob::glob(item) {
                Ok(paths) => {
                    let mut is_empty = true;
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                let s = path.to_string_lossy();
                                if !item.starts_with('.') && s.starts_with('.') &&
                                    !s.contains('/')
                                {
                                    // skip hidden files, you may need to
                                    // type `ls .*rc` instead of `ls *rc`
                                    continue;
                                }
                                result.push(s.into_owned());
                                is_empty = false;
                            }
                            Err(e) => {
                                log!("glob error: {:?}", e);
                            }
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
    extend_home(line);
    do_brace_expansion(line);
    extend_glob(line);
    shell::extend_env(sh, line);
    do_command_substitution(line);
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
                result.push_str(&sep);
                let replace_to = format!("\\{}", sep);
                result.push_str(&arg.replace(sep, &replace_to));
                result.push_str(&sep);
                continue;
            }

            if i == 0 {
                is_cmd = true;
            } else if arg == "|" {
                result.push(' ');
                result.push_str(&arg);
                is_cmd = true;
                continue;
            }
            if !is_cmd {
                result.push(' ');
                result.push_str(&arg);
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

pub fn remove_envs_from_line(line: &str, envs: &mut HashMap<String, String>) -> String {
    let mut result = line.to_string();
    loop {
        match libs::re::find_first_group(r"^( *[a-zA-Z][a-zA-Z0-9_]+=[^ ]*)", &result) {
            Some(x) => {
                let v: Vec<&str> = x.split('=').collect();
                if v.len() != 2 {
                    println_stderr!("remove envs error");
                    break;
                }
                envs.insert(v[0].to_string(), v[1].to_string());

                result = result.trim().replace(&x, "").trim().to_owned();
            }
            None => {
                break;
            }
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
        Err(e) => {
            Err(format!("failed to create fd from file: {:?}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use shell;
    use std::collections::HashMap;
    use super::needs_extend_home;
    use super::needs_globbing;
    use super::is_alias;
    use super::do_brace_expansion;
    use super::do_command_substitution;
    use super::should_do_brace_command_extension;
    use super::extend_alias;
    use super::remove_envs_from_line;
    use super::extend_bandband;

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
        assert!(needs_globbing("grep -i 'desc' /etc/*release*"));
        assert!(!needs_globbing("2 * 3"));
        assert!(!needs_globbing("ls '*.md'"));
        assert!(!needs_globbing("ls 'a * b'"));
        assert!(!needs_globbing("ls foo"));
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

    #[test]
    fn test_do_command_substitution() {
        let mut s = String::from("ls `echo yoo`");
        do_command_substitution(&mut s);
        assert_eq!(s, "ls `yoo`"); // may need to change to "ls yoo"

        s = String::from("ls `echo yoo` foo `echo hoo`");
        do_command_substitution(&mut s);
        assert_eq!(s, "ls `yoo` foo `hoo`");

        s = String::from("ls $(echo yoo)");
        do_command_substitution(&mut s);
        assert_eq!(s, "ls yoo");

        s = String::from("ls $(echo yoo) foo $(echo hoo)");
        do_command_substitution(&mut s);
        assert_eq!(s, "ls yoo foo hoo");
    }

    #[test]
    fn test_should_do_brace_command_extension() {
        assert!(!should_do_brace_command_extension("ls $HOME"));
        assert!(!should_do_brace_command_extension("echo $[pwd]"));
        assert!(should_do_brace_command_extension("echo $(pwd)"));
        assert!(should_do_brace_command_extension("echo $(pwd) foo"));
        assert!(should_do_brace_command_extension("echo $(foo bar)"));
        assert!(should_do_brace_command_extension("echo $(echo foo)"));
        assert!(should_do_brace_command_extension("$(pwd) foo"));
    }

    #[test]
    fn test_extend_alias() {
        let mut sh = shell::Shell::new();
        sh.add_alias("ls", "ls -G");
        sh.add_alias("wc", "wc -l");
        sh.add_alias("grep", "grep -I --color=auto --exclude-dir=.git");
        sh.add_alias("tx", "tmux");

        assert_eq!(extend_alias(&sh, "ls"), "ls -G");
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
    fn test_remove_envs_from_line() {
        let line = "foo=1 echo hi";
        let mut envs = HashMap::new();
        assert_eq!(remove_envs_from_line(line, &mut envs), "echo hi");
        assert_eq!(envs["foo"], "1");

        let line = "foo=1 bar=2 echo hi";
        let mut envs = HashMap::new();
        assert_eq!(remove_envs_from_line(line, &mut envs), "echo hi");
        assert_eq!(envs["foo"], "1");
        assert_eq!(envs["bar"], "2");

        let line = "foo=1 bar=2 baz=3 bbq=4 cicada -c 'abc'";
        let mut envs = HashMap::new();
        assert_eq!(remove_envs_from_line(line, &mut envs), "cicada -c 'abc'");
        assert_eq!(envs["foo"], "1");
        assert_eq!(envs["bar"], "2");
        assert_eq!(envs["baz"], "3");
        assert_eq!(envs["bbq"], "4");
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
