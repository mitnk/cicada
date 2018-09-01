use errno::errno;
use libc;
use std::collections::HashMap;
use std::env;
use std::mem;

use regex::Regex;

use parsers;
use tools::{self, clog};

#[derive(Debug, Clone)]
pub struct Shell {
    pub alias: HashMap<String, String>,
    pub cmd: String,
    pub previous_dir: String,
    pub previous_cmd: String,
    pub previous_status: i32,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            alias: HashMap::new(),
            cmd: String::new(),
            previous_dir: String::new(),
            previous_cmd: String::new(),
            previous_status: 0,
        }
    }

    pub fn add_alias(&mut self, name: &str, value: &str) {
        self.alias.insert(name.to_string(), value.to_string());
    }

    pub fn get_alias_content(&self, name: &str) -> Option<String> {
        let mut result;
        match self.alias.get(name) {
            Some(x) => {
                result = x.to_string();
            }
            None => {
                result = String::new();
            }
        }
        tools::pre_handle_cmd_line(self, &mut result);
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

pub unsafe fn give_terminal_to(gid: i32) -> bool {
    let mut mask: libc::sigset_t = mem::zeroed();
    let mut old_mask: libc::sigset_t = mem::zeroed();

    libc::sigemptyset(&mut mask);
    libc::sigaddset(&mut mask, libc::SIGTSTP);
    libc::sigaddset(&mut mask, libc::SIGTTIN);
    libc::sigaddset(&mut mask, libc::SIGTTOU);
    libc::sigaddset(&mut mask, libc::SIGCHLD);

    let rcode = libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut old_mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    let rcode = libc::tcsetpgrp(1, gid);
    let given;
    if rcode == -1 {
        given = false;
        let e = errno();
        let code = e.0;
        log!("Error {}: {}", code, e);
    } else {
        given = true;
    }
    let rcode = libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, &mut mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    given
}

pub fn extend_env_blindly(sh: &Shell, token: &str) -> String {
    let re;
    if let Ok(x) = Regex::new(r"([^\$]*)\$\{?([A-Za-z0-9\?\$_]+)\}?(.*)") {
        re = x;
    } else {
        println!("cicada: re new error");
        return String::new();
    }
    if !re.is_match(token) {
        return token.to_string();
    }
    let mut result = String::new();
    let mut _token = token.to_string();
    let mut _head = String::new();
    let mut _output = String::new();
    let mut _tail = String::new();
    loop {
        if !re.is_match(&_token) {
            if !_token.is_empty() {
                result.push_str(&_token);
            }
            break;
        }
        for cap in re.captures_iter(&_token) {
            _head = cap[1].to_string();
            _tail = cap[3].to_string();
            let _key = cap[2].to_string();
            if _key == "?" {
                result.push_str(format!("{}{}", _head, sh.previous_status).as_str());
            } else if _key == "$" {
                unsafe {
                    let val = libc::getpid();
                    result.push_str(format!("{}{}", _head, val).as_str());
                }
            } else if let Ok(val) = env::var(_key) {
                result.push_str(format!("{}{}", _head, val).as_str());
            } else {
                result.push_str(&_head);
            }
        }
        if _tail.is_empty() {
            break;
        }
        _token = _tail.clone();
    }
    result
}

pub fn extend_env(sh: &Shell, line: &mut String) {
    let mut result: Vec<String> = Vec::new();
    let _line = line.clone();
    let args = parsers::parser_line::cmd_to_tokens(_line.as_str());
    for (sep, token) in args {
        if sep == "`" || sep == "'" {
            result.push(tools::wrap_sep_string(&sep, &token));
        } else {
            let _token = extend_env_blindly(sh, &token);
            result.push(tools::wrap_sep_string(&sep, &_token));
        }
    }
    *line = result.join(" ");
}

#[cfg(test)]
mod tests {
    use super::extend_env;
    use super::Shell;

    #[test]
    fn test_extend_env() {
        let sh = Shell::new();
        let mut s = String::from("echo '$PATH'");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo '$PATH'");

        let mut s = String::from("echo a\\ b xy");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo a\\ b xy");

        let mut s = String::from("echo 'hi $PATH'");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo 'hi $PATH'");

        let mut s = String::from("echo \'\\\'");
        extend_env(&sh, &mut s);
        assert_eq!(s, "echo \'\\\'");

        let mut s = String::from("export DIR=`brew --prefix openssl`/include");
        extend_env(&sh, &mut s);
        assert_eq!(s, "export DIR=`brew --prefix openssl`/include");

        let mut s = String::from("export FOO=\"`date` and `go version`\"");
        extend_env(&sh, &mut s);
        assert_eq!(s, "export \"FOO=`date` and `go version`\"");

        let mut s = String::from("foo is XX${CICADA_NOT_EXIST}XX");
        extend_env(&sh, &mut s);
        assert_eq!(s, "foo is XXXX");

        let mut s = String::from("foo is $CICADA_NOT_EXIST_1 and bar is $CICADA_NOT_EXIST_2.");
        extend_env(&sh, &mut s);
        assert_eq!(s, "foo is  and bar is .");
    }
}
