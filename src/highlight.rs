use std::ops::Range;
use std::sync::Arc;
use std::collections::HashSet;
use std::path::Path;
use std::env;
use std::fs;
use std::sync::Mutex;
use std::os::unix::fs::PermissionsExt;

use linefeed::highlighting::{Highlighter, Style};

use crate::tools;
use crate::shell;

#[derive(Clone)]
pub struct CicadaHighlighter;

// ANSI color codes wrapped with \x01 and \x02 for linefeed
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

        // Check if this is an exact match for a known command, builtin, or alias
        let is_exact_command = if let Ok(commands) = AVAILABLE_COMMANDS.lock() {
            commands.contains(command)
        } else {
            false
        };

        let is_builtin = tools::is_builtin(command);

        let is_alias = if let Ok(aliases) = ALIASES.lock() {
            aliases.contains(command)
        } else {
            false
        };

        if is_exact_command || is_builtin || is_alias {
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
