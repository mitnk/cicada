use std::collections::HashSet;
use std::env;
use std::fs;
use std::ops::Range;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use lineread::highlighting::{Highlighter, Style};

use crate::parsers::parser_line;
use crate::shell;
use crate::tools;

#[derive(Clone)]
pub struct CicadaHighlighter;

// ANSI color codes wrapped with \x01 and \x02 for lineread
const GREEN: &str = "\x01\x1b[0;32m\x02";

lazy_static! {
    static ref AVAILABLE_COMMANDS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    static ref ALIASES: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

/// Initialize the available commands cache by scanning PATH directories
pub fn init_command_cache() {
    let commands = scan_available_commands();
    if let Ok(mut cache) = AVAILABLE_COMMANDS.lock() {
        *cache = commands;
    }
}

/// Update aliases in the highlighter's cache
pub fn update_aliases(sh: &shell::Shell) {
    if let Ok(mut aliases) = ALIASES.lock() {
        aliases.clear();
        for alias_name in sh.aliases.keys() {
            aliases.insert(alias_name.clone());
        }
    }
}

fn scan_available_commands() -> HashSet<String> {
    let mut commands = HashSet::new();

    if let Ok(path_var) = env::var("PATH") {
        for path in path_var.split(':') {
            if path.is_empty() {
                continue;
            }

            let dir_path = Path::new(path);
            if !dir_path.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(dir_path) {
                for entry in entries.filter_map(Result::ok) {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() || file_type.is_symlink() {
                            if let Ok(metadata) = entry.metadata() {
                                // Check if file is executable
                                if metadata.permissions().mode() & 0o111 != 0 {
                                    if let Some(name) = entry.file_name().to_str() {
                                        commands.insert(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    commands
}

fn is_command(word: &str) -> bool {
    if tools::is_builtin(word) {
        return true;
    }
    if let Ok(aliases) = ALIASES.lock() {
        if aliases.contains(word) {
            return true;
        }
    }
    if let Ok(commands) = AVAILABLE_COMMANDS.lock() {
        if commands.contains(word) {
            return true;
        }
    }
    false
}

fn find_token_range_heuristic(
    line: &str,
    start_byte: usize,
    token: &(String, String),
) -> Option<Range<usize>> {
    let (sep, word) = token;

    // Find the start of the token, skipping leading whitespace from the search start position
    let mut search_area = &line[start_byte..];
    let token_start_byte =
        if let Some(non_ws_offset) = search_area.find(|c: char| !c.is_whitespace()) {
            // Calculate the actual byte index of the first non-whitespace character
            start_byte
                + search_area
                    .char_indices()
                    .nth(non_ws_offset)
                    .map_or(0, |(idx, _)| idx)
        } else {
            return None; // Only whitespace left
        };

    search_area = &line[token_start_byte..];

    // Estimate the end byte based on the token structure
    let mut estimated_len = 0;
    let mut current_search_offset = 0;

    // Match separator prefix if needed (e.g., `"` or `'`)
    if !sep.is_empty() && search_area.starts_with(sep) {
        estimated_len += sep.len();
        current_search_offset += sep.len();
    }

    // Match the word content
    // Use starts_with for a basic check, assuming the word appears next
    if search_area[current_search_offset..].starts_with(word) {
        estimated_len += word.len();
        current_search_offset += word.len();

        // Match separator suffix if needed
        if !sep.is_empty() && search_area[current_search_offset..].starts_with(sep) {
            estimated_len += sep.len();
        }

        Some(token_start_byte..(token_start_byte + estimated_len))
    } else if word.is_empty()
        && !sep.is_empty()
        && search_area.starts_with(sep)
        && search_area[sep.len()..].starts_with(sep)
    {
        // Handle empty quoted string like "" or ''
        estimated_len += sep.len() * 2;
        Some(token_start_byte..(token_start_byte + estimated_len))
    } else {
        // Fallback: Maybe it's just the word without quotes, or a separator like `|`
        if search_area.starts_with(word) {
            Some(token_start_byte..(token_start_byte + word.len()))
        } else {
            // Could not reliably map the token back to the original string segment
            // This might happen with complex escapes or parser ambiguities
            // As a basic fallback, consume up to the next space or end of line? Unsafe.
            // Return None to signal failure for this token.
            None
        }
    }
}

impl Highlighter for CicadaHighlighter {
    fn highlight(&self, line: &str) -> Vec<(Range<usize>, Style)> {
        let mut styles = Vec::new();
        if line.is_empty() {
            return styles;
        }

        let line_info = parser_line::parse_line(line);
        if line_info.tokens.is_empty() {
            // If parser returns no tokens, style whole line as default
            styles.push((0..line.len(), Style::Default));
            return styles;
        }

        let mut current_byte_idx = 0;
        let mut is_start_of_segment = true;

        for token in &line_info.tokens {
            // Find the range in the original line for this token
            match find_token_range_heuristic(line, current_byte_idx, token) {
                Some(token_range) => {
                    // Style potential whitespace before the token
                    if token_range.start > current_byte_idx {
                        styles.push((current_byte_idx..token_range.start, Style::Default));
                    }

                    let (_sep, word) = token;
                    let mut current_token_style = Style::Default;

                    if is_start_of_segment && !word.is_empty() {
                        if is_command(word) {
                            current_token_style = Style::AnsiColor(GREEN.to_string());
                        }
                        // Only the first non-empty token in a segment can be a command
                        is_start_of_segment = false;
                    }

                    styles.push((token_range.clone(), current_token_style));

                    // Check if this token marks the end of a command segment
                    if ["|", "&&", "||", ";"].contains(&word.as_str()) {
                        is_start_of_segment = true;
                    }

                    current_byte_idx = token_range.end;
                }
                None => {
                    // If we can't map a token, style the rest of the line as default and stop.
                    if current_byte_idx < line.len() {
                        styles.push((current_byte_idx..line.len(), Style::Default));
                    }
                    current_byte_idx = line.len(); // Mark as done
                    break; // Stop processing further tokens
                }
            }
        }

        // Style any remaining characters after the last processed token
        if current_byte_idx < line.len() {
            styles.push((current_byte_idx..line.len(), Style::Default));
        }

        styles
    }
}

pub fn create_highlighter() -> Arc<CicadaHighlighter> {
    Arc::new(CicadaHighlighter)
}
