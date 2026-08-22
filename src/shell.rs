use errno::errno;
use std::collections::{HashMap, HashSet};
use std::env;
use std::io::Write;
use std::mem;
use std::path::{Path, PathBuf};

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
    pub heredocs: HashMap<String, types::Heredoc>,
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
            heredocs: HashMap::new(),
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

    pub fn mark_job_member_continued(&mut self, pid: i32, gid: i32) -> Option<&types::Job> {
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

    pub fn mark_job_member_stopped(&mut self, pid: i32, gid: i32) -> Option<&types::Job> {
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
            None => env::var(name).ok(),
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

    pub fn remove_path(&mut self, path_to_remove: &Path) {
        if let Ok(paths) = env::var("PATH") {
            let mut paths_new: Vec<PathBuf> = env::split_paths(&paths).collect();
            paths_new.retain(|x| x != path_to_remove);
            let joined = env::join_paths(paths_new).unwrap_or_default();
            env::set_var("PATH", joined);
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

/// Read the name of a `${NAME}` that starts at `chars[i]` (a `$`).
///
/// Returns `None` for every other braced form, e.g. `${NAME:-default}`, so
/// that syntax we do not implement is left alone instead of half-expanded.
pub(crate) fn read_braced_name(chars: &[char], i: usize) -> Option<(String, usize)> {
    if chars.get(i + 1) != Some(&'{') {
        return None;
    }

    let start = i + 2;
    // `${$}` and `${?}`
    if let Some(c) = chars.get(start) {
        if (*c == '$' || *c == '?') && chars.get(start + 1) == Some(&'}') {
            return Some((c.to_string(), start + 2));
        }
    }

    let mut j = start;
    while let Some(c) = chars.get(j) {
        if c.is_ascii_alphanumeric() || *c == '_' {
            j += 1;
        } else {
            break;
        }
    }

    if j == start || chars.get(j) != Some(&'}') {
        return None;
    }
    Some((chars[start..j].iter().collect(), j + 1))
}

/// Read the name of a `$NAME`, `$$` or `$?` that starts at `chars[i]` (a `$`).
pub(crate) fn read_bare_name(chars: &[char], i: usize) -> Option<(String, usize)> {
    let c = chars.get(i + 1)?;
    if *c == '$' || *c == '?' {
        return Some((c.to_string(), i + 2));
    }

    let start = i + 1;
    let mut j = start;
    while let Some(c) = chars.get(j) {
        if c.is_ascii_alphanumeric() || *c == '_' {
            j += 1;
        } else {
            break;
        }
    }

    if j == start {
        return None;
    }
    Some((chars[start..j].iter().collect(), j))
}

/// The value an expandable name stands for. `$` is the pid, `?` the status of
/// the previous command, and a name that is not set expands to nothing.
pub(crate) fn env_value_of(sh: &Shell, key: &str) -> String {
    if key == "?" {
        return sh.previous_status.to_string();
    }

    if key == "$" {
        unsafe {
            return libc::getpid().to_string();
        }
    }

    if let Ok(val) = env::var(key) {
        return val;
    }

    if let Some(val) = sh.get_env(key) {
        return val;
    }

    String::new()
}

/// Expand `$NAME`, `${NAME}`, `$$` and `$?` in `token`.
///
/// The scan walks the source text once, from left to right, and appends each
/// value to the result without looking at it again. A value that contains a
/// `$` or a newline therefore stays data, and forms we cannot expand -- such
/// as `${NAME:-default}` or the `}` of `awk '{print $NF}'` -- are copied
/// through untouched instead of being retried until the shell spins.
fn expand_one_env(sh: &Shell, token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        // A marked character is text -- an escaped `$` the parser marked, or
        // one a value brought in -- so copy it out with its mark still on: the
        // steps after this one need the mark too, and `do_expansion` takes
        // them all off at the end.
        if chars[i] == DATA_MARK {
            result.push(chars[i]);
            i += 1;
            if i < chars.len() {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }

        if chars[i] != '$' {
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if let Some((name, next)) = read_braced_name(&chars, i) {
            result.push_str(&mark_as_data(&env_value_of(sh, &name)));
            i = next;
            continue;
        }

        if let Some((name, next)) = read_bare_name(&chars, i) {
            result.push_str(&mark_as_data(&env_value_of(sh, &name)));
            i = next;
            continue;
        }

        result.push('$');
        i += 1;
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

            let c = match ss.chars().next() {
                Some(x) => x,
                None => return (out, ss),
            };

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

/// Characters that mean something to the shell when they stand in a word of
/// their own, or inside one: redirections, pipes and background.
const OPERATOR_CHARS: [char; 4] = ['<', '>', '|', '&'];

/// Quote marker put on a word once expansion has put an operator character
/// into it. Later stages only treat a word as an operator when its marker is
/// empty, so this keeps generated bytes as data.
///
/// It is deliberately not `"`: a word the user wrote in double quotes and a
/// word expansion generated need to stay distinguishable, because they are
/// treated differently by `drain_env_tokens` and by backtick substitution.
pub const SEP_GENERATED: &str = "\u{0}";

/// Put in front of a character to say that it is text, and not the syntax it
/// looks like: a `$` or backtick that would otherwise start a substitution, a
/// quote that would otherwise quote.
///
/// Two things get marked. A value a word brought in is data: `V='$(cmd)';
/// echo "$V"` prints the text, and a value read from a file cannot run a
/// command by containing one. And a character the line escaped is data by the
/// author's say-so: `\$b` is a dollar and a `b`, `V=a\"b` holds a quote.
///
/// Every step that could read one of these as syntax -- expansion,
/// substitution, quote removal -- steps over a marked character instead, and
/// `do_expansion` takes the marks back out once the last of them has run.
/// `\u{0}` cannot come from a command line, which is what makes it usable as
/// a mark -- the same reason `SEP_GENERATED` uses it, one field over, for the
/// same kind of job.
pub(crate) const DATA_MARK: char = '\u{0}';

/// A value on its way into a word, with every character that could be read as
/// syntax -- substitution or quoting -- marked as the text it is.
fn mark_as_data(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if c == DATA_MARK {
            continue;
        }
        if c == '$' || c == '`' || c == '\'' || c == '"' {
            out.push(DATA_MARK);
        }
        out.push(c);
    }
    out
}

/// Quote removal: the last thing expansion does to a word.
///
/// Most words never need it -- the parser records a word that *begins* with a
/// quote in the token's `sep` and keeps the quotes out of the text. What it
/// cannot record that way is a quote that opens partway into an assignment
/// value (`V=a"x"c`), because one word can hold several such stretches and
/// there is only one `sep`. Those quotes stay in the text, where substitution
/// needs them anyway to tell quoted from unquoted, and come out here.
///
/// A marked quote stays: it is a character of the value, not syntax -- one
/// that was escaped (`V=a\"b`), one inside a stretch opened by the other
/// quote (the apostrophe in `V="it's"`), or one a value brought in
/// (`B='x"y'; A=$B`).
fn remove_quotes(tokens: &mut types::Tokens) {
    for (sep, token) in tokens.iter_mut() {
        // Only a word the parser left unquoted can still hold syntax quotes,
        // and of those only an assignment: anything else with a quote in its
        // text got it from a value, or from inside a `$(...)` that did not run.
        if !sep.is_empty() || !libs::re::re_contains(token, r"(?s)^[a-zA-Z0-9_]+=") {
            continue;
        }
        if !token.contains('\'') && !token.contains('"') {
            continue;
        }

        let mut out = String::with_capacity(token.len());
        let mut chars = token.chars();
        let mut in_single = false;
        let mut in_double = false;
        let mut freed_operator = false;
        while let Some(c) = chars.next() {
            if c == DATA_MARK {
                out.push(c);
                if let Some(marked) = chars.next() {
                    out.push(marked);
                }
                continue;
            }
            if c == '\'' && !in_double {
                in_single = !in_single;
                continue;
            }
            if c == '"' && !in_single {
                in_double = !in_double;
                continue;
            }
            if (in_single || in_double) && OPERATOR_CHARS.contains(&c) {
                freed_operator = true;
            }
            out.push(c);
        }
        *token = out;

        // The quotes were the only thing saying that a `>` in the value is
        // text: `export E1=' >'` sets a variable, it does not redirect. Now
        // that they are gone, say it the way the rest of expansion does.
        if freed_operator {
            *sep = SEP_GENERATED.to_string();
        }
    }
}

/// Take the marks back out, once no step is left that could read a marked
/// character as syntax.
fn strip_data_marks(tokens: &mut types::Tokens) {
    for (_, token) in tokens.iter_mut() {
        if token.contains(DATA_MARK) {
            *token = token.replace(DATA_MARK, "");
        }
    }
}

/// Decide the quote marker for a word whose text expansion has just changed.
///
/// A word keeps its marker unless expansion *introduced* an operator
/// character: `echo hi >$F` must still redirect, because its `>` is in the
/// source, while `V='a>b'; echo $V` must print `a>b`, because that `>` only
/// appeared once `$V` was replaced by its value.
///
/// "Introduced" is judged for the token as a whole, not per character: a
/// source token that already contains *any* operator character is never
/// marked, even if expansion adds a different one. That is safe today because
/// operator recognition compares whole tokens (`types::is_op`), but it is a
/// containment, not a parse -- the real fix is to recognize operators before
/// expansion runs.
fn sep_after_expansion(sep: &str, before: &str, after: &str) -> String {
    // A backtick word is replaced wholesale by the output of its command, so
    // the marker has to change: `` sep == "`" `` means "still to be run".
    let sep_kept = if sep == "`" { "" } else { sep };
    if !sep_kept.is_empty() || before.contains(OPERATOR_CHARS) || !after.contains(OPERATOR_CHARS) {
        return sep_kept.to_string();
    }
    SEP_GENERATED.to_string()
}

/// Expand `$NAME` in a word list, leaving no marks behind.
///
/// This is the entry point for callers outside `do_expansion`, which has more
/// steps to run before the marks can come off (see `DATA_MARK`).
pub fn expand_env(sh: &Shell, tokens: &mut types::Tokens) {
    expand_env_marked(sh, tokens);
    strip_data_marks(tokens);
}

fn expand_env_marked(sh: &Shell, tokens: &mut types::Tokens) {
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

        // Expand the source token exactly once. Re-scanning the result would
        // expand a `$...` that came *out* of a value (`A='$HOME'; echo $A`),
        // and could never terminate when the text left is something we do not
        // implement, e.g. `${V:-default}`.
        let text = expand_one_env(sh, token);
        let sep_new = sep_after_expansion(sep, token, &text);
        buff.push((idx, sep_new, text));
        idx += 1;
    }

    for (i, sep, text) in buff.iter().rev() {
        tokens[*i].0 = sep.to_string();
        tokens[*i].1 = text.to_string();
    }
}

/// A cheap look for `$(...)` before the word is scanned properly.
///
/// Whether a `$(` found here is really a substitution is `find_sub_start`'s
/// question -- it knows which of them are quoted, and which came out of a
/// value -- so this only has to be quick and never miss one.
fn should_do_dollar_command_extension(line: &str) -> bool {
    // `(?s)` so that a substitution whose body spans a newline is recognized.
    libs::re::re_contains(line, r"(?s)\$\(.+\)")
}

/// Where a substitution opened by `opener` (`$` for `$(...)`, a backtick for
/// `` `...` ``) starts in `bytes`, if one does.
///
/// Two kinds of character are stepped over rather than read as syntax:
///
/// * one carrying `DATA_MARK`, because it came out of a value and is text;
/// * one inside single quotes, when `honor_quotes` says the quoting is part of
///   the word. That is where ``V='`cmd`'`` hides its backticks: the parser
///   leaves the quotes in the text of a word that does not begin with one.
///
/// Double quotes are not a hiding place -- `V="$(date)"` runs `date`, as it
/// does in any shell -- but they do turn an apostrophe inside them into an
/// ordinary character. A word the parser already recorded as quoted keeps its
/// marker, and `honor_quotes` is false for it: `echo "it's $(date)"` arrives
/// here as the text between the double quotes.
fn find_sub_start(bytes: &[u8], opener: u8, honor_quotes: bool) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];

        if c == DATA_MARK as u8 {
            i += 2;
            continue;
        }
        if c == b'\\' && !in_single {
            i += 2;
            continue;
        }

        if in_single {
            if c == b'\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }

        if honor_quotes {
            if c == b'\'' && !in_double {
                in_single = true;
                i += 1;
                continue;
            }
            if c == b'"' {
                in_double = !in_double;
                i += 1;
                continue;
            }
        }

        let opens = if opener == b'$' {
            c == b'$' && bytes.get(i + 1) == Some(&b'(')
        } else {
            c == opener
        };
        if opens {
            return Some(i);
        }

        i += 1;
    }
    None
}

/// Find the first `` `...` `` in `line`: the command inside it and the byte
/// range the whole thing occupies.
fn find_first_backtick_cmdsub(line: &str, honor_quotes: bool) -> Option<(String, usize, usize)> {
    let bytes = line.as_bytes();
    let start = find_sub_start(bytes, b'`', honor_quotes)?;

    let mut i = start + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            return Some((line[start + 1..i].to_string(), start, i + 1));
        }
        i += 1;
    }
    None
}

/// Find the first `$(...)` in `line` and return the command inside it, plus
/// the byte range the whole `$(...)` occupies.
///
/// Parentheses are counted so that the match ends at the `)` that closes the
/// substitution we started, and not at the first `)` (which would truncate
/// `$(echo $(echo N))`) nor at the last one (which would swallow the `-` and
/// the second command of `$(echo A)-$(echo B)`).
///
/// A parenthesis inside quotes is a character, not a delimiter, so
/// `$(echo "a)b")` runs the whole `echo "a)b"`.
fn find_first_dollar_cmdsub(line: &str, honor_quotes: bool) -> Option<(String, usize, usize)> {
    let bytes = line.as_bytes();
    let start = find_sub_start(bytes, b'$', honor_quotes)?;

    let mut depth = 0;
    // The quote we are inside, if any: `'`, `"` or a backtick.
    let mut quote = 0u8;
    let mut i = start + 1;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' && quote != b'\'' {
            // An escaped character cannot open, close or delimit anything.
            i += 2;
            continue;
        }
        if quote != 0 {
            if c == quote {
                quote = 0;
            }
        } else if c == b'\'' || c == b'"' || c == b'`' {
            quote = c;
        } else if c == b'(' {
            depth += 1;
        } else if c == b')' {
            depth -= 1;
            if depth == 0 {
                return Some((line[start + 2..i].to_string(), start, i + 1));
            }
        }
        i += 1;
    }
    None
}

