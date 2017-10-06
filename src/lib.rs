#![allow(dead_code)]
extern crate errno;
extern crate exec;
extern crate glob;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate os_type;
extern crate regex;
extern crate sqlite;
extern crate time;

#[macro_use]
extern crate nom;

use std::collections::HashMap;

#[macro_use]
mod tools;

mod shell;
mod libs;
mod history;
mod builtins;
mod execute;
mod parsers;

/// Parse command line to multiple commands.
///
/// # Examples
///
/// ```no-run
/// >>> line_to_cmds("echo foo && echo bar; echo end");
/// vec!["echo foo", "&&", "echo bar", ";", "echo end"]
/// >>> line_to_cmds("man awk | grep version");
/// vec!["man awk | grep version"]
/// ```
pub fn line_to_cmds(line: &str) -> Vec<String> {
    return parsers::parser_line::line_to_cmds(line);
}


/// parse command line to tokens.
///
/// # Examples
///
/// ```no-run
/// >>> line_to_tokens("echo 'hi yoo' | wc -l");
/// vec![
///     ("", "echo"),
///     ("'", "hi yoo"),
///     ("", "|"),
///     ("", "wc"),
///     ("", "-l"),
/// ]
/// ```
pub fn line_to_tokens(line: &str) -> Vec<(String, String)> {
    return parsers::parser_line::line_to_tokens(line);
}


/// Run a command or a pipeline.
///
/// # Example
///
/// File content of src/main.rs:
///
/// ```rust,no-run
/// extern crate cicada;
///
/// fn main() {
///     let out1 = cicada::run("ls");
///     println!("out1: {:?}", out1);
///
///     let out2 = cicada::run("ls | wc");
///     println!("out2: {:?}", out2);
///
///     let out3 = cicada::run("date >> out.txt");
///     println!("out3: {:?}", out3);
///
///     let out4 = cicada::run("cat out.txt");
///     println!("out4: {:?}", out4);
/// }
/// ```
///
/// Output:
///
/// ```no-run
/// out1: Ok("Cargo.lock\nCargo.toml\nsrc\ntarget\n")
/// out2: Ok("       4       4      33\n")
/// out3: Ok("")
/// out4: Ok("Fri Oct  6 14:53:25 CST 2017\n")
/// ```

pub fn run(line: &str) -> Result<String, &str> {
    let mut envs = HashMap::new();
    let cmd_line = tools::remove_envs_from_line(line, &mut envs);

    let mut tokens = parsers::parser_line::line_to_tokens(&cmd_line);
    if tokens.is_empty() {
        return Ok(String::new());
    }

    let mut len = tokens.len();
    if len > 1 && tokens[len - 1].1 == "&" {
        tokens.pop();
        len -= 1;
    }
    let mut redirect_from = String::new();
    let has_redirect_from = tokens.iter().any(|x| x.1 == "<");
    if has_redirect_from {
        if let Some(idx) = tokens.iter().position(|x| x.1 == "<") {
            tokens.remove(idx);
            len -= 1;
            if len >= idx + 1 {
                redirect_from = tokens.remove(idx).1;
                len -= 1;
            } else {
                return Err("cicada: invalid command: cannot get redirect from");
            }
        }
    }
    if len == 0 {
        return Ok(String::new());
    }

    let (_, _, output) =
        if len > 2 && (tokens[len - 2].1 == ">" || tokens[len - 2].1 == ">>") {
            let append = tokens[len - 2].1 == ">>";
            let redirect_to;
            match tokens.pop() {
                Some(x) => redirect_to = x.1,
                None => {
                    return Err("cicada: redirect_to pop error");
                }
            }
            tokens.pop(); // pop '>>' or '>'
            execute::run_pipeline(
                tokens,
                redirect_from.as_str(),
                redirect_to.as_str(),
                append,
                false,
                false,
                true,
                Some(envs),
            )
        } else {
            execute::run_pipeline(
                tokens.clone(),
                redirect_from.as_str(),
                "",
                false,
                false,
                false,
                true,
                Some(envs),
            )
        };

    match output {
        Some(x) => {
            return Ok(String::from_utf8_lossy(&x.stdout).into_owned());
        }
        None => {
            return Err("no output");
        }
    }
}
