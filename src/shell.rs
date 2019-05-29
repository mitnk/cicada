use errno::errno;
use libc;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::mem;

use glob;
use regex::Regex;

use crate::core;
use crate::libs;
use crate::parsers;
use crate::tools::{self, clog};
use crate::types;

#[derive(Debug, Clone)]
pub struct Shell {
    pub jobs: HashMap<i32, types::Job>,
    pub alias: HashMap<String, String>,
    pub envs: HashMap<String, String>,
    pub funcs: HashMap<String, String>,
    pub cmd: String,
    pub previous_dir: String,
    pub previous_cmd: String,
    pub previous_status: i32,
    pub is_login: bool,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            jobs: HashMap::new(),
            alias: HashMap::new(),
            envs: HashMap::new(),
            funcs: HashMap::new(),
            cmd: String::new(),
            previous_dir: String::new(),
            previous_cmd: String::new(),
            previous_status: 0,
            is_login: false,
        }
    }

    pub fn insert_job(&mut self, gid: i32, pid: i32, cmd: &str, status: &str, bg: bool) {
        let mut i = 1;
        loop {
            let mut indexed_job_missing = false;
            if let Some(x) = self.jobs.get_mut(&i) {
                if x.gid == gid {
                    x.pids.push(pid);
                    return;
                }
            } else {
                indexed_job_missing = true;
            }

            let mut _cmd = cmd.to_string();
            if bg && !_cmd.ends_with('&') {
                _cmd.push_str(" &");
            }
            if indexed_job_missing {
                self.jobs.insert(
                    i,
                    types::Job {
                        cmd: _cmd.to_string(),
                        id: i,
                        gid: gid,
                        pids: vec![pid],
                        status: status.to_string(),
                        report: bg,
                    },
                );
                return;
            }
            i += 1;
        }
    }

    pub fn get_job_by_id(&self, job_id: i32) -> Option<&types::Job> {
        self.jobs.get(&job_id)
    }

    pub fn get_job_by_gid(&self, gid: i32) -> Option<&types::Job> {
        if self.jobs.is_empty() {
            return None;
        }

        let mut i = 1;
        loop {
            if let Some(x) = self.jobs.get(&i) {
                if x.gid == gid {
                    return Some(&x);
                }
            }

            i += 1;
            if i >= 65535 {
                break;
            }
        }
        None
    }

    pub fn mark_job_as_running(&mut self, gid: i32, bg: bool) {
        if self.jobs.is_empty() {
            return;
        }

        let mut i = 1;
        loop {
            if let Some(x) = self.jobs.get_mut(&i) {
                if x.gid == gid {
                    x.status = "Running".to_string();
                    x.report = bg;
                    if bg && !x.cmd.ends_with(" &") {
                        x.cmd = format!("{} &", x.cmd);
                    }
                    return;
                }
            }

            i += 1;
            if i >= 65535 {
                break;
            }
        }
    }

    pub fn mark_job_as_stopped(&mut self, gid: i32) {
        if self.jobs.is_empty() {
            return;
        }

        let mut i = 1;
        loop {
            if let Some(x) = self.jobs.get_mut(&i) {
                if x.gid == gid {
                    x.status = "Stopped".to_string();
                    return;
                }
            }

            i += 1;
            if i >= 65535 {
                break;
            }
        }
    }

    pub fn remove_pid_from_job(&mut self, gid: i32, pid: i32) -> Option<types::Job> {
        if self.jobs.is_empty() {
            return None;
        }

        let mut empty_pids = false;
        let mut i = 1;
        loop {
            if let Some(x) = self.jobs.get_mut(&i) {
                if x.gid == gid {
                    if let Ok(i_pid) = x.pids.binary_search(&pid) {
                        x.pids.remove(i_pid);
                    }
                    empty_pids = x.pids.is_empty();
                    break;
                }
            }

            i += 1;
            if i >= 65535 {
                break;
            }
        }

        if empty_pids {
            return self.jobs.remove(&i);
        }
        None
    }

    pub fn set_env(&mut self, name: &str, value: &str) {
        if env::var(name).is_ok() {
            env::set_var(name, value);
        } else {
            self.envs.insert(name.to_string(), value.to_string());
        }
    }

    pub fn get_env(&self, name: &str) -> Option<String> {
        match self.envs.get(name) {
            Some(x) => Some(x.to_string()),
            None => None,
        }
    }

    pub fn set_func(&mut self, name: &str, value: &str) {
        self.funcs.insert(name.to_string(), value.to_string());
    }

    pub fn get_func(&self, name: &str) -> Option<String> {
        match self.funcs.get(name) {
            Some(x) => Some(x.to_string()),
            None => None,
        }
    }

    pub fn get_alias_list(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for (name, value) in &self.alias {
            result.push((name.clone(), value.clone()));
        }
        result
    }

    pub fn add_alias(&mut self, name: &str, value: &str) {
        self.alias.insert(name.to_string(), value.to_string());
    }

    pub fn is_alias(&self, name: &str) -> bool {
        self.alias.contains_key(name)
    }

    pub fn remove_alias(&mut self, name: &str) -> bool {
        let opt = self.alias.remove(name);
        return opt.is_some();
    }

    pub fn get_alias_content(&self, name: &str) -> Option<String> {
        let result;
        match self.alias.get(name) {
            Some(x) => {
                result = x.to_string();
            }
            None => {
                result = String::new();
            }
        }
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
        log!("error in give_terminal_to() {}: {}", code, e);
    } else {
        given = true;
    }
    let rcode = libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, &mut mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    given
}