/// Run the text inside a substitution and return what it wrote to stdout.
///
/// The body is a command list, not a single pipeline, so `$(printf a; printf
/// b)` runs both commands and yields `ab`. `line_to_cmds` keeps `;`, `&&` and
/// newlines inside a `$(...)` out of the *outer* split for the same reason.
fn run_substitution(sh: &mut Shell, cmd: &str) -> String {
    let mut output = String::new();
    for cr in crate::execute::run_command_line(sh, cmd, true, true) {
        output.push_str(&cr.stdout);
    }
    output
}

fn do_command_substitution_for_dollar(sh: &mut Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, (String, String)> = HashMap::new();

    for (sep, token) in tokens.iter() {
        if sep == "'" || sep == "\\" || !should_do_dollar_command_extension(token) {
            idx += 1;
            continue;
        }

        // Rebuild the token left to right. `done` holds the text we are
        // finished with, so each substitution is run exactly once and its
        // output is never looked at again.
        let mut done = String::new();
        let mut rest = token.to_string();
        while let Some((cmd, start, end)) = find_first_dollar_cmdsub(&rest, sep.is_empty()) {
            log!("run subcmd dollar: {:?}", &cmd);
            let output = run_substitution(sh, &cmd);

            // Splice the output in directly. Passing it through a regex
            // replacement template would let a `$0` or `$name` in the output
            // rewrite the command line, and a `$0` would put the `$(...)`
            // back and run the command again forever.
            done.push_str(&rest[..start]);
            done.push_str(&mark_as_data(output.trim()));
            rest = rest[end..].to_string();
        }
        done.push_str(&rest);

        buff.insert(idx, (sep_after_expansion(sep, token, &done), done));
        idx += 1;
    }

    for (i, (sep, text)) in buff.iter() {
        tokens[*i].0 = sep.to_string();
        tokens[*i].1 = text.to_string();
    }
}

