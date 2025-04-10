//! Defines the set of line editing commands

use std::borrow::Cow::{self, Borrowed, Owned};
use std::fmt;

use crate::chars::escape_sequence;

macro_rules! define_commands {
    ( $( #[$meta:meta] $name:ident => $str:expr , )+ ) => {
        /// Represents a command to modify `Reader` state
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum Command {
            $( #[$meta] $name , )+
            /// Custom application-defined command
            Custom(Cow<'static, str>),
            /// Execute a given key sequence
            Macro(Cow<'static, str>),
        }

        /// List of all command names
        pub static COMMANDS: &[&str] = &[ $( $str ),+ ];

        impl fmt::Display for Command {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match *self {
                    $( Command::$name => f.write_str($str) , )+
                    Command::Custom(ref s) => f.write_str(s),
                    Command::Macro(ref s) => write!(f, "\"{}\"",
                        escape_sequence(s))
                }
            }
        }

        impl Command {
            /// Constructs a command from a `'static str` reference.
            ///
            /// If the string does not refer to a built-in command, a value
            /// of `Command::Custom(Borrowed(name))` will be returned.
            pub fn from_str(name: &'static str) -> Command {
                Command::opt_from_str(name)
                    .unwrap_or_else(|| Command::Custom(Borrowed(name)))
            }

            /// Constructs a command from a non-`'static` string-like type.
            ///
            /// If the string does not refer to a built-in command, a value
            /// of `Command::Custom(Owned(name.into()))` will be returned.
            pub fn from_string<T>(name: T) -> Command
                    where T: AsRef<str> + Into<String> {
                Command::opt_from_str(name.as_ref())
                    .unwrap_or_else(|| Command::Custom(Owned(name.into())))
            }

            fn opt_from_str(s: &str) -> Option<Command> {
                match s {
                    $( $str => Some(Command::$name), )+
                    _ => None
                }
            }
        }
    }
}

define_commands!{
    /// Abort history search
    Abort => "abort",
    /// Accepts the current input line
    AcceptLine => "accept-line",
    /// Perform completion
    Complete => "complete",
    /// Insert all completions into the input buffer
    InsertCompletions => "insert-completions",
    /// Show possible completions
    PossibleCompletions => "possible-completions",
    /// Insert the next possible completion
    MenuComplete => "menu-complete",
    /// Insert the previous possible completion
    MenuCompleteBackward => "menu-complete-backward",
    /// Begin numeric argument input
    DigitArgument => "digit-argument",
    /// Insert character or sequence at the cursor
    SelfInsert => "self-insert",
    /// Inserts a tab character
    TabInsert => "tab-insert",
    /// Toggles insert/overwrite mode
    OverwriteMode => "overwrite-mode",
    /// Insert a comment and accept input
    InsertComment => "insert-comment",
    /// Move the cursor backward one character
    BackwardChar => "backward-char",
    /// Move the cursor forward one character
    ForwardChar => "forward-char",
    /// Search for a given character
    CharacterSearch => "character-search",
    /// Search backward for a given character
    CharacterSearchBackward => "character-search-backward",
    /// Move the cursor backward one word
    BackwardWord => "backward-word",
    /// Move the cursor forward one word
    ForwardWord => "forward-word",
    /// Kill all characters before the cursor
    BackwardKillLine => "backward-kill-line",
    /// Kill all characters after the cursor
    KillLine => "kill-line",
    /// Kill a word before the cursor
    BackwardKillWord => "backward-kill-word",
    /// Kill a word after the cursor
    KillWord => "kill-word",
    /// Kill a word before the cursor, delimited by whitespace
    UnixWordRubout => "unix-word-rubout",
    /// Clear the screen
    ClearScreen => "clear-screen",
    /// Move the cursor to the beginning of the line
    BeginningOfLine => "beginning-of-line",
    /// Move the cursor to the end of the line
    EndOfLine => "end-of-line",
    /// Delete one character before the cursor
    BackwardDeleteChar => "backward-delete-char",
    /// Delete one character after the cursor
    DeleteChar => "delete-char",
    /// Drag the character before the cursor forward
    TransposeChars => "transpose-chars",
    /// Drag the word before the cursor forward
    TransposeWords => "transpose-words",
    /// Move to the first line of history
    BeginningOfHistory => "beginning-of-history",
    /// Move to the last line of history
    EndOfHistory => "end-of-history",
    /// Select next line in history
    NextHistory => "next-history",
    /// Select previous line in history
    PreviousHistory => "previous-history",
    /// Incremental search in history
    ForwardSearchHistory => "forward-search-history",
    /// Incremental reverse search in history
    ReverseSearchHistory => "reverse-search-history",
    /// Non-incremental forward history search using input up to the cursor
    HistorySearchForward => "history-search-forward",
    /// Non-incremental backward history search using input up to the cursor
    HistorySearchBackward => "history-search-backward",
    /// Insert literal character
    QuotedInsert => "quoted-insert",
    /// Insert text into buffer from the kill ring
    Yank => "yank",
    /// Rotate the kill ring and yank the new top
    YankPop => "yank-pop",
}

/// Describes the category of a command
///
/// A command's category determines how particular operations behave
/// in succession.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Category {
    /// Completion command
    Complete,
    /// Kill command
    Kill,
    /// Non-incremental search command
    Search,
    /// Incremental search command
    IncrementalSearch,
    /// Yank command
    Yank,
    /// Digit argument command
    Digit,
    /// Other command
    Other,
}

impl Command {
    /// Returns the category of the command
    pub fn category(&self) -> Category {
        use self::Command::*;

        match *self {
            DigitArgument => Category::Digit,
            Complete | InsertCompletions | PossibleCompletions |
                MenuComplete | MenuCompleteBackward => Category::Complete,
            BackwardKillLine | KillLine | BackwardKillWord | KillWord |
                UnixWordRubout => Category::Kill,
            ForwardSearchHistory | ReverseSearchHistory => Category::IncrementalSearch,
            HistorySearchForward | HistorySearchBackward => Category::Search,
            Yank | YankPop => Category::Yank,
            _ => Category::Other
        }
    }
}

impl Default for Command {
    fn default() -> Self {
        Command::Custom(Borrowed(""))
    }
}
