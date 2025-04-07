//! Provides utilities for implementing word completion

use std::borrow::Cow::{self, Borrowed, Owned};
use std::fs::read_dir;
use std::path::{is_separator, MAIN_SEPARATOR};

use crate::prompter::Prompter;
use crate::terminal::Terminal;

/// Represents a single possible completion
#[derive(Clone, Debug)]
pub struct Completion {
    /// Whole completion text
    pub completion: String,
    /// Listing display string; `None` if matches completion
    pub display: Option<String>,
    /// Completion suffix; replaces append character
    pub suffix: Suffix,
}

/// Specifies an optional suffix to override the default value
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Suffix {
    /// Use the default suffix
    Default,
    /// Use no suffix
    None,
    /// Use the given suffix
    Some(char),
}

impl Completion {
    /// Returns a simple `Completion` value, with display string matching
    /// completion and using the default completion suffix.
    pub fn simple(s: String) -> Completion {
        Completion{
            completion: s,
            display: None,
            suffix: Suffix::default(),
        }
    }

    /// Returns the full completion string, including suffix, using the given
    /// default suffix if one is not assigned to this completion.
    pub fn completion(&self, def_suffix: Option<char>) -> Cow<str> {
        let mut s = Borrowed(&self.completion[..]);

        if let Some(suffix) = self.suffix.with_default(def_suffix) {
            s.to_mut().push(suffix);
        }

        s
    }

    /// Returns the display string, including suffix
    pub fn display(&self) -> Cow<str> {
        let mut s = Borrowed(self.display_str());

        if let Suffix::Some(suffix) = self.suffix {
            s.to_mut().push(suffix);
        }

        s
    }

    /// Returns the number of characters displayed
    pub fn display_chars(&self) -> usize {
        let n = self.display_str().chars().count();
        n + if self.suffix.is_some() { 1 } else { 0 }
    }

    fn display_str(&self) -> &str {
        match self.display {
            Some(ref dis) => dis,
            None => &self.completion
        }
    }
}

impl Suffix {
    /// Returns whether the `Suffix` value is the `Default` variant.
    pub fn is_default(&self) -> bool {
        match *self {
            Suffix::Default => true,
            _ => false
        }
    }

    /// Returns whether the `Suffix` value is the `Some(_)` variant.
    pub fn is_some(&self) -> bool {
        match *self {
            Suffix::Some(_) => true,
            _ => false
        }
    }

    /// Returns whether the `Suffix` value is the `None` variant.
    pub fn is_none(&self) -> bool {
        match *self {
            Suffix::None => true,
            _ => false
        }
    }

    /// Returns an `Option<char>`, using the given value in place of `Default`.
    pub fn with_default(self, default: Option<char>) -> Option<char> {
        match self {
            Suffix::None => None,
            Suffix::Some(ch) => Some(ch),
            Suffix::Default => default
        }
    }
}

impl Default for Suffix {
    fn default() -> Suffix {
        Suffix::Default
    }
}

/// Performs completion for `Prompter` when triggered by a user input sequence
pub trait Completer<Term: Terminal>: Send + Sync {
    /// Returns the set of possible completions for the prefix `word`.
    fn complete(&self, word: &str, prompter: &Prompter<Term>,
        start: usize, end: usize) -> Option<Vec<Completion>>;

    /// Returns the starting position of the word under the cursor.
    ///
    /// The default implementation uses `Prompter::word_break_chars()` to
    /// detect the start of a word.
    fn word_start(&self, line: &str, end: usize, prompter: &Prompter<Term>) -> usize {
        word_break_start(&line[..end], prompter.word_break_chars())
    }

    /// Quotes a possible completion for insertion into input.
    ///
    /// The default implementation returns the word, as is.
    fn quote<'a>(&self, word: &'a str) -> Cow<'a, str> { Borrowed(word) }

    /// Unquotes a piece of user input before searching for completions.
    ///
    /// The default implementation returns the word, as is.
    fn unquote<'a>(&self, word: &'a str) -> Cow<'a, str> { Borrowed(word) }
}

/// `Completer` type that performs no completion
///
/// This is the default `Completer` for a new `Prompter` instance.
pub struct DummyCompleter;

