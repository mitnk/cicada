#![allow(dead_code)]
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

#[macro_use]
mod tools;

mod shell;
mod libs;
mod history;
mod builtins;
mod execute;
mod parsers;

use tools::CommandResult;

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
pub fn cmd_to_tokens(cmd: &str) -> Vec<(String, String)> {
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
pub fn run(line: &str) -> Result<CommandResult, &str> {
    execute::run(line)
}
