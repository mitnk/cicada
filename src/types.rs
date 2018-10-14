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

#[derive(Clone, Debug, Default)]
pub struct CommandResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn empty() -> CommandResult {
        CommandResult {
            status: 107,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.status == 107
    }

    pub fn ok() -> CommandResult {
        CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn from_status(status: i32) -> CommandResult {
        CommandResult {
            status: status,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn error() -> CommandResult {
        CommandResult {
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
