use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::parsers;
use crate::parsers::parser_line::tokens_to_redirections;
use crate::shell;
use crate::libs;
use crate::tools;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WaitStatus(i32, i32, i32);

impl WaitStatus {
    pub fn from_exited(pid: i32, status: i32) -> Self {
        WaitStatus(pid, 0, status)
    }

    pub fn from_signaled(pid: i32, sig: i32) -> Self {
        WaitStatus(pid, 1, sig)
    }

    pub fn from_stopped(pid: i32, sig: i32) -> Self {
        WaitStatus(pid, 2, sig)
    }

    pub fn from_continuted(pid: i32) -> Self {
        WaitStatus(pid, 3, 0)
    }

    pub fn from_others() -> Self {
        WaitStatus(0, 9, 9)
    }

    pub fn from_error(errno: i32) -> Self {
        WaitStatus(0, 255, errno)
    }

    pub fn empty() -> Self {
        WaitStatus(0, 0, 0)
    }

    pub fn is_error(&self) -> bool {
        self.1 == 255
    }

    pub fn is_others(&self) -> bool {
        self.1 == 9
    }

    pub fn is_signaled(&self) -> bool {
        self.1 == 1
    }

    pub fn get_errno(&self) -> nix::Error {
        nix::Error::from_i32(self.2)
    }

    pub fn is_exited(&self) -> bool {
        self.0 != 0 && self.1 == 0
    }

    pub fn is_stopped(&self) -> bool {
        self.1 == 2
    }

    pub fn is_continued(&self) -> bool {
        self.1 == 3
    }

    pub fn get_pid(&self) -> i32 {
        self.0
    }

    fn _get_signaled_status(&self) -> i32 {
        self.2 + 128
    }

    pub fn get_signal(&self) -> i32 {
        self.2
    }

    pub fn get_name(&self) -> String {
        if self.is_exited() {
            "Exited".to_string()
        } else if self.is_stopped() {
            "Stopped".to_string()
        } else if self.is_continued() {
            "Continued".to_string()
        } else if self.is_signaled() {
            "Signaled".to_string()
        } else if self.is_others() {
            "Others".to_string()
        } else if self.is_error() {
            "Error".to_string()
        } else {
            format!("unknown: {}", self.2)
        }
    }

    pub fn get_status(&self) -> i32 {
        if self.is_exited() {
            self.2
        } else {
            self._get_signaled_status()
        }
    }
}

impl fmt::Debug for WaitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("WaitStatus");
        formatter.field("pid", &self.0);
        let name = self.get_name();
        formatter.field("name", &name);
        formatter.field("ext", &self.2);
        formatter.finish()
    }
}

pub type Token = (String, String);
pub type Tokens = Vec<Token>;
pub type Redirection = (String, String, String);

#[derive(Debug)]
pub struct LineInfo {
    // e.g. echo 'foo
    // is not a completed line, need to turn to multiple-line mode.
    pub tokens: Tokens,
    pub is_complete: bool,
}

impl LineInfo {
    pub fn new(tokens: Tokens) -> Self {
        LineInfo { tokens: tokens, is_complete: true }
    }
}

///
/// command line: `ls 'foo bar' 2>&1 > /dev/null < one-file` would be:
/// Command {
///     tokens: [("", "ls"), ("", "-G"), ("\'", "foo bar")],
///     redirects_to: [
///         ("2", ">", "&1"),
///         ("1", ">", "/dev/null"),
///     ],
///     redirect_from: Some(("<", "one-file")),
/// }
///
#[derive(Debug)]
pub struct Command {
    pub tokens: Tokens,
    pub redirects_to: Vec<Redirection>,
    pub redirect_from: Option<Token>,
}

#[derive(Debug)]
pub struct CommandLine {
    pub line: String,
    pub commands: Vec<Command>,
    pub envs: HashMap<String, String>,
    pub background: bool,
}

