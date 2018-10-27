use std::collections::HashMap;

pub type Tokens = Vec<(String, String)>;
pub type Redirection = (String, String, String);

///
/// command line: `ls 'foo bar' 2>&1 > /dev/null` would be:
/// Command {
///     tokens: [("", "ls"), ("", "-G"), ("\'", "foo bar")],
///     redirects: [
///         ("2", ">", "&1"),
///         ("1", ">", "/dev/null"),
///     ],
/// }
///
#[derive(Debug)]
pub struct Command {
    pub tokens: Tokens,
    pub redirects: Vec<Redirection>,
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
    pub redirect_from: String,
    pub background: bool,
    pub isatty: bool,
    pub capture_output: bool,
    pub envs: HashMap<String, String>,
}
