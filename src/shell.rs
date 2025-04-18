use errno::errno;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::Write;
use std::mem;

use regex::Regex;
use uuid::Uuid;

use crate::core;
use crate::libs;
use crate::parsers;
use crate::tools;
use crate::types::{self, CommandLine};

#[derive(Debug, Clone)]
pub struct Shell {
    pub jobs: HashMap<i32, types::Job>,
    pub aliases: HashMap<String, String>,
    pub envs: HashMap<String, String>,
    pub funcs: HashMap<String, String>,
    pub cmd: String,
    pub current_dir: String,
    pub previous_dir: String,
    pub previous_cmd: String,
    pub previous_status: i32,
    pub is_login: bool,
    pub exit_on_error: bool,
    pub has_terminal: bool,
    pub session_id: String,
}

impl Shell {
    pub fn new() -> Shell {
        let uuid = Uuid::new_v4().as_hyphenated().to_string();
        let current_dir = tools::get_current_dir();
        // TODO: the shell proc may have terminal later
        // e.g. $ cicada foo.sh &
        // then with a $ fg
        let has_terminal = proc_has_terminal();
        let (session_id, _) = uuid.split_at(13);
        Shell {
            jobs: HashMap::new(),
            aliases: HashMap::new(),
            envs: HashMap::new(),
            funcs: HashMap::new(),
            cmd: String::new(),
            current_dir: current_dir.clone(),
            previous_dir: String::new(),
            previous_cmd: String::new(),
            previous_status: 0,
            is_login: false,
            exit_on_error: false,
            has_terminal,
            session_id: session_id.to_string(),
        }
    }

