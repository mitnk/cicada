//! Cicada is a bash-like Unix shell written in Rust.
//!
//! If you would like to use cicada as a regular shell,
//! please see details in [its repository](https://github.com/mitnk/cicada)
//!
//! Here is how to use cicada as a library:
//!
//! **Add cicada into Cargo.toml**
//!
//! ```ignore
//! [dependencies]
//! cicada = "1.0"
//! ```
//!
//! **Use cicada functions**
//!
//! ```no_run
//! extern crate cicada;
//!
//! fn main() {
//!     let info = cicada::parse_line("echo 'hi yoo' | `which wc`");
//!     assert!(info.is_complete);
//!
//!     let tokens = info.tokens;
//!     assert_eq!(tokens.len(), 4);
//!
//!     assert_eq!(tokens[0].0, "");
//!     assert_eq!(tokens[0].1, "echo");
//!
//!     assert_eq!(tokens[1].0, "'");
//!     assert_eq!(tokens[1].1, "hi yoo");
//!
//!     assert_eq!(tokens[2].0, "");
//!     assert_eq!(tokens[2].1, "|");
//!
//!     assert_eq!(tokens[3].0, "`");
//!     assert_eq!(tokens[3].1, "which wc");
//!
//!     let out1 = cicada::run("ls Cargo.toml foo");
//!     assert_eq!(out1.status, 1);
//!     assert_eq!(out1.stdout, "Cargo.toml\n");
//!     assert_eq!(out1.stderr, "ls: foo: No such file or directory\n");
//!
//!     let out2 = cicada::run("ls | wc");
//!     assert_eq!(out2.status, 0);
//!     assert_eq!(out2.stdout, "       4       4      33\n");
//! }
//! ```
//!
#![allow(dead_code)]
#![allow(unknown_lints)]
// #![feature(tool_lints)]
extern crate errno;
extern crate exec;
extern crate glob;
extern crate libc;
extern crate lineread;
extern crate nix;
extern crate regex;
extern crate rusqlite;

#[macro_use]
extern crate lazy_static;
extern crate pest;
#[macro_use]
extern crate pest_derive;

mod ctime;
mod types;

#[macro_use]
mod tlog;
#[macro_use]
mod tools;

mod builtins;
mod calculator;
mod core;
mod execute;
mod history;
mod jobc;
mod libs;
mod parsers;
mod rcfile;
mod scripting;
mod shell;
mod signals;

/// Represents an error calling `exec`.
pub use crate::types::CommandResult;
pub use crate::types::LineInfo;

/// Parse a command to tokens.
pub fn parse_line(cmd: &str) -> LineInfo {
    parsers::parser_line::parse_line(cmd)
}

/// Run a command or a pipeline.
pub fn run(line: &str) -> CommandResult {
    execute::run(line)
}
