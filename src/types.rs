use regex::Regex;
use std::collections::HashMap;

use crate::parsers;
use crate::parsers::parser_line::tokens_to_redirections;
use crate::shell;
use crate::libs;
use crate::tools;

pub const STOPPED: i32 = 148;

pub type Token = (String, String);
pub type Tokens = Vec<Token>;
pub type Redirection = (String, String, String);

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
    pub status: String,
    pub report: bool,
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
        let mut tokens = parsers::parser_line::cmd_to_tokens(line);
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

    pub fn with_pipeline(&self) -> bool {
        self.commands.len() > 1
    }

    pub fn is_builtin(&self) -> bool {
        self.commands.len() == 1 && self.commands[0].is_builtin()
    }
}
