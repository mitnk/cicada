//! Provides the `Function` trait for implementing custom `Prompter` commands

use std::io;

use crate::command::Category;
use crate::prompter::Prompter;
use crate::terminal::Terminal;

/// Implements custom functionality for a `Prompter` command
pub trait Function<Term: Terminal>: Send + Sync {
    /// Executes the function.
    ///
    /// `count` is the numerical argument supplied by the user; `1` by default.
    /// `prompter.explicit_arg()` may be called to determine whether this value
    /// was explicitly supplied by the user.
    ///
    /// `ch` is the final character of the sequence that triggered the command.
    /// `prompter.sequence()` may be called to determine the full sequence that
    /// triggered the command.
    fn execute(&self, prompter: &mut Prompter<Term>, count: i32, ch: char) -> io::Result<()>;

    /// Returns the command category.
    fn category(&self) -> Category { Category::Other }
}

impl<F, Term: Terminal> Function<Term> for F where
        F: Send + Sync,
        F: Fn(&mut Prompter<Term>, i32, char) -> io::Result<()> {
    fn execute(&self, prompter: &mut Prompter<Term>, count: i32, ch: char) -> io::Result<()> {
        self(prompter, count, ch)
    }
}