fn do_command_substitution_for_dot(sh: &mut Shell, tokens: &mut types::Tokens) {
    let mut idx: usize = 0;
    let mut buff: HashMap<usize, (String, String)> = HashMap::new();

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

            new_token = mark_as_data(cr.stdout.trim());
        } else if sep == "\"" || sep.is_empty() {
            // Rebuild the token left to right, as the `$(...)` pass does: each
            // substitution runs once, and its output is marked as data so that
            // nothing coming out of one is read as syntax afterwards.
            let mut done = String::new();
            let mut rest = token.to_string();
            let mut found = false;
            while let Some((cmd, start, end)) = find_first_backtick_cmdsub(&rest, sep.is_empty()) {
                found = true;
                log!("run subcmd dot2: {:?}", &cmd);

                let output = match CommandLine::from_line(&cmd, sh) {
                    Ok(c) => {
                        let (term_given, cr) = core::run_pipeline(sh, &c, true, true, false);
                        if term_given {
                            unsafe {
                                let gid = libc::getpgid(0);
                                give_terminal_to(gid);
                            }
                        }
                        cr.stdout.trim().to_string()
                    }
                    Err(e) => {
                        println_stderr!("cicada: {}", e);
                        String::new()
                    }
                };

                done.push_str(&rest[..start]);
                done.push_str(&mark_as_data(&output));
                rest = rest[end..].to_string();
            }

            if !found {
                idx += 1;
                continue;
            }
            done.push_str(&rest);
            new_token = done;
        } else {
            idx += 1;
            continue;
        }

        buff.insert(
            idx,
            (
                sep_after_expansion(sep, token, &new_token),
                new_token.clone(),
            ),
        );
        idx += 1;
    }

    for (i, (sep, text)) in buff.iter() {
        tokens[*i].0 = sep.to_string();
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

    // A prompt is kept whole: its `$` sequences are the prompt's own syntax,
    // to be read when the prompt is drawn, not names to expand now. Quote
    // removal still has to run -- `export PROMPT="..."` is quoted like any
    // other assignment, and the quotes are not part of the prompt.
    if tokens.len() >= 2 && tokens[0].1 == "export" && tokens[1].1.starts_with("PROMPT=") {
        remove_quotes(tokens);
        strip_data_marks(tokens);
        return;
    }

    expand_alias(sh, tokens);
    expand_home(tokens);
    expand_env_marked(sh, tokens);
    expand_brace(tokens);
    expand_glob(tokens);
    do_command_substitution(sh, tokens);
    expand_brace_range(tokens);
    remove_quotes(tokens);
    strip_data_marks(tokens);
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
    use super::env_in_token;
    use super::expand_alias;
    use super::expand_brace;
    use super::expand_brace_range;
    use super::expand_env;
    use super::expand_one_env;
    use super::find_first_dollar_cmdsub;
    use super::libs;
    use super::needs_globbing;
    use super::remove_quotes;
    use super::sep_after_expansion;
    use super::should_do_dollar_command_extension;
    use super::Shell;
    use super::DATA_MARK;
    use super::SEP_GENERATED;
    use std::env;

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
        // quoted or not is decided later, by `find_sub_start`
        assert!(should_do_dollar_command_extension("='pwd is $(pwd).'"));
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

        let mut tokens = make_tokens(&vec![("", "alias"), ("", "foo=\'echo $PWD\'")]);
        let exp_tokens = vec![("", "alias"), ("", "foo=\'echo $PWD\'")];
        expand_env(&sh, &mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "awk"), ("\"", "{print $NF}")]);
        let exp_tokens = vec![("", "awk"), ("\"", "{print }")];
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
            panic!("expect RE: {:?}, real: {:?}", ptn_expected, tokens[1].1);
        }

        let mut tokens = vec![
            ("".to_string(), "echo".to_string()),
            (
                "\"".to_string(),
                "==$++$$foo$$=-$++==$$==$--$$end".to_string(),
            ),
        ];
        let ptn_expected = r"^==\$\+\+[0-9]+foo[0-9]+=-\$\+\+==[0-9]+==\$--[0-9]+end$";
        expand_env(&sh, &mut tokens);
        if !libs::re::re_contains(&tokens[1].1, ptn_expected) {
            panic!("expect RE: {:?}, real: {:?}", ptn_expected, tokens[1].1);
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

        let mut tokens = make_tokens(&vec![("", "foo"), ("", "|"), ("", "xargs"), ("", "ls")]);
        let exp_tokens = vec![
            ("", "foo"),
            ("", "|"),
            ("", "xargs"),
            ("", "ls"),
            ("", "--color=auto"),
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
        assert!(!env_in_token(
            "VERSION=$(foobar -h | grep 'version: v' | awk '{print $NF}')"
        ));
        assert!(!env_in_token(
            "VERSION=`foobar -h | grep 'version: v' | awk '{print $NF}'`"
        ));
        assert!(!env_in_token("foo='echo $PWD'"));
    }

    #[test]
    fn test_expand_brace_range() {
        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{1..4}")]);
        let exp_tokens = vec![("", "echo"), ("", "1"), ("", "2"), ("", "3"), ("", "4")];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{1..3..0}")]);
        let exp_tokens = vec![("", "echo"), ("", "1"), ("", "2"), ("", "3")];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{-2..1}")]);
        let exp_tokens = vec![("", "echo"), ("", "-2"), ("", "-1"), ("", "0"), ("", "1")];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{3..1}")]);
        let exp_tokens = vec![("", "echo"), ("", "3"), ("", "2"), ("", "1")];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{10..4..3}")]);
        let exp_tokens = vec![("", "echo"), ("", "10"), ("", "7"), ("", "4")];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);

        let mut tokens = make_tokens(&vec![("", "echo"), ("", "{10..3..2}")]);
        let exp_tokens = vec![("", "echo"), ("", "10"), ("", "8"), ("", "6"), ("", "4")];
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
            ("", "1"),
            ("", "2"),
            ("", "3"),
            ("", "bar"),
            ("", "1"),
            ("", "4"),
            ("", "7"),
            ("", "10"),
            ("", "end"),
        ];
        expand_brace_range(&mut tokens);
        assert_vec_eq(tokens, exp_tokens);
    }

    /// A value is expanded once. Text that came *out* of a value is data, so a
    /// `$` in it stays a `$`, and forms we do not implement are copied through
    /// instead of being retried forever (the old loop spun on these).
    #[test]
    fn test_expand_one_env_does_not_rescan() {
        let sh = Shell::new();
        env::set_var("test_eoe_plain", "abc");
        env::set_var("test_eoe_dollar", "$test_eoe_plain");
        env::set_var("test_eoe_newline", "PREFIX\n");

        assert_eq!(expand_one_env(&sh, "$test_eoe_plain"), "abc");
        assert_eq!(expand_one_env(&sh, "${test_eoe_plain}"), "abc");
        assert_eq!(
            expand_one_env(&sh, "pre${test_eoe_plain}post"),
            "preabcpost"
        );

        // the value of `test_eoe_dollar` is not expanded again. The `$`
        // it brought in comes back marked as data, so that no later step reads
        // it as syntax either; `do_expansion` drops the mark at the end.
        assert_eq!(
            expand_one_env(&sh, "$test_eoe_dollar"),
            format!("{}$test_eoe_plain", DATA_MARK)
        );
        assert_eq!(
            expand_one_env(&sh, "${test_eoe_dollar}"),
            format!("{}$test_eoe_plain", DATA_MARK)
        );

        // a newline in a value neither hangs nor eats the prefix.
        assert_eq!(
            expand_one_env(&sh, "$test_eoe_newline$test_eoe_plain"),
            "PREFIX\nabc"
        );
        assert_eq!(
            expand_one_env(&sh, "${test_eoe_newline}${test_eoe_plain}"),
            "PREFIX\nabc"
        );

        // unimplemented `${...}` forms are left as they were written.
        for form in &[
            "${test_eoe_plain:-d}",
            "${test_eoe_plain-d}",
            "${test_eoe_plain:=d}",
            "${test_eoe_plain:+d}",
            "${test_eoe_plain%.txt}",
            "${test_eoe_plain#pat}",
            "${test_eoe_plain:1:2}",
            "${#test_eoe_plain}",
        ] {
            assert_eq!(&expand_one_env(&sh, form), form);
        }

        // A name that is not set expands to nothing, and a lone `$` stays.
        assert_eq!(expand_one_env(&sh, "[$test_eoe_unset_xyz]"), "[]");
        assert_eq!(expand_one_env(&sh, "a $ b"), "a $ b");
        assert_eq!(expand_one_env(&sh, "{print $NF}"), "{print }");
    }

    /// `$(...)` is delimited by counting parentheses, so siblings do not merge
    /// and a nested substitution is not cut at the first `)`.
    #[test]
    fn test_find_first_dollar_cmdsub() {
        let (cmd, start, end) = find_first_dollar_cmdsub("$(echo A)-$(echo B)", true).unwrap();
        assert_eq!(cmd, "echo A");
        assert_eq!((start, end), (0, 9));

        let (cmd, start, end) = find_first_dollar_cmdsub("x$(echo $(echo N))y", true).unwrap();
        assert_eq!(cmd, "echo $(echo N)");
        assert_eq!((start, end), (1, 18));

        let (cmd, ..) = find_first_dollar_cmdsub("pre$(printf a; printf b)post", true).unwrap();
        assert_eq!(cmd, "printf a; printf b");

        // A paren inside quotes is a character, not the closing delimiter.
        let (cmd, ..) = find_first_dollar_cmdsub("$(echo \"a)b\")", true).unwrap();
        assert_eq!(cmd, "echo \"a)b\"");
        let (cmd, ..) = find_first_dollar_cmdsub("$(echo 'a)b')", true).unwrap();
        assert_eq!(cmd, "echo 'a)b'");
        let (cmd, ..) = find_first_dollar_cmdsub("$(echo \\))", true).unwrap();
        assert_eq!(cmd, "echo \\)");

        assert!(find_first_dollar_cmdsub("echo $HOME", true).is_none());
        // Unbalanced: no match rather than a truncated command.
        assert!(find_first_dollar_cmdsub("echo $(echo A", true).is_none());
    }

    /// an operator character that expansion *introduced* must not make the
    /// word syntax, while one written in the source still must.
    #[test]
    fn test_sep_after_expansion() {
        // Generated operator characters: word becomes a quoted literal.
        assert_eq!(sep_after_expansion("", "$V", "a>b"), SEP_GENERATED);
        assert_eq!(sep_after_expansion("", "$V", "|"), SEP_GENERATED);
        assert_eq!(sep_after_expansion("", "$V", "<"), SEP_GENERATED);
        assert_eq!(sep_after_expansion("", "$V", "<<<"), SEP_GENERATED);
        assert_eq!(sep_after_expansion("", "$V", "&"), SEP_GENERATED);
        // A backtick word is replaced by its output, so it is checked too.
        assert_eq!(sep_after_expansion("`", "cat f", "a>b"), SEP_GENERATED);

        // Source operators are untouched: `echo hi >$F` still redirects.
        assert_eq!(sep_after_expansion("", ">$F", ">out.txt"), "");
        assert_eq!(sep_after_expansion("", "2>&1", "2>&1"), "");
        // No operator character at all: nothing to protect.
        assert_eq!(sep_after_expansion("", "$V", "plain"), "");
        // An already-quoted word keeps its own marker.
        assert_eq!(sep_after_expansion("'", "$V", "a>b"), "'");
        // A backtick word with no operator in its output is plain again.
        assert_eq!(sep_after_expansion("`", "uname", "Darwin"), "");
    }

    #[test]
    fn test_remove_quotes() {
        let mark = DATA_MARK;
        let removed = |sep: &str, token: &str| {
            let mut tokens = vec![(sep.to_string(), token.to_string())];
            remove_quotes(&mut tokens);
            tokens[0].1.clone()
        };
        let marker = |token: &str| {
            let mut tokens = vec![(String::new(), token.to_string())];
            remove_quotes(&mut tokens);
            tokens[0].0.clone()
        };

        // The quotes an assignment value was written with come out, wherever
        // in the value they open.
        assert_eq!(removed("", "V=a\"x\"c"), "V=axc");
        assert_eq!(removed("", "V=a'x'c"), "V=axc");
        assert_eq!(removed("", "V=\"x\""), "V=x");
        assert_eq!(removed("", "V=a\"x\"c\"y\"d"), "V=axcyd");
        assert_eq!(removed("", "V=a\"b c\""), "V=ab c");
        assert_eq!(removed("", "V=a\"\"b"), "V=ab");

        // A quote inside a stretch the other quote opened is a character.
        assert_eq!(removed("", "V=\"it's\""), "V=it's");
        assert_eq!(removed("", "V='a\"b'"), "V=a\"b");

        // A marked quote is a character too: it was escaped, or a value
        // brought it in. Its mark rides along to `strip_data_marks`.
        assert_eq!(
            removed("", &format!("V=a{}\"b", mark)),
            format!("V=a{}\"b", mark)
        );
        assert_eq!(
            removed("", &format!("V={}'x{}'", mark, mark)),
            format!("V={}'x{}'", mark, mark)
        );

        // A word the parser already recorded as quoted has no syntax quotes
        // left in its text, and a word that is not an assignment never had
        // any: neither is touched.
        assert_eq!(removed("\"", "a\"b"), "a\"b");
        assert_eq!(removed("'", "a'b"), "a'b");
        assert_eq!(removed(SEP_GENERATED, "V=a\"b"), "V=a\"b");
        assert_eq!(removed("", "echo"), "echo");
        assert_eq!(removed("", "--opt=\"x\""), "--opt=\"x\"");

        // An operator character the quotes were hiding is still text once
        // they are gone, so the word says so the way expansion does.
        assert_eq!(removed("", "E=' >'"), "E= >");
        assert_eq!(marker("E=' >'"), SEP_GENERATED);
        assert_eq!(marker("E=\"a|b\""), SEP_GENERATED);
        assert_eq!(marker("E='a&b'"), SEP_GENERATED);
        // ... and a value with nothing to hide keeps its plain marker.
        assert_eq!(marker("E=\"ab\""), "");
        assert_eq!(marker("E=ab"), "");
    }
}