impl<Term: Terminal> Completer<Term> for DummyCompleter {
    fn complete(&self, _word: &str, _reader: &Prompter<Term>,
            _start: usize, _end: usize) -> Option<Vec<Completion>> { None }
}

/// Performs completion by searching for filenames matching the word prefix.
pub struct PathCompleter;

impl<Term: Terminal> Completer<Term> for PathCompleter {
    fn complete(&self, word: &str, _reader: &Prompter<Term>, _start: usize, _end: usize)
            -> Option<Vec<Completion>> {
        Some(complete_path(word))
    }

    fn word_start(&self, line: &str, end: usize, _reader: &Prompter<Term>) -> usize {
        escaped_word_start(&line[..end])
    }

    fn quote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        escape(word)
    }

    fn unquote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        unescape(word)
    }
}

/// Returns a sorted list of paths whose prefix matches the given path.
pub fn complete_path(path: &str) -> Vec<Completion> {
    let (base_dir, fname) = split_path(path);
    let mut res = Vec::new();

    let lookup_dir = base_dir.unwrap_or(".");

    if let Ok(list) = read_dir(lookup_dir) {
        for ent in list {
            if let Ok(ent) = ent {
                let ent_name = ent.file_name();

                // TODO: Deal with non-UTF8 paths in some way
                if let Ok(path) = ent_name.into_string() {
                    if path.starts_with(fname) {
                        let (name, display) = if let Some(dir) = base_dir {
                            (format!("{}{}{}", dir, MAIN_SEPARATOR, path),
                                Some(path))
                        } else {
                            (path, None)
                        };

                        let is_dir = ent.metadata().ok()
                            .map_or(false, |m| m.is_dir());

                        let suffix = if is_dir {
                            Suffix::Some(MAIN_SEPARATOR)
                        } else {
                            Suffix::Default
                        };

                        res.push(Completion{
                            completion: name,
                            display: display,
                            suffix: suffix,
                        });
                    }
                }
            }
        }
    }

    res.sort_by(|a, b| a.display_str().cmp(b.display_str()));
    res
}

/// Returns the start position of the word that ends at the end of the string.
pub fn word_break_start(s: &str, word_break: &str) -> usize {
    let mut start = s.len();

    for (idx, ch) in s.char_indices().rev() {
        if word_break.contains(ch) {
            break;
        }
        start = idx;
    }

    start
}

/// Returns the start position of a word with non-word characters escaped by
/// backslash (`\\`).
pub fn escaped_word_start(s: &str) -> usize {
    let mut chars = s.char_indices().rev();
    let mut start = s.len();

    while let Some((idx, ch)) = chars.next() {
        if needs_escape(ch) {
            let n = {
                let mut n = 0;

                loop {
                    let mut clone = chars.clone();

                    let ch = match clone.next() {
                        Some((_, ch)) => ch,
                        None => break
                    };

                    if ch == '\\' {
                        chars = clone;
                        n += 1;
                    } else {
                        break;
                    }
                }

                n
            };

            if n % 2 == 0 {
                break;
            }
        }

        start = idx;
    }

    start
}

/// Escapes a word by prefixing a backslash (`\\`) to non-word characters.
pub fn escape(s: &str) -> Cow<str> {
    let n = s.chars().filter(|&ch| needs_escape(ch)).count();

    if n == 0 {
        Borrowed(s)
    } else {
        let mut res = String::with_capacity(s.len() + n);

        for ch in s.chars() {
            if needs_escape(ch) {
                res.push('\\');
            }
            res.push(ch);
        }

        Owned(res)
    }
}

/// Unescapes a word by removing the backslash (`\\`) from escaped characters.
pub fn unescape(s: &str) -> Cow<str> {
    if s.contains('\\') {
        let mut res = String::with_capacity(s.len());
        let mut chars = s.chars();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(ch) = chars.next() {
                    res.push(ch);
                }
            } else {
                res.push(ch);
            }
        }

        Owned(res)
    } else {
        Borrowed(s)
    }
}

fn needs_escape(ch: char) -> bool {
    match ch {
        ' ' | '\t' | '\n' | '\\' => true,
        _ => false
    }
}

fn split_path(path: &str) -> (Option<&str>, &str) {
    match path.rfind(is_separator) {
        Some(pos) => (Some(&path[..pos]), &path[pos + 1..]),
        None => (None, path)
    }
}