fn needs_globbing(line: &str) -> bool {
    if tools::is_arithmetic(line) {
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

pub fn expand_glob(tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !needs_globbing(text) {
            idx += 1;
            continue;
        }

        let mut result: Vec<String> = Vec::new();
        let item = text.as_str();

        if !item.contains('*') || item.trim().starts_with('\'') || item.trim().starts_with('"') {
            result.push(item.to_string());
        } else {
            let _basename = libs::path::basename(item);
            let show_hidden = _basename.starts_with(".*");

            match glob::glob(item) {
                Ok(paths) => {
                    let mut is_empty = true;
                    for entry in paths {
                        match entry {
                            Ok(path) => {
                                let file_path = path.to_string_lossy();
                                let _basename = libs::path::basename(&file_path);
                                if _basename == ".." || _basename == "." {
                                    continue;
                                }
                                if _basename.starts_with('.') && !show_hidden {
                                    // skip hidden files, you may need to
                                    // type `ls .*rc` instead of `ls *rc`
                                    continue;
                                }
                                result.push(file_path.to_string());
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

        buff.push((idx, result));
        idx += 1;
    }

    for (i, result) in buff.iter().rev() {
        tokens.remove(*i as usize);
        for (j, token) in result.iter().enumerate() {
            let sep = if token.contains(' ') { "\"" } else { "" };
            tokens.insert((*i + j) as usize, (sep.to_string(), token.clone()));
        }
    }
}

pub fn extend_env_blindly(sh: &Shell, token: &str) -> String {
    let re;
    if let Ok(x) = Regex::new(r"^(.*?)\$\{?([A-Za-z0-9_]+|\$|\?)\}?(.*)$") {
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

        let cap_results = re.captures_iter(&_token);

        for cap in cap_results {
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
            } else if let Ok(val) = env::var(&_key) {
                result.push_str(format!("{}{}", _head, val).as_str());
            } else if let Some(val) = sh.get_env(&_key) {
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

fn need_expand_brace(line: &str) -> bool {
    libs::re::re_contains(line, r#"\{[^ "']*,[^ "']*,?[^ "']*\}"#)
}

fn brace_getitem(s: &str, depth: i32) -> (Vec<String>, String) {
    let mut out: Vec<String> = vec![String::new()];
    let mut ss = s.to_string();
    let mut tmp;
    while ss.len() > 0 {
        let c = ss.chars().next().unwrap();
        if depth > 0 && (c == ',' || c == '}') {
            return (out, ss);
        }
        if c == '{' {
            let mut sss = ss.clone();
            sss.remove(0);
            let result_groups = brace_getgroup(&sss, depth + 1);
            if let Some((out_group, s_group)) = result_groups {
                let mut tmp_out = Vec::new();
                for x in out.iter() {
                    for y in out_group.iter() {
                        let item = format!("{}{}", x, y);
                        tmp_out.push(item);
                    }
                }
                out = tmp_out;
                ss = s_group.clone();
                continue;
            }
        }
        // FIXME: here we mean more than one char.
        if c == '\\' && ss.len() > 1 {
            ss.remove(0);
            let c = ss.chars().next().unwrap();
            tmp = format!("\\{}", c);
        } else {
            tmp = c.to_string();
        }
        let mut result = Vec::new();
        for x in out.iter() {
            let item = format!("{}{}", x, tmp);
            result.push(item);
        }
        out = result;
        ss.remove(0);
    }
    (out, ss)
}

fn brace_getgroup(s: &str, depth: i32) -> Option<(Vec<String>, String)> {
    let mut out: Vec<String> = Vec::new();
    let mut comma = false;
    let mut ss = s.to_string();
    while ss.len() > 0 {
        let (g, sss) = brace_getitem(ss.as_str(), depth);
        ss = sss.clone();
        if ss.is_empty() {
            break;
        }
        for x in g.iter() {
            out.push(x.clone());
        }
        let c = ss.chars().next().unwrap();
        if c == '}' {
            let mut sss = ss.clone();
            sss.remove(0);
            if comma {
                return Some((out, sss));
            }
            let mut result = Vec::new();
            for x in out.iter() {
                let item = format!("{{{}}}", x);
                result.push(item);
            }
            return Some((result, ss));
        }
        if c == ',' {
            comma = true;
            ss.remove(0);
        }
    }
    return None;
}

fn expand_brace(tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();
    for (sep, token) in tokens.iter() {
        if !sep.is_empty() || !need_expand_brace(&token) {
            idx += 1;
            continue;
        }

        let mut result: Vec<String> = Vec::new();
        let items = brace_getitem(&token, 0);
        for x in items.0 {
            result.push(x.clone());
        }
        buff.push((idx, result));
        idx += 1;
    }

    for (i, items) in buff.iter().rev() {
        tokens.remove(*i as usize);
        for (j, token) in items.iter().enumerate() {
            let sep = if token.contains(' ') { "\"" } else { "" };
            tokens.insert((*i + j) as usize, (sep.to_string(), token.clone()));
        }
    }
}

fn expand_brace_range(tokens: &mut types::Tokens) {
    let re;
    if let Ok(x) = Regex::new(r#"\{(-?[0-9]+)\.\.(-?[0-9]+)(\.\.)?([0-9]+)?\}"#) {
        re = x;
    } else {
        println_stderr!("cicada: re new error");
        return;
    }

    let mut idx: usize = 0;
    let mut buff: Vec<(usize, Vec<String>)> = Vec::new();
    for (sep, token) in tokens.iter() {
        if !sep.is_empty() || !re.is_match(&token) {
            idx += 1;
            continue;
        }

        // safe to unwrap here, since the `is_match` above already validated
        let caps = re.captures(&token).unwrap();
        let start = caps[1].to_string().parse::<i32>().unwrap();
        let end = caps[2].to_string().parse::<i32>().unwrap();
        // incr is always positive
        let mut incr = if caps.get(4).is_none() {
            1
        } else {
            caps[4].to_string().parse::<i32>().unwrap()
        };
        if incr <= 1 {
            incr = 1;
        }

        let mut result: Vec<String> = Vec::new();
        let mut n = start;
        if start > end {
            while n >= end {
                result.push(format!("{}", n));
                n -= incr;
            }
        } else {
            while n <= end {
                result.push(format!("{}", n));
                n += incr;
            }
        }

        buff.push((idx, result));
        idx += 1;
    }

    for (i, items) in buff.iter().rev() {
        tokens.remove(*i as usize);
        for (j, token) in items.iter().enumerate() {
            let sep = if token.contains(' ') { "\"" } else { "" };
            tokens.insert((*i + j) as usize, (sep.to_string(), token.clone()));
        }
    }
}

pub fn expand_home_string(text: &mut String) {
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
        let home = tools::get_user_home();
        let ss = text.clone();
        let to = format!("$head{}$tail", home);
        let result = re.replace_all(ss.as_str(), to.as_str());
        *text = result.to_string();
    }
}

fn expand_alias(sh: &Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();
    let mut is_head = true;
    for (sep, text) in tokens.iter() {
        if sep.is_empty() && text == "|" {
            is_head = true;
            idx += 1;
            continue;
        }
        if is_head && text == "xargs" {
            idx += 1;
            continue;
        }

        if !is_head || !sh.is_alias(&text) {
            idx += 1;
            is_head = false;
            continue;
        }

        if let Some(value) = sh.get_alias_content(&text) {
            buff.push((idx, value.clone()));
        }

        idx += 1;
        is_head = false;
    }

    for (i, text) in buff.iter().rev() {
        let tokens_ = parsers::parser_line::cmd_to_tokens(&text);
        tokens.remove(*i as usize);
        for item in tokens_.iter().rev() {
            tokens.insert(*i as usize, item.clone());
        }
    }
}

fn expand_home(tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !needs_expand_home(&text) {
            idx += 1;
            continue;
        }

        let mut s: String = text.clone();
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
            let home = tools::get_user_home();
            let ss = s.clone();
            let to = format!("$head{}$tail", home);
            let result = re.replace_all(ss.as_str(), to.as_str());
            s = result.to_string();
        }
        buff.push((idx, s.clone()));
        idx += 1;
    }

    for (i, text) in buff.iter().rev() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn env_in_token(token: &str) -> bool {
    if libs::re::re_contains(token, r"\$\{?[a-zA-Z][a-zA-Z0-9_]*\}?") {
        return !libs::re::re_contains(token, r"='.*\$\{?[a-zA-Z][a-zA-Z0-9_]*\}?.*'$");
    }

    libs::re::re_contains(token, r"\$\{?\$\}?") ||
        libs::re::re_contains(token, r"\$\{?\?\}?")
}

pub fn expand_env(sh: &Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();

    for (sep, token) in tokens.iter() {
        if sep == "`" || sep == "'" || !env_in_token(token) {
            idx += 1;
            continue;
        }

        let _token = extend_env_blindly(sh, token);
        buff.push((idx, _token));
        idx += 1;
    }

    for (i, text) in buff.iter().rev() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn should_do_dollar_command_extension(line: &str) -> bool {
    libs::re::re_contains(line, r"\$\([^\)]+\)") &&
    !libs::re::re_contains(line, r"='.*\$\([^\)]+\).*'$")
}

fn do_command_substitution_for_dollar(sh: &mut Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, String> = HashMap::new();

    for (sep, token) in tokens.iter() {
        if sep == "'" || sep == "\\" || !should_do_dollar_command_extension(token) {
            idx += 1;
            continue;
        }

        let mut line = token.to_string();
        loop {
            if !should_do_dollar_command_extension(&line) {
                break;
            }
            let ptn_cmd = r"\$\((.+)\)";
            let cmd;
            match libs::re::find_first_group(ptn_cmd, &line) {
                Some(x) => {
                    cmd = x;
                }
                None => {
                    println_stderr!("cicada: calculator: no first group");
                    return;
                }
            }

            log!("run subcmd: {:?}", &cmd);
            let _args = parsers::parser_line::cmd_to_tokens(&cmd);
            let (_, cmd_result) =
                core::run_pipeline(sh, &_args, "", false, false, true, false, None);
            let output_txt = cmd_result.stdout.trim();

            let ptn = r"(?P<head>[^\$]*)\$\(.+\)(?P<tail>.*)";
            let re;
            if let Ok(x) = Regex::new(ptn) {
                re = x;
            } else {
                return;
            }

            let to = format!("${{head}}{}${{tail}}", output_txt);
            let line_ = line.clone();
            let result = re.replace(&line_, to.as_str());
            line = result.to_string();
        }

        buff.insert(idx, line.clone());
        idx += 1;
    }

    for (i, text) in buff.iter() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn do_command_substitution_for_dot(sh: &mut Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, String> = HashMap::new();
    for (sep, token) in tokens.iter() {
        let new_token: String;
        if sep == "`" {
            log!("run subcmd: {:?}", token);
            let _args = parsers::parser_line::cmd_to_tokens(&token);
            let (_, cr) = core::run_pipeline(sh, &_args, "", false, false, true, false, None);
            new_token = cr.stdout.trim().to_string();
        } else if sep == "\"" || sep.is_empty() {
            let re;
            if let Ok(x) = Regex::new(r"^([^`]*)`([^`]+)`(.*)$") {
                re = x;
            } else {
                println_stderr!("cicada: re new error");
                return;
            }
            if !re.is_match(&token) {
                idx += 1;
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
                    log!("run subcmd: {:?}", &cap[2]);
                    let _args = parsers::parser_line::cmd_to_tokens(&cap[2]);
                    let (_, cr) =
                        core::run_pipeline(sh, &_args, "", false, false, true, false, None);
                    _output = cr.stdout.trim().to_string();
                }
                _item = format!("{}{}{}", _item, _head, _output);
                if _tail.is_empty() {
                    break;
                }
                _token = _tail.clone();
            }
            new_token = _item;
        } else {
            idx += 1;
            continue;
        }

        buff.insert(idx, new_token.clone());
        idx += 1;
    }

    for (i, text) in buff.iter() {
        tokens[*i as usize].1 = text.to_string();
    }
}

fn do_command_substitution(sh: &mut Shell, tokens: &mut types::Tokens) {
    do_command_substitution_for_dot(sh, tokens);
    do_command_substitution_for_dollar(sh, tokens);
}

pub fn do_expansion(sh: &mut Shell, tokens: &mut types::Tokens) {
    if tokens.len() >= 2 {
        if tokens[0].1 == "export" && tokens[1].1.starts_with("PROMPT=") {
            return;
        }
    }

    expand_alias(sh, tokens);
    expand_home(tokens);
    expand_env(sh, tokens);
    expand_brace(tokens);
    expand_glob(tokens);
    do_command_substitution(sh, tokens);
    expand_brace_range(tokens);
}

pub fn needs_expand_home(line: &str) -> bool {
    libs::re::re_contains(line, r"( +~ +)|( +~/)|(^ *~/)|( +~ *$)")
}

pub fn get_rc_file() -> String {
    let home = tools::get_user_home();
    format!("{}/{}", home, ".cicadarc")
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::expand_alias;
    use super::expand_brace;
    use super::expand_brace_range;
    use super::expand_env;
    use super::libs;
    use super::needs_expand_home;
    use super::needs_globbing;
    use super::should_do_dollar_command_extension;
    use super::Shell;

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

    #[test]
    fn test_needs_globbing() {
        assert!(needs_globbing("*"));
        assert!(needs_globbing("ls *"));
        assert!(needs_globbing("ls  *.txt"));
        assert!(needs_globbing("grep -i 'desc' /etc/*release*"));
        assert!(needs_globbing("echo foo\\ 0*"));
        assert!(needs_globbing("echo foo\\ bar\\ 0*"));
        assert!(!needs_globbing("2 * 3"));
        assert!(!needs_globbing("ls '*.md'"));
        assert!(!needs_globbing("ls 'a * b'"));
        assert!(!needs_globbing("ls foo"));
    }

    #[test]
    fn test_should_do_dollar_command_extension() {
        assert!(!should_do_dollar_command_extension("ls $HOME"));
        assert!(!should_do_dollar_command_extension("echo $[pwd]"));
        assert!(!should_do_dollar_command_extension("='pwd is $(pwd).'"));
        assert!(should_do_dollar_command_extension("echo $(pwd)"));
        assert!(should_do_dollar_command_extension("echo $(pwd) foo"));
        assert!(should_do_dollar_command_extension("echo $(foo bar)"));
        assert!(should_do_dollar_command_extension("echo $(echo foo)"));
        assert!(should_do_dollar_command_extension("$(pwd) foo"));
    }

    #[test]
    fn test_expand_env() {
        let sh = Shell::new();
        env::set_var("test_foo_expand_env1", "Test foo >> ");
        env::set_var("test_foo_expand_env2", "test-foo");
        env::set_var("c", "X");

        let mut tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "$c".to_string()),
        ];
        let exp_tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "X".to_string()),
        ];
        expand_env(&sh, &mut tokens);
        assert_eq!(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![
            ("", "alias"), ("", "foo=\'echo $PWD\'")
        ]);
        let exp_tokens = vec![
            ("", "alias"), ("", "foo=\'echo $PWD\'")
        ];
        expand_env(&sh, &mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "$test_foo_expand_env1".to_string()),
        ];
        let exp_tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "Test foo >> ".to_string()),
        ];
        expand_env(&sh, &mut tokens);
        assert_eq!(tokens, exp_tokens);

        let mut tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("".to_string(), "$test_foo_expand_env2".to_string()),
        ];
        let exp_tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("".to_string(), "test-foo".to_string()),
        ];
        expand_env(&sh, &mut tokens);
        assert_eq!(tokens, exp_tokens);

        let mut tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "foo$$=-$++==$$==".to_string()),
        ];
        let ptn_expected = r"^foo[0-9]+=-\$\+\+==[0-9]+==$";
        expand_env(&sh, &mut tokens);
        if !libs::re::re_contains(&tokens[1].1, ptn_expected) {
            println!("expect RE: {:?}", ptn_expected);
            println!("real: {:?}", &tokens[1].1);
            assert!(false);
        }

        let mut tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "==$++$$foo$$=-$++==$$==$--$$end".to_string()),
        ];
        let ptn_expected = r"^==\$\+\+[0-9]+foo[0-9]+=-\$\+\+==[0-9]+==\$--[0-9]+end$";
        expand_env(&sh, &mut tokens);
        if !libs::re::re_contains(&tokens[1].1, ptn_expected) {
            println!("expect RE: {:?}", ptn_expected);
            println!("real: {:?}", &tokens[1].1);
            assert!(false);
        }
    }

    #[test]
    fn test_expand_alias() {
        let mut sh = Shell::new();
        sh.add_alias("ls", "ls --color=auto");
        sh.add_alias("wc", "wc -l");

        let mut tokens = vec![
            ("".to_string(), "ls".to_string()),
            ("".to_string(), "|".to_string()),
            ("".to_string(), "wc".to_string()),
        ];
        let exp_tokens = vec![
            ("".to_string(), "ls".to_string()),
            ("".to_string(), "--color=auto".to_string()),
            ("".to_string(), "|".to_string()),
            ("".to_string(), "wc".to_string()),
            ("".to_string(), "-l".to_string()),
        ];
        expand_alias(&sh, &mut tokens);
        assert_eq!(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![
             ("", "foo"), ("", "|"), ("", "xargs"), ("", "ls"),
        ]);
        let exp_tokens = vec![
            ("", "foo"), ("", "|"), ("", "xargs"),
            ("", "ls"), ("", "--color=auto"),
        ];
        expand_alias(&sh, &mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = vec![
            ("".to_string(), "which".to_string()),
            ("".to_string(), "ls".to_string()),
        ];
        let exp_tokens = vec![
            ("".to_string(), "which".to_string()),
            ("".to_string(), "ls".to_string()),
        ];
        expand_alias(&sh, &mut tokens);
        assert_eq!(tokens, exp_tokens);
    }

    fn assert_vec_eq(v1: Vec<(String, String)>, v2: Vec<(&str, &str)>) {
        let mut v3: Vec<(&str, &str)> = Vec::new();
        for (k, v) in v1.iter() {
            v3.push((k.as_str(), v.as_str()));
        }
        assert_eq!(v3, v2);
    }

    fn make_tokens(v: &Vec<(&str, &str)>) -> Vec<(String, String)> {
        let mut tokens = Vec::new();
        for (k, v) in v.iter() {
            tokens.push((k.to_string(), v.to_string()));
        }
        tokens
    }

    #[test]
    fn test_expand_brace() {
        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f{1,2}.txt")]);
        let exp_tokens = vec![("", "echo"), ("", "f1.txt"), ("", "f2.txt")];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f{1,2,3,5}.txt")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "f1.txt"),
            ("", "f2.txt"),
            ("", "f3.txt"),
            ("", "f5.txt"),
        ];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f{1,}.txt")]);
        let exp_tokens = vec![("", "echo"), ("", "f1.txt"), ("", "f.txt")];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f{,1}.txt")]);
        let exp_tokens = vec![("", "echo"), ("", "f.txt"), ("", "f1.txt")];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f{,}.txt")]);
        let exp_tokens = vec![("", "echo"), ("", "f.txt"), ("", "f.txt")];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f {1,2}.txt")]);
        let exp_tokens = vec![("", "echo"), ("\"", "f 1.txt"), ("\"", "f 2.txt")];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f {1,2}.txt"), ("", "bar.rs")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("\"", "f 1.txt"),
            ("\"", "f 2.txt"),
            ("", "bar.rs"),
        ];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "f{1,2}b{3,4}.txt")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "f1b3.txt"),
            ("", "f1b4.txt"),
            ("", "f2b3.txt"),
            ("", "f2b4.txt"),
        ];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{a,f{1,2}}b.txt")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "ab.txt"),
            ("", "f1b.txt"),
            ("", "f2b.txt"),
        ];
        expand_brace(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);
    }

    #[test]
    fn test_expand_brace_range() {
        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{1..4}")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "1"), ("", "2"), ("", "3"), ("", "4"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{1..3..0}")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "1"), ("", "2"), ("", "3"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{-2..1}")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "-2"), ("", "-1"), ("", "0"), ("", "1"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{3..1}")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "3"), ("", "2"), ("", "1"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{10..4..3}")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "10"), ("", "7"), ("", "4"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{10..3..2}")]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "10"), ("", "8"), ("", "6"), ("", "4"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![
             ("", "echo"),
             ("", "foo"),
             ("", "{1..3}"),
             ("", "bar"),
             ("", "{1..10..3}"),
             ("", "end"),
        ]);
        let exp_tokens = vec![
            ("", "echo"),
            ("", "foo"),
            ("", "1"), ("", "2"), ("", "3"),
            ("", "bar"),
            ("", "1"), ("", "4"), ("", "7"), ("", "10"),
            ("", "end"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);
    }
}