    pub fn insert_job(&mut self, gid: i32, pid: i32, cmd: &str, status: &str, bg: bool) {
        let mut i = 1;
        loop {
            let mut indexed_job_missing = false;
            if let Some(x) = self.jobs.get_mut(&i) {
                if x.gid == gid {
                    x.pids.push(pid);
                    x.cmd = format!("{} | {}", x.cmd, cmd);
                    return;
                }
            } else {
                indexed_job_missing = true;
            }

            if indexed_job_missing {
                self.jobs.insert(
                    i,
                    types::Job {
                        cmd: cmd.to_string(),
                        id: i,
                        gid,
                        pids: vec![pid],
                        pids_stopped: HashSet::new(),
                        status: status.to_string(),
                        is_bg: bg,
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

    pub fn mark_job_member_continued(&mut self, pid: i32,
                                     gid: i32) -> Option<&types::Job> {
        if self.jobs.is_empty() {
            return None;
        }
        let mut i = 1;
        let mut idx_found = 0;
        loop {
            if let Some(job) = self.jobs.get_mut(&i) {
                if job.gid == gid {
                    job.pids_stopped.remove(&pid);
                    idx_found = i;
                    break;
                }
            }


            i += 1;
            if i >= 65535 {
                break;
            }
        }

        self.jobs.get(&idx_found)
    }

    pub fn mark_job_member_stopped(&mut self, pid: i32,
                                   gid: i32) -> Option<&types::Job> {
        if self.jobs.is_empty() {
            return None;
        }
        let mut i = 1;
        let mut idx_found = 0;
        loop {
            if let Some(job) = self.jobs.get_mut(&i) {
                if job.gid == gid {
                    job.pids_stopped.insert(pid);
                    idx_found = i;
                    break;
                }
            }


            i += 1;
            if i >= 65535 {
                break;
            }
        }

        self.jobs.get(&idx_found)
    }

    pub fn get_job_by_gid(&self, gid: i32) -> Option<&types::Job> {
        if self.jobs.is_empty() {
            return None;
        }

        let mut i = 1;
        loop {
            if let Some(x) = self.jobs.get(&i) {
                if x.gid == gid {
                    return Some(x);
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
            if let Some(job) = self.jobs.get_mut(&i) {
                if job.gid == gid {
                    job.status = "Running".to_string();
                    job.pids_stopped.clear();
                    job.is_bg = bg;
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
                    x.is_bg = true;
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

    /// Update existing *ENV Variable* if such name exists in ENVs,
    /// otherwise, we define a local *Shell Variable*, which would not
    /// be exported into child processes.
    pub fn set_env(&mut self, name: &str, value: &str) {
        if env::var(name).is_ok() {
            env::set_var(name, value);
        } else {
            self.envs.insert(name.to_string(), value.to_string());
        }
    }

    /// get *Shell Variable*, or *ENV Variable*.
    pub fn get_env(&self, name: &str) -> Option<String> {
        match self.envs.get(name) {
            Some(x) => Some(x.to_string()),
            None => {
                match env::var(name) {
                    Ok(x) => Some(x),
                    Err(_) => None,
                }
            }
        }
    }

    /// Remove environment variable, function from the environment of
    /// the currently running process
    pub fn remove_env(&mut self, name: &str) -> bool {
        // function names can contain the `-` char.
        let ptn_env = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_-]*$").unwrap();
        if !ptn_env.is_match(name) {
            return false;
        }

        env::remove_var(name);
        self.envs.remove(name);
        self.remove_func(name);
        true
    }

    pub fn remove_path(&mut self, path: &str) {
        if let Ok(paths) = env::var("PATH") {
            let mut paths_new: Vec<&str> = paths.split(":").collect();
            paths_new.retain(|&x| x != path);
            env::set_var("PATH", paths_new.join(":").as_str());
        }
    }

    fn remove_func(&mut self, name: &str) {
        self.funcs.remove(name);
    }

    pub fn set_func(&mut self, name: &str, value: &str) {
        self.funcs.insert(name.to_string(), value.to_string());
    }

    pub fn get_func(&self, name: &str) -> Option<String> {
        self.funcs.get(name).map(|x| x.to_string())
    }

    pub fn get_alias_list(&self) -> Vec<(String, String)> {
        let mut result = Vec::new();
        for (name, value) in &self.aliases {
            result.push((name.clone(), value.clone()));
        }
        result
    }

    pub fn add_alias(&mut self, name: &str, value: &str) {
        self.aliases.insert(name.to_string(), value.to_string());
    }

    pub fn is_alias(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    pub fn remove_alias(&mut self, name: &str) -> bool {
        let opt = self.aliases.remove(name);
        opt.is_some()
    }

    pub fn get_alias_content(&self, name: &str) -> Option<String> {
        let result = match self.aliases.get(name) {
            Some(x) => x.to_string(),
            None => String::new(),
        };
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
    let re = Regex::new(r"\*+").expect("Invalid regex ptn");
    re.is_match(line)
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
        tokens.remove(*i);
        for (j, token) in result.iter().enumerate() {
            let sep = if token.contains(' ') { "\"" } else { "" };
            tokens.insert(*i + j, (sep.to_string(), token.clone()));
        }
    }
}

fn expand_one_env(sh: &Shell, token: &str) -> String {
    // do not combine these two into one: `\{?..\}?`,
    // otherwize `}` in `{print $NF}` would gone.
    let re1 = Regex::new(r"^(.*?)\$([A-Za-z0-9_]+|\$|\?)(.*)$").unwrap();
    let re2 = Regex::new(r"(.*?)\$\{([A-Za-z0-9_]+|\$|\?)\}(.*)$").unwrap();
    if !re1.is_match(token) && !re2.is_match(token) {
        return token.to_string();
    }

    let mut result = String::new();
    let match_re1 = re1.is_match(token);
    let match_re2 = re2.is_match(token);
    if !match_re1 && !match_re2 {
        return token.to_string();
    }

    let cap_results = if match_re1 {
        re1.captures_iter(token)
    } else {
        re2.captures_iter(token)
    };

    for cap in cap_results {
        let head = cap[1].to_string();
        let tail = cap[3].to_string();
        let key = cap[2].to_string();
        if key == "?" {
            result.push_str(format!("{}{}", head, sh.previous_status).as_str());
        } else if key == "$" {
            unsafe {
                let val = libc::getpid();
                result.push_str(format!("{}{}", head, val).as_str());
            }
        } else if let Ok(val) = env::var(&key) {
            result.push_str(format!("{}{}", head, val).as_str());
        } else if let Some(val) = sh.get_env(&key) {
            result.push_str(format!("{}{}", head, val).as_str());
        } else {
            result.push_str(&head);
        }
        result.push_str(&tail);
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
    while !ss.is_empty() {
        let c = match ss.chars().next() {
            Some(x) => x,
            None => {
                return (out, ss);
            }
        };
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
            let c;
            match ss.chars().next() {
                Some(x) => c = x,
                None => {
                    return (out, ss)
                }
            }

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
    while !ss.is_empty() {
        let (g, sss) = brace_getitem(ss.as_str(), depth);
        ss = sss.clone();
        if ss.is_empty() {
            break;
        }
        for x in g.iter() {
            out.push(x.clone());
        }

        let c = match ss.chars().next() {
            Some(x) => x,
            None => {
                break;
            }
        };
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

    None
}

fn expand_brace(tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();
    for (sep, token) in tokens.iter() {
        if !sep.is_empty() || !need_expand_brace(token) {
            idx += 1;
            continue;
        }

        let mut result: Vec<String> = Vec::new();
        let items = brace_getitem(token, 0);
        for x in items.0 {
            result.push(x.clone());
        }
        buff.push((idx, result));
        idx += 1;
    }

    for (i, items) in buff.iter().rev() {
        tokens.remove(*i);
        for (j, token) in items.iter().enumerate() {
            let sep = if token.contains(' ') { "\"" } else { "" };
            tokens.insert(*i + j, (sep.to_string(), token.clone()));
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
        if !sep.is_empty() || !re.is_match(token) {
            idx += 1;
            continue;
        }

        // safe to unwrap here, since the `is_match` above already validated
        let caps = re.captures(token).unwrap();

        let start = match caps[1].to_string().parse::<i32>() {
            Ok(x) => x,
            Err(e) => {
                println_stderr!("cicada: {}", e);
                return;
            }
        };

        let end = match caps[2].to_string().parse::<i32>() {
            Ok(x) => x,
            Err(e) => {
                println_stderr!("cicada: {}", e);
                return;
            }
        };

        // incr is always positive
        let mut incr = if caps.get(4).is_none() {
            1
        } else {
            match caps[4].to_string().parse::<i32>() {
                Ok(x) => x,
                Err(e) => {
                    println_stderr!("cicada: {}", e);
                    return;
                }
            }
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
        tokens.remove(*i);
        for (j, token) in items.iter().enumerate() {
            let sep = if token.contains(' ') { "\"" } else { "" };
            tokens.insert(*i + j, (sep.to_string(), token.clone()));
        }
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

        if !is_head || !sh.is_alias(text) {
            idx += 1;
            is_head = false;
            continue;
        }

        if let Some(value) = sh.get_alias_content(text) {
            buff.push((idx, value.clone()));
        }

        idx += 1;
        is_head = false;
    }

    for (i, text) in buff.iter().rev() {
        let linfo = parsers::parser_line::parse_line(text);
        let tokens_ = linfo.tokens;
        tokens.remove(*i);
        for item in tokens_.iter().rev() {
            tokens.insert(*i, item.clone());
        }
    }
}

fn expand_home(tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !text.starts_with("~") {
            idx += 1;
            continue;
        }

        let mut s: String = text.clone();
        let ptn = r"^~(?P<tail>.*)";
        let re = Regex::new(ptn).expect("invalid re ptn");
        let home = tools::get_user_home();
        let ss = s.clone();
        let to = format!("{}$tail", home);
        let result = re.replace_all(ss.as_str(), to.as_str());
        s = result.to_string();

        buff.push((idx, s.clone()));
        idx += 1;
    }

    for (i, text) in buff.iter().rev() {
        tokens[*i].1 = text.to_string();
    }
}

fn env_in_token(token: &str) -> bool {
    if libs::re::re_contains(token, r"\$\{?[\$\?]\}?") {
        return true;
    }

    let ptn_env_name = r"[a-zA-Z_][a-zA-Z0-9_]*";
    let ptn_env = format!(r"\$\{{?{}\}}?", ptn_env_name);
    if !libs::re::re_contains(token, &ptn_env) {
        return false;
    }

    // do not expand env in a command substitution, e.g.:
    // - echo $(echo '$HOME')
    // - VERSION=$(foobar -h | grep 'version: v' | awk '{print $NF}')
    let ptn_cmd_sub1 = format!(r"^{}=`.*`$", ptn_env_name);
    let ptn_cmd_sub2 = format!(r"^{}=\$\(.*\)$", ptn_env_name);
    if libs::re::re_contains(token, &ptn_cmd_sub1)
        || libs::re::re_contains(token, &ptn_cmd_sub2)
        || libs::re::re_contains(token, r"^\$\(.+\)$")
    {
        return false;
    }

    // for cmd-line like `alias foo='echo $PWD'`
    let ptn_env = format!(r"='.*\$\{{?{}\}}?.*'$", ptn_env_name);
    !libs::re::re_contains(token, &ptn_env)
}

pub fn expand_env(sh: &Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff = Vec::new();

    for (sep, token) in tokens.iter() {
        if sep == "`" || sep == "'" {
            idx += 1;
            continue;
        }

        if !env_in_token(token) {
            idx += 1;
            continue;
        }

        let mut _token = token.clone();
        while env_in_token(&_token) {
            _token = expand_one_env(sh, &_token);
        }
        buff.push((idx, _token));
        idx += 1;
    }

    for (i, text) in buff.iter().rev() {
        tokens[*i].1 = text.to_string();
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
            let cmd = match libs::re::find_first_group(ptn_cmd, &line) {
                Some(x) => x,
                None => {
                    println_stderr!("cicada: calculator: no first group");
                    return;
                }
            };

            let cmd_result = match CommandLine::from_line(&cmd, sh) {
                Ok(c) => {
                    log!("run subcmd dollar: {:?}", &cmd);
                    let (term_given, cr) = core::run_pipeline(sh, &c, true, true, false);
                    if term_given {
                        unsafe {
                            let gid = libc::getpgid(0);
                            give_terminal_to(gid);
                        }
                    }

                    cr
                }
                Err(e) => {
                    println_stderr!("cicada: {}", e);
                    continue;
                }
            };

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
        tokens[*i].1 = text.to_string();
    }
}

fn do_command_substitution_for_dot(sh: &mut Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, String> = HashMap::new();
    for (sep, token) in tokens.iter() {
        let new_token: String;
        if sep == "`" {
            log!("run subcmd dot1: {:?}", token);
            let cr = match CommandLine::from_line(token, sh) {
                Ok(c) => {
                    let (term_given, _cr) = core::run_pipeline(sh, &c, true, true, false);
                    if term_given {
                        unsafe {
                            let gid = libc::getpgid(0);
                            give_terminal_to(gid);
                        }
                    }

                    _cr
                }
                Err(e) => {
                    println_stderr!("cicada: {}", e);
                    continue;
                }
            };

            new_token = cr.stdout.trim().to_string();
        } else if sep == "\"" || sep.is_empty() {
            let re;
            if let Ok(x) = Regex::new(r"^([^`]*)`([^`]+)`(.*)$") {
                re = x;
            } else {
                println_stderr!("cicada: re new error");
                return;
            }
            if !re.is_match(token) {
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
                    log!("run subcmd dot2: {:?}", &cap[2]);

                    let cr = match CommandLine::from_line(&cap[2], sh) {
                        Ok(c) => {
                            let (term_given, _cr) = core::run_pipeline(sh, &c, true, true, false);
                            if term_given {
                                unsafe {
                                    let gid = libc::getpgid(0);
                                    give_terminal_to(gid);
                                }
                            }

                            _cr
                        }
                        Err(e) => {
                            println_stderr!("cicada: {}", e);
                            continue;
                        }
                    };

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
        tokens[*i].1 = text.to_string();
    }
}

fn do_command_substitution(sh: &mut Shell, tokens: &mut types::Tokens) {
    do_command_substitution_for_dot(sh, tokens);
    do_command_substitution_for_dollar(sh, tokens);
}

pub fn do_expansion(sh: &mut Shell, tokens: &mut types::Tokens) {
    let line = parsers::parser_line::tokens_to_line(tokens);
    if tools::is_arithmetic(&line) {
        return;
    }

    if tokens.len() >= 2 && tokens[0].1 == "export" && tokens[1].1.starts_with("PROMPT=") {
        return;
    }

    expand_alias(sh, tokens);
    expand_home(tokens);
    expand_env(sh, tokens);
    expand_brace(tokens);
    expand_glob(tokens);
    do_command_substitution(sh, tokens);
    expand_brace_range(tokens);
}

pub fn trim_multiline_prompts(line: &str) -> String {
    // remove sub-prompts from multiple line mode
    // 1. assuming '\n' char cannot be typed manually?
    // 2. `>>` is defined as `src/prompt/multilines.rs`
    let line_new = libs::re::replace_all(line, r"\\\n>> ", "");
    let line_new = libs::re::replace_all(&line_new, r"\| *\n>> ", "| ");
    libs::re::replace_all(&line_new, r"(?P<NEWLINE>\n)>> ", "$NEWLINE")
}

fn proc_has_terminal() -> bool {
    unsafe {
        let tgid = libc::tcgetpgrp(0);
        let pgid = libc::getpgid(0);
        tgid == pgid
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use super::env_in_token;
    use super::expand_alias;
    use super::expand_brace;
    use super::expand_brace_range;
    use super::expand_env;
    use super::libs;
    use super::needs_globbing;
    use super::should_do_dollar_command_extension;
    use super::Shell;

    #[test]
    fn test_needs_globbing() {
        assert!(needs_globbing("*"));
        assert!(needs_globbing("2*"));
        assert!(needs_globbing("ls *"));
        assert!(needs_globbing("ls  *.txt"));
        assert!(needs_globbing("grep -i 'desc' /etc/*release*"));
        assert!(needs_globbing("echo foo\\ 0*"));
        assert!(needs_globbing("echo foo\\ bar\\ 0*"));
        assert!(needs_globbing("*.1"));
        assert!(!needs_globbing("foo"));
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

        let mut tokens = make_tokens(&vec![
            ("", "awk"), ("\"", "{print $NF}")
        ]);
        let exp_tokens = vec![
            ("", "awk"), ("\"", "{print }")
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
    fn test_env_in_token() {
        assert!(env_in_token("$foo"));
        assert!(env_in_token("${foo}"));
        assert!(env_in_token("$foo125"));
        assert!(env_in_token("$fo_o125"));
        assert!(env_in_token("$_foo"));
        assert!(env_in_token("$_foo12"));
        assert!(env_in_token("${_foo12}"));

        assert!(env_in_token("$$"));
        assert!(env_in_token("$?"));
        assert!(env_in_token("${$}"));
        assert!(env_in_token("${?}"));

        assert!(!env_in_token("foobar"));
        assert!(!env_in_token("{foobar}"));
        assert!(!env_in_token("foobar123"));
        assert!(!env_in_token("foobar_123"));
        assert!(!env_in_token("$1"));
        assert!(!env_in_token("$(echo $HOME)"));
        assert!(!env_in_token("$(echo \"$HOME\")"));
        assert!(!env_in_token("$(echo \'$HOME\')"));
        assert!(!env_in_token("VERSION=$(foobar -h | grep 'version: v' | awk '{print $NF}')"));
        assert!(!env_in_token("VERSION=`foobar -h | grep 'version: v' | awk '{print $NF}'`"));
        assert!(!env_in_token("foo='echo $PWD'"));
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