impl Command {
    pub fn from_tokens(tokens: Tokens) -> Result<Command, String> {
        let mut tokens_new = tokens.clone();
        let mut redirects_from_type = String::new();
        let mut redirects_from_value = String::new();
        let mut has_redirect_from = tokens_new.iter().any(|x| x.1 == "<" || x.1 == "<<<");

        let mut len = tokens_new.len();
        while has_redirect_from {
            if let Some(idx) = tokens_new.iter().position(|x| x.1 == "<") {
                redirects_from_type = "<".to_string();
                tokens_new.remove(idx);
                len -= 1;
                if len > idx {
                    redirects_from_value = tokens_new.remove(idx).1;
                    len -= 1;
                }
            }
            if let Some(idx) = tokens_new.iter().position(|x| x.1 == "<<<") {
                redirects_from_type = "<<<".to_string();
                tokens_new.remove(idx);
                len -= 1;
                if len > idx {
                    redirects_from_value = tokens_new.remove(idx).1;
                    len -= 1;
                }
            }

            has_redirect_from = tokens_new.iter().any(|x| x.1 == "<" || x.1 == "<<<");
        }

        let tokens_final;
        let redirects_to;
        match tokens_to_redirections(&tokens_new) {
            Ok((_tokens, _redirects_to)) => {
                tokens_final = _tokens;
                redirects_to = _redirects_to;
            }
            Err(e) => {
                return Err(e);
            }
        }

        let redirect_from = if redirects_from_type.is_empty() {
            None
        } else {
            Some((redirects_from_type, redirects_from_value))
        };

        Ok(Command{
            tokens: tokens_final,
            redirects_to: redirects_to,
            redirect_from: redirect_from,
        })
    }

    pub fn has_redirect_from(&self) -> bool {
        self.redirect_from.is_some() &&
        self.redirect_from.clone().unwrap().0 == "<"
    }

    pub fn has_here_string(&self) -> bool {
        self.redirect_from.is_some() &&
        self.redirect_from.clone().unwrap().0 == "<<<"
    }

    pub fn is_builtin(&self) -> bool {
        tools::is_builtin(&self.tokens[0].1)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Job {
    pub cmd: String,
    pub id: i32,
    pub gid: i32,
    pub pids: Vec<i32>,
    pub pids_stopped: HashSet<i32>,
    pub status: String,
    pub is_bg: bool,
}

impl Job {
    pub fn all_members_stopped(&self) -> bool {
        for pid in &self.pids {
            if !self.pids_stopped.contains(&pid) {
                return false;
            }
        }
        return true;
    }

    pub fn all_members_running(&self) -> bool {
        self.pids_stopped.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandResult {
    pub gid: i32,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn new() -> CommandResult {
        CommandResult {
            gid: 0,
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn from_status(gid: i32, status: i32) -> CommandResult {
        CommandResult {
            gid: gid,
            status: status,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn error() -> CommandResult {
        CommandResult {
            gid: 0,
            status: 1,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CommandOptions {
    pub background: bool,
    pub isatty: bool,
    pub capture_output: bool,
    pub envs: HashMap<String, String>,
}

fn split_tokens_by_pipes(tokens: &[Token]) -> Vec<Tokens> {
    let mut cmd = Vec::new();
    let mut cmds = Vec::new();
    for token in tokens {
        let sep = &token.0;
        let value = &token.1;
        if sep.is_empty() && value == "|" {
            if cmd.is_empty() {
                return Vec::new();
            }
            cmds.push(cmd.clone());
            cmd = Vec::new();
        } else {
            cmd.push(token.clone());
        }
    }
    if cmd.is_empty() {
        return Vec::new();
    }
    cmds.push(cmd.clone());
    cmds
}

fn drain_env_tokens(tokens: &mut Tokens) -> HashMap<String, String> {
    let mut envs: HashMap<String, String> = HashMap::new();
    let mut n = 0;
    let re = Regex::new(r"^([a-zA-Z0-9_]+)=(.*)$").unwrap();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !libs::re::re_contains(text, r"^([a-zA-Z0-9_]+)=(.*)$") {
            break;
        }

        for cap in re.captures_iter(text) {
            let name = cap[1].to_string();
            let value = parsers::parser_line::unquote(&cap[2]);
            envs.insert(name, value);
        }

        n += 1;
    }
    if n > 0 {
        tokens.drain(0..n);
    }
    envs
}

impl CommandLine {
    pub fn from_line(line: &str, sh: &mut shell::Shell) -> Result<CommandLine, String> {
        let linfo = parsers::parser_line::parse_line(line);
        let mut tokens = linfo.tokens;
        shell::do_expansion(sh, &mut tokens);
        let envs = drain_env_tokens(&mut tokens);

        let mut background = false;
        let len = tokens.len();
        if len > 1 && tokens[len - 1].1 == "&" {
            background = true;
            tokens.pop();
        }

        let mut commands = Vec::new();
        for sub_tokens in split_tokens_by_pipes(&tokens) {
            match Command::from_tokens(sub_tokens) {
                Ok(c) => {
                    commands.push(c);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(CommandLine{
            line: line.to_string(),
            commands: commands,
            envs: envs,
            background: background,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn with_pipeline(&self) -> bool {
        self.commands.len() > 1
    }

    pub fn is_single_and_builtin(&self) -> bool {
        self.commands.len() == 1 && self.commands[0].is_builtin()
    }
}
