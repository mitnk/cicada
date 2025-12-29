/// Command wrappers: commands that execute another command
pub const WRAPPER_COMMANDS: &[&str] = &[
    "builtin",
    "caffeinate",
    "check",
    "command",
    "env",
    "exec",
    "ionice",
    "nice",
    "nohup",
    "sudo",
    "time",
    "timeout",
    "which",
    "xargs",
];

/// Check if a word is a command wrapper
pub fn is_wrapper_command(word: &str) -> bool {
    if WRAPPER_COMMANDS.contains(&word) {
        return true;
    }

    if let Ok(extra) = std::env::var("CICADA_CMD_WRAPPERS") {
        for cmd in extra.split(':') {
            if cmd == word {
                return true;
            }
        }
    }
    false
}

/// Check if a word looks like an environment variable assignment (VAR=value)
pub fn is_env_assignment(word: &str) -> bool {
    // Must not start with '-' (that would be a flag like --foo=bar)
    if word.starts_with('-') || !word.contains('=') {
        return false;
    }
    let eq_pos = word.find('=').unwrap();
    let var_name = &word[..eq_pos];
    if var_name.is_empty() {
        return false;
    }
    let mut chars = var_name.chars();
    // First char must be letter or underscore
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Get the current command segment (after the last |, &&, ||, ;)
/// This handles quoted strings properly.
pub fn get_current_segment(line: &str) -> &str {
    let mut last_sep_end = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut prev_char = '\0';
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        let c = chars[i];

        // Handle escapes outside quotes
        if prev_char == '\\' && !in_single_quote {
            prev_char = '\0'; // Reset to avoid double-escape issues
            i += 1;
            continue;
        }

        // Track quote state
        if c == '\'' && !in_double_quote && !in_backtick && prev_char != '\\' {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote && !in_backtick && prev_char != '\\' {
            in_double_quote = !in_double_quote;
        } else if c == '`' && !in_single_quote && prev_char != '\\' {
            in_backtick = !in_backtick;
        }

        // Only look for separators outside quotes
        if !in_single_quote && !in_double_quote && !in_backtick {
            // Check for || or &&
            if i + 1 < len {
                let next = chars[i + 1];
                if (c == '|' && next == '|') || (c == '&' && next == '&') {
                    last_sep_end = i + 2;
                    i += 2;
                    prev_char = next;
                    continue;
                }
            }
            // Check for single | (but not ||) or ;
            if c == '|' || c == ';' {
                last_sep_end = i + 1;
            }
        }

        prev_char = c;
        i += 1;
    }

    &line[last_sep_end..]
}

/// Get the effective command from a line, skipping command wrappers and
/// env assignments. Returns the command name that should be used for
/// completion/highlighting decisions.
pub fn get_effective_command(line: &str) -> Option<String> {
    let segment = get_current_segment(line);
    let tokens = crate::parsers::parser_line::line_to_plain_tokens(segment);

    let mut after_wrapper = false;
    let mut skip_next = false;

    for token in &tokens {
        // Skip token if it was flagged as an option argument
        if skip_next {
            skip_next = false;
            continue;
        }
        // Skip empty tokens
        if token.is_empty() {
            continue;
        }
        // Skip environment assignments
        if is_env_assignment(token) {
            continue;
        }
        // Skip command wrappers
        if is_wrapper_command(token) {
            after_wrapper = true;
            continue;
        }
        // After command wrappers, skip options and their arguments
        if after_wrapper && token.starts_with('-') {
            // Single-letter options like -u often take an argument
            // Skip the next token as a potential argument
            if token.len() == 2 && !token.starts_with("--") {
                skip_next = true;
            }
            continue;
        }
        // Found the effective command
        return Some(token.clone());
    }

    // If we only found command wrappers, return the last one
    tokens
        .into_iter()
        .rev()
        .find(|token| !token.is_empty() && !is_env_assignment(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_env_assignment_valid() {
        assert!(is_env_assignment("FOO=bar"));
        assert!(is_env_assignment("_VAR=1"));
        assert!(is_env_assignment("MY_VAR="));
        assert!(is_env_assignment("a=b"));
        assert!(is_env_assignment("ABC123=xyz"));
    }

    #[test]
    fn test_is_env_assignment_invalid() {
        assert!(!is_env_assignment("-foo=bar"));
        assert!(!is_env_assignment("--opt=val"));
        assert!(!is_env_assignment("=value"));
        assert!(!is_env_assignment("123=foo"));
        assert!(!is_env_assignment("FOO-BAR=x"));
        assert!(!is_env_assignment("FOO"));
        assert!(!is_env_assignment(""));
    }

    #[test]
    fn test_is_wrapper_command() {
        assert!(is_wrapper_command("sudo"));
        assert!(is_wrapper_command("xargs"));
        assert!(is_wrapper_command("nohup"));
        assert!(is_wrapper_command("env"));
        assert!(!is_wrapper_command("ls"));
        assert!(!is_wrapper_command("grep"));
        assert!(!is_wrapper_command(""));
    }

    #[test]
    fn test_get_current_segment() {
        assert_eq!(get_current_segment("ls foo"), "ls foo");
        assert_eq!(get_current_segment("ls | grep bar"), " grep bar");
        assert_eq!(get_current_segment("ls && echo done"), " echo done");
        assert_eq!(get_current_segment("ls || echo fail"), " echo fail");
        assert_eq!(get_current_segment("ls; pwd"), " pwd");
        assert_eq!(get_current_segment("ls | grep foo | wc -l"), " wc -l");
        assert_eq!(
            get_current_segment("echo \"foo | bar\" | grep x"),
            " grep x"
        );
        assert_eq!(get_current_segment("echo 'foo | bar' | grep x"), " grep x");
    }

    #[test]
    fn test_get_effective_command() {
        assert_eq!(get_effective_command("ls foo"), Some("ls".to_string()));
        assert_eq!(get_effective_command("sudo ls foo"), Some("ls".to_string()));
        assert_eq!(
            get_effective_command("sudo -u root ls foo"),
            Some("ls".to_string())
        );
        assert_eq!(
            get_effective_command("FOO=1 echo hi"),
            Some("echo".to_string())
        );
        assert_eq!(
            get_effective_command("FOO=1 BAR=2 ls"),
            Some("ls".to_string())
        );
        assert_eq!(
            get_effective_command("env FOO=1 ls"),
            Some("ls".to_string())
        );
        assert_eq!(
            get_effective_command("ls | xargs rm"),
            Some("rm".to_string())
        );
        assert_eq!(
            get_effective_command("ls | sudo xargs rm"),
            Some("rm".to_string())
        );
        assert_eq!(
            get_effective_command("cd foo && make"),
            Some("make".to_string())
        );
        // When only command wrappers, return the last one
        assert_eq!(get_effective_command("sudo"), Some("sudo".to_string()));
        assert_eq!(
            get_effective_command("sudo nohup"),
            Some("nohup".to_string())
        );
    }
}
