#![allow(dead_code)]
#![allow(unknown_lints)]
// #![feature(tool_lints)]
extern crate errno;
extern crate exec;
extern crate glob;
extern crate libc;
extern crate linefeed;
extern crate nix;
extern crate regex;
extern crate sqlite;
extern crate time;

#[macro_use]
extern crate nom;

mod types;

#[macro_use]
mod tools;

mod builtins;
mod execute;
mod history;
mod jobc;
mod libs;
mod parsers;
mod shell;

use types::CommandResult;
use types::Tokens;

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

/// Parse a command to tokens.
///
/// # Examples
///
/// ```no-run
/// >>> cmd_to_tokens("echo 'hi yoo' | `which wc`");
/// vec![
///     ("", "echo"),
///     ("'", "hi yoo"),
///     ("", "|"),
///     ("`", "which wc"),
/// ]
/// ```
pub fn cmd_to_tokens(cmd: &str) -> Tokens {
    return parsers::parser_line::cmd_to_tokens(cmd);
}

/// Determine whether line a valid input.
///
/// # Examples
///
/// ```no-run
/// is_valid_input("foo");  // true
/// is_valid_input("foo bar");  // true
/// is_valid_input("foo ;");  // true
/// is_valid_input("ls | wc -l");  // true
/// is_valid_input("foo; bar");  // true
/// is_valid_input("foo || bar");  // true
///
/// is_valid_input("foo |");  // false
/// is_valid_input("foo ||");  // false
/// is_valid_input("foo &&");  // false
/// is_valid_input("foo || && bar ");  // false
/// ```
pub fn is_valid_input(line: &str) -> bool {
    return parsers::parser_line::is_valid_input(line);
}

/// Run a command or a pipeline.
///
/// # Example
///
/// File content of src/main.rs:
///
/// ```no-run
/// extern crate cicada;
///
/// fn main() {
///     let out1 = cicada::run("ls").unwrap();
///     println!("out1: {:?}", out1.stdout);
///
///     let out2 = cicada::run("ls | wc").unwrap();
///     println!("out2: {:?}", out2.stdout);
///
///     let out3 = cicada::run("date >> out.txt").unwrap();
///     println!("out3: {:?}", out3.stdout);
///
///     let out4 = cicada::run("cat out.txt").unwrap();
///     println!("out4: {:?}", out4.stdout);
/// }
/// ```
///
/// Output:
///
/// ```no-run
/// out1: "Cargo.lock\nCargo.toml\nsrc\ntarget\n"
/// out2: "       4       4      33\n"
/// out3: ""
/// out4: "Fri Oct  6 14:53:25 CST 2017\n"
/// ```
pub fn run(line: &str) -> CommandResult {
    execute::run(line)
}
