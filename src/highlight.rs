use std::ops::Range;
use std::sync::Arc;

use linefeed::highlighting::{Highlighter, Style};

#[derive(Clone)]
pub struct CicadaHighlighter;

// ANSI color codes wrapped with \x01 and \x02 for linefeed
const GREEN: &str = "\x01\x1b[0;32m\x02";

// Common shell commands for testing
const COMMON_COMMANDS: &[&str] = &[
    "ls", "cd", "pwd", "echo", "cat", "grep", "find",
    "git", "rm", "cp", "mv", "mkdir", "touch"
];

impl Highlighter for CicadaHighlighter {
    fn highlight(&self, line: &str) -> Vec<(Range<usize>, Style)> {
        let mut styles = Vec::new();
        if line.is_empty() {
            return styles;
        }

        // Trim leading whitespace and find the start index of the actual command
        let trimmed_line = line.trim_start();
        let leading_whitespace_len = line.len() - trimmed_line.len();

        if trimmed_line.is_empty() {
            styles.push((0..line.len(), Style::Default));
            return styles;
        }

        // Find where the command ends in the trimmed line
        let first_word_end_in_trimmed = trimmed_line.find(char::is_whitespace).unwrap_or(trimmed_line.len());

        // Calculate the actual start and end indices in the original line
        let command_start_index = leading_whitespace_len;
        let command_end_index = leading_whitespace_len + first_word_end_in_trimmed;

        // Get the command part from the trimmed line
        let command = &trimmed_line[..first_word_end_in_trimmed];

        // Check if this is an exact match for a known command
        let is_exact_command = COMMON_COMMANDS.contains(&command);

        if is_exact_command {
            // Style leading whitespace (if any) as default
            if command_start_index > 0 {
                styles.push((0..command_start_index, Style::Default));
            }

            // Style the command green
            styles.push((
                command_start_index..command_end_index,
                Style::AnsiColor(GREEN.to_string()),
            ));

            // Style the rest of the line as default if there is more text
            if command_end_index < line.len() {
                styles.push((command_end_index..line.len(), Style::Default));
            }
        } else {
            // If it's not a command, style the whole line as default
            styles.push((0..line.len(), Style::Default));
        }

        styles
    }
}

pub fn create_highlighter() -> Arc<CicadaHighlighter> {
    Arc::new(CicadaHighlighter)
}
