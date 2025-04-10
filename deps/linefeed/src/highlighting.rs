//! Syntax highlighting functionality for the terminal interface

use std::ops::Range;

/// Represents a style to be applied to a text range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Style {
    /// A style using raw ANSI color codes
    AnsiColor(String),
    /// The default terminal style
    Default,
}

/// A trait for providing style information for a line of text.
pub trait Highlighter {
    /// Takes the current line buffer and returns a list of styled ranges.
    fn highlight(&self, line: &str) -> Vec<(Range<usize>, Style)>;
}
