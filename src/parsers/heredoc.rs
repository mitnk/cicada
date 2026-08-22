use std::sync::atomic::{AtomicUsize, Ordering};

use crate::shell::{self, Shell};
use crate::types::Heredoc;

static HEREDOC_ID: AtomicUsize = AtomicUsize::new(0);

/// The name a heredoc body is parked under while the command is being built.
/// It carries the session id, so that the name cannot be guessed from outside
/// the shell.
fn next_marker(session_id: &str) -> String {
    let n = HEREDOC_ID.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}{}_{}",
        crate::types::HEREDOC_MARKER_PREFIX,
        session_id.replace('-', ""),
        n
    )
}

struct OpenHeredoc {
    marker: String,
    delimiter: String,
    quoted: bool,
    strip_tabs: bool,
    body: Vec<String>,
}

/// Lift the heredoc bodies out of `line`, so that what comes back is a plain
/// command line the rest of the shell already knows how to run: every
/// `<< EOF` becomes `<<< MARKER`, and the body it collected is parked in
/// `sh.heredocs` under that marker until the command is built.
pub fn preprocess(line: &str, sh: &mut Shell) -> Result<(String, Vec<String>), String> {
    if !line.contains("<<") {
        return Ok((line.to_string(), Vec::new()));
    }

    let mut out = String::new();
    let mut markers = Vec::new();
    let mut active: Option<OpenHeredoc> = None;

    for raw in line.split_inclusive('\n') {
        let (content, has_newline) = match raw.strip_suffix('\n') {
            Some(stripped) => (stripped, true),
            None => (raw, false),
        };

        if let Some(open) = active.as_mut() {
            if is_closing_line(content, open) {
                let body = open.body.join("\n");
                let body = if body.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", body)
                };
                markers.push(open.marker.clone());
                sh.heredocs.insert(
                    open.marker.clone(),
                    Heredoc {
                        body,
                        quoted: open.quoted,
                        transient: false,
                    },
                );
                active = None;
            } else {
                open.body
                    .push(strip_line_tabs(content, open.strip_tabs).to_string());
            }
        } else {
            let mut starts = Vec::new();
            let processed = scan_line(content, Some(&sh.session_id), &mut starts)?;
            out.push_str(&processed);
            if has_newline {
                out.push('\n');
            }

            if !starts.is_empty() {
                if starts.len() > 1 {
                    return Err("multiple heredocs on one line are not supported".to_string());
                }
                active = Some(starts.pop().unwrap());
            }
        }
    }

    if let Some(open) = active {
        // Nothing followed the head, so there was never a body to collect --
        // as happens inside `$(...)`, whose text is one line by construction
        // (`echo $((1 << 3))` opens no heredoc). Hand the line back as it was
        // and let the parser have it.
        if !line.contains('\n') {
            forget(sh, &markers);
            return Ok((line.to_string(), Vec::new()));
        }
        return Err(format!("heredoc `{}` not closed", open.delimiter));
    }

    Ok((out, markers))
}

/// Split an interactive buffer into the part the line parser should see and
/// whether a heredoc is still waiting for its delimiter.
///
/// Heredoc body lines are data, not shell syntax, so they are dropped from the
/// first half: an apostrophe in the body must not leave `parse_line()` waiting
/// for a closing quote that is never coming. Lines that are not body lines are
/// handed back untouched, sub-prompts included, so buffers without a heredoc
/// reach the parser exactly as they did before.
pub fn split_buffer(buf: &str) -> (String, bool) {
    let mut out = String::new();
    let mut active: Option<OpenHeredoc> = None;

    for (line_idx, raw) in buf.split_inclusive('\n').enumerate() {
        let content = raw.strip_suffix('\n').unwrap_or(raw);
        let stripped = if line_idx > 0 {
            strip_prompt(content)
        } else {
            content
        };

        if let Some(open) = active.as_mut() {
            if is_closing_line(stripped, open) {
                active = None;
            }
            continue;
        }

        out.push_str(raw);

        let mut starts = Vec::new();
        match scan_line(stripped, None, &mut starts) {
            Ok(_) => {
                if starts.len() == 1 {
                    active = Some(starts.pop().unwrap());
                }
            }
            // a malformed head (e.g. `cat <<`) is not going to become valid by
            // typing more body lines -- let the parser report it
            Err(_) => return (buf.to_string(), false),
        }
    }

    (out, active.is_some())
}

/// Rewrite the heredoc operators of a single command line into here-string
/// markers, pushing what it learned about each heredoc onto `starts`.
///
/// Callers that only want to know whether a heredoc opens here pass no
/// session: the scan then costs nothing that outlives it, which matters
/// because the interactive prompt runs it on every Enter.
fn scan_line(
    line: &str,
    session_id: Option<&str>,
    starts: &mut Vec<OpenHeredoc>,
) -> Result<String, String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut quote: Option<char> = None;

    while i < chars.len() {
        let c = chars[i];

        if let Some(q) = quote {
            out.push(c);
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        if c == '\\' {
            out.push(c);
            if i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // a `#` where a word could start comments out the rest of the line,
        // so a `<<` behind it is prose, not an operator
        if c == '#' && (out.is_empty() || out.ends_with([' ', '\t'])) {
            out.push_str(&chars[i..].iter().collect::<String>());
            break;
        }

        // `$(...)` and `$((...))` are copied over whole: the `<<` in
        // `echo $((1 << 3))` is a shift, not a heredoc
        if c == '$' && chars.get(i + 1) == Some(&'(') {
            let end = skip_parens(&chars, i + 1);
            out.push_str(&chars[i..end].iter().collect::<String>());
            i = end;
            continue;
        }

        if c == '\'' || c == '"' || c == '`' {
            quote = Some(c);
            out.push(c);
            i += 1;
            continue;
        }

        if c == '<'
            && chars.get(i + 1) == Some(&'<')
            && (i == 0 || chars[i - 1] != '<')
            && chars.get(i + 2) != Some(&'<')
        {
            let (op_len, strip_tabs) = if chars.get(i + 2) == Some(&'-') {
                (3, true)
            } else {
                (2, false)
            };

            let mut j = i + op_len;
            while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
                j += 1;
            }
            if j >= chars.len() {
                return Err("missing heredoc delimiter".to_string());
            }

            let (delimiter, quoted, next_j) = read_delimiter(&chars, j)?;
            let marker = match session_id {
                Some(session_id) => next_marker(session_id),
                None => String::new(),
            };

            if !out.is_empty() && !out.chars().last().unwrap().is_whitespace() {
                out.push(' ');
            }
            // written in single quotes: the quote marker is what tells
            // `Command::from_tokens` that this word is the preprocessor's own
            // and not the value of some expansion
            out.push_str("<<< '");
            out.push_str(&marker);
            out.push('\'');
            if next_j < chars.len() && !chars[next_j].is_whitespace() {
                out.push(' ');
            }

            starts.push(OpenHeredoc {
                marker,
                delimiter,
                quoted,
                strip_tabs,
                body: Vec::new(),
            });

            i = next_j;
            continue;
        }

        out.push(c);
        i += 1;
    }

    Ok(out)
}

/// Index just past the `(` at `start` and its matching `)`. Runs off the end
/// of the line when the parens do not balance, which leaves the unbalanced
/// tail to the line parser.
fn skip_parens(chars: &[char], start: usize) -> usize {
    let mut j = start;
    let mut depth: usize = 0;
    let mut quote: Option<char> = None;

    while j < chars.len() {
        let c = chars[j];

        if c == '\\' && quote != Some('\'') {
            j += 2;
            continue;
        }

        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            quote = Some(c);
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            if depth <= 1 {
                return j + 1;
            }
            depth -= 1;
        }

        j += 1;
    }

    chars.len()
}

/// Read the delimiter word of a heredoc: its text with the quoting taken off,
/// whether any of it was quoted, and where the word ends.
///
/// Quoting *anywhere* in the word turns expansion off, and the quotes are not
/// part of the delimiter the body is matched against. So `<< \EOF`, `<< 'EOF'`
/// and `<< EO"F"` all wait for a line reading `EOF` and all pass the body
/// through untouched -- which is what `<< \EOF`, the usual way of asking for a
/// literal body, has to mean.
fn read_delimiter(chars: &[char], start: usize) -> Result<(String, bool, usize), String> {
    let mut delimiter = String::new();
    let mut quoted = false;
    let mut j = start;

    while j < chars.len() {
        let c = chars[j];

        if c == '\\' {
            match chars.get(j + 1) {
                Some(next) => {
                    delimiter.push(*next);
                    quoted = true;
                    j += 2;
                }
                None => return Err("heredoc delimiter ends with a backslash".to_string()),
            }
            continue;
        }

        if c == '\'' || c == '"' {
            quoted = true;
            j += 1;
            let mut closed = false;
            while j < chars.len() {
                if chars[j] == c {
                    closed = true;
                    j += 1;
                    break;
                }
                delimiter.push(chars[j]);
                j += 1;
            }
            if !closed {
                return Err("quoted heredoc delimiter is not closed".to_string());
            }
            continue;
        }

        if c.is_whitespace() || is_delim_metachar(c) {
            break;
        }

        delimiter.push(c);
        j += 1;
    }

    // `<< ''` is a heredoc that ends on an empty line; `<<` with nothing after
    // it is a missing delimiter
    if delimiter.is_empty() && !quoted {
        return Err("missing heredoc delimiter".to_string());
    }

    Ok((delimiter, quoted, j))
}

fn is_delim_metachar(c: char) -> bool {
    matches!(c, '|' | '&' | ';' | '<' | '>')
}

fn is_closing_line(line: &str, open: &OpenHeredoc) -> bool {
    let candidate = if open.strip_tabs {
        line.trim_start_matches('\t')
    } else {
        line
    };
    candidate == open.delimiter
}

fn strip_line_tabs(line: &str, strip_tabs: bool) -> &str {
    if strip_tabs {
        line.trim_start_matches('\t')
    } else {
        line
    }
}

fn strip_prompt(line: &str) -> &str {
    line.strip_prefix(">> ").unwrap_or(line)
}

/// Every heredoc marker mentioned in `line`.
pub fn markers_in_line(line: &str) -> Vec<String> {
    let prefix = crate::types::HEREDOC_MARKER_PREFIX;
    let mut markers = Vec::new();
    let mut rest = line;

    while let Some(pos) = rest.find(prefix) {
        let tail = &rest[pos..];
        let end = tail
            .char_indices()
            .find(|(i, c)| *i >= prefix.len() && !c.is_ascii_alphanumeric() && *c != '_')
            .map(|(i, _)| i)
            .unwrap_or(tail.len());
        markers.push(tail[..end].to_string());
        rest = &tail[end..];
    }

    markers
}

/// Rewrite the body of each heredoc `line` refers to with `expand`, and point
/// the line at the rewritten copies.
///
/// The stored body is left as it was typed: a heredoc inside a function body
/// is expanded again, against fresh arguments, every time the function runs.
/// The copies are marked transient so that running the command consumes them.
pub fn apply_to_bodies<F>(sh: &mut Shell, line: &str, expand: F) -> String
where
    F: Fn(&str) -> String,
{
    let mut line_new = line.to_string();

    for marker in markers_in_line(line) {
        let stored = match sh.heredocs.get(&marker) {
            // a quoted delimiter turns off every expansion, this one included
            Some(hd) if !hd.quoted && !hd.transient => hd.clone(),
            _ => continue,
        };

        let body = expand(&stored.body);
        if body == stored.body {
            continue;
        }

        let marker_new = next_marker(&sh.session_id);
        sh.heredocs.insert(
            marker_new.clone(),
            Heredoc {
                body,
                quoted: stored.quoted,
                transient: true,
            },
        );
        line_new = line_new.replace(&marker, &marker_new);
    }

    line_new
}

/// The body a marker stands for, expanded and ready to become the command's
/// stdin. Copies made for one run are dropped here; the bodies the
/// preprocessor parked stay put, so that a command inside a loop or a function
/// still has its heredoc on the second time round.
pub fn take_body(sh: &mut Shell, marker: &str) -> Option<String> {
    let stored = sh.heredocs.get(marker)?.clone();
    if stored.transient {
        sh.heredocs.remove(marker);
    }
    Some(expand_heredoc_body(sh, &stored.body, stored.quoted))
}

/// Forget the bodies behind `markers`, for a line that has finished running
/// and cannot come back (anything outside a script file).
pub fn forget(sh: &mut Shell, markers: &[String]) {
    for marker in markers {
        sh.heredocs.remove(marker);
    }
}

/// Drop a per-run copy whose command never ran, e.g. the right-hand side of a
/// `&&` that short-circuited.
pub fn forget_if_transient(sh: &mut Shell, marker: &str) {
    if let Some(hd) = sh.heredocs.get(marker) {
        if hd.transient {
            sh.heredocs.remove(marker);
        }
    }
}

pub fn expand_heredoc_body(sh: &mut Shell, body: &str, quoted: bool) -> String {
    if quoted {
        return body.to_string();
    }

    let chars: Vec<char> = body.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '\\' {
            i += 1;
            if i >= chars.len() {
                result.push('\\');
                break;
            }
            let next = chars[i];
            match next {
                '\n' => {
                    i += 1;
                    continue;
                }
                '$' | '`' | '\\' => {
                    result.push(next);
                    i += 1;
                    continue;
                }
                _ => {
                    result.push('\\');
                    result.push(next);
                    i += 1;
                    continue;
                }
            }
        }

        if c == '$' {
            if let Some(&'(') = chars.get(i + 1) {
                if let Some((cmd, next)) = find_dollar_cmdsub(&chars, i) {
                    result.push_str(&run_heredoc_command_substitution(sh, &cmd));
                    i = next;
                    continue;
                }
            }

            if let Some((name, next)) = shell::read_braced_name(&chars, i) {
                result.push_str(&shell::env_value_of(sh, &name));
                i = next;
                continue;
            }

            if let Some((name, next)) = shell::read_bare_name(&chars, i) {
                result.push_str(&shell::env_value_of(sh, &name));
                i = next;
                continue;
            }

            result.push('$');
            i += 1;
            continue;
        }

        if c == '`' {
            if let Some((cmd, next)) = find_backtick_cmdsub(&chars, i) {
                result.push_str(&run_heredoc_command_substitution(sh, &cmd));
                i = next;
                continue;
            }
        }

        result.push(c);
        i += 1;
    }

    result
}

fn find_dollar_cmdsub(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) != Some(&'$') || chars.get(start + 1) != Some(&'(') {
        return None;
    }

    let mut j = start + 2;
    let mut depth: usize = 0;
    let mut quote: Option<char> = None;

    while j < chars.len() {
        let c = chars[j];

        if c == '\\' && quote != Some('\'') {
            j += 2;
            continue;
        }

        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '\'' || c == '"' || c == '`' {
            quote = Some(c);
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            if depth == 0 {
                let cmd: String = chars[start + 2..j].iter().collect();
                return Some((cmd, j + 1));
            }
            depth -= 1;
        }

        j += 1;
    }

    None
}

fn find_backtick_cmdsub(chars: &[char], start: usize) -> Option<(String, usize)> {
    if chars.get(start) != Some(&'`') {
        return None;
    }

    let mut j = start + 1;
    while j < chars.len() {
        if chars[j] == '\\' {
            j += 2;
            continue;
        }
        if chars[j] == '`' {
            let cmd: String = chars[start + 1..j].iter().collect();
            return Some((cmd, j + 1));
        }
        j += 1;
    }

    None
}

fn run_heredoc_command_substitution(sh: &mut Shell, cmd: &str) -> String {
    let cr_list = crate::execute::run_command_line(sh, cmd, true, true);
    let mut output = String::new();
    for cr in cr_list {
        output.push_str(&cr.stdout);
    }
    output.trim_end_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::markers_in_line;
    use super::preprocess;
    use super::split_buffer;
    use crate::shell::Shell;
    use crate::types::HEREDOC_MARKER_PREFIX;

    /// The line as rewritten, with the marker made predictable, plus the body
    /// the one heredoc in it collected.
    fn run(input: &str) -> Result<(String, String), String> {
        let mut sh = Shell::new();
        let (line, markers) = preprocess(input, &mut sh)?;
        let body = match markers.first() {
            Some(marker) => sh.heredocs[marker].body.clone(),
            None => String::new(),
        };
        let mut line_new = line;
        for marker in &markers {
            line_new = line_new.replace(marker.as_str(), "M");
        }
        Ok((line_new, body))
    }

    /// The body the one heredoc in `input` collected, and whether its
    /// delimiter was quoted.
    fn heredoc_of(input: &str) -> (String, bool) {
        let mut sh = Shell::new();
        let (_, markers) = preprocess(input, &mut sh).unwrap();
        let stored = &sh.heredocs[&markers[0]];
        (stored.body.clone(), stored.quoted)
    }

    #[test]
    fn test_quoting_anywhere_in_the_delimiter_turns_expansion_off() {
        // the delimiter is the word with its quoting removed, and the body is
        // literal -- which is what `<< \EOF`, the usual way of asking for a
        // verbatim body, has to mean
        for head in &[
            "cat << \\EOF",
            "cat << 'EOF'",
            "cat << \"EOF\"",
            "cat << EO'F'",
            "cat << E\\OF",
        ] {
            let (body, quoted) = heredoc_of(&format!("{}\nkeep $HOME\nEOF\n", head));
            assert_eq!(body, "keep $HOME\n", "head: {}", head);
            assert!(quoted, "head: {}", head);
        }

        let (_, quoted) = heredoc_of("cat << EOF\nkeep $HOME\nEOF\n");
        assert!(!quoted);
    }

    #[test]
    fn test_marker_is_not_guessable() {
        let mut sh = Shell::new();
        let (line, markers) = preprocess("cat << EOF\nbody\nEOF\n", &mut sh).unwrap();
        // the session id is in the name, so a word from an expansion cannot
        // spell it out and reach the body behind it
        assert!(!markers[0].ends_with("_0") || markers[0].len() > HEREDOC_MARKER_PREFIX.len() + 2);
        assert!(markers[0].contains(&sh.session_id.replace('-', "")));
        // and it is written quoted, which is what marks it as ours
        assert!(line.contains(&format!("<<< '{}'", markers[0])));
    }

    #[test]
    fn test_heredoc_head_forms() {
        for head in &[
            "cat << EOF",
            "cat << EOF  ",
            "cat<<EOF",
            "cat <<EOF",
            "cat <<'EOF'",
            "cat << 'EOF'",
            "cat <<     \"EOF\"",
        ] {
            let input = format!("{}\nfoo\nEOF\n", head);
            let (line, body) = run(&input).unwrap();
            assert!(
                line.starts_with("cat <<< 'M'"),
                "head: {}, got {}",
                head,
                line
            );
            assert_eq!(body, "foo\n", "head: {}", head);
        }
    }

    #[test]
    fn test_heredoc_keeps_the_rest_of_the_line() {
        let (line, _) = run("cat > out.txt << XX\n1\nXX\n").unwrap();
        assert_eq!(line, "cat > out.txt <<< 'M'\n");

        let (line, _) = run("cat <<EOF>out.txt\n1\nEOF\n").unwrap();
        assert_eq!(line, "cat <<< 'M' >out.txt\n");

        let (line, _) = run("cat <<'EOF' | sed 's/l/e/g'\nHello\nEOF\n").unwrap();
        assert_eq!(line, "cat <<< 'M' | sed 's/l/e/g'\n");
    }

    #[test]
    fn test_delimiter_must_match_exactly() {
        // a trailing space is not part of the delimiter, so this never closes
        assert!(run("cat << EOF\nfoo\nEOF \n").is_err());
        assert!(run("cat << EOF\nfoo\nEOFEOF\n").is_err());
        assert!(run("cat << EOF\nfoo\n EOF\n").is_err());
        // ... and with `<<-`, leading tabs are, but leading spaces are not
        assert!(run("cat <<- EOF\nfoo\n\tEOF\n").is_ok());
        assert!(run("cat <<- EOF\nfoo\n    EOF\n").is_err());
    }

    #[test]
    fn test_dash_strips_leading_tabs_from_body() {
        let (_, body) = run("cat <<- EOF\n\t\talpha\n\tbeta\n\tEOF\n").unwrap();
        assert_eq!(body, "alpha\nbeta\n");
    }

    #[test]
    fn test_empty_body() {
        let (_, body) = run("cat << EOF\nEOF\n").unwrap();
        assert_eq!(body, "");
    }

    #[test]
    fn test_not_a_heredoc() {
        // in quotes, in a comment, or an arithmetic shift
        for input in &[
            "echo \"a << b\"",
            "echo 'a << b'",
            "echo hi  # use << EOF for heredocs",
            "echo $((1 << 3))",
            "cat <<< here-string",
        ] {
            let (line, body) = run(input).unwrap();
            assert_eq!(&line, input, "input: {}", input);
            assert_eq!(body, "", "input: {}", input);
        }
    }

    #[test]
    fn test_unclosed_heredoc_is_an_error() {
        assert!(run("cat << EOF\nfoo\n").is_err());
        assert!(run("cat << 'EOF\nfoo\nEOF\n").is_err());
        assert!(run("cat << A << B\nfoo\nA\n").is_err());
    }

    #[test]
    fn test_markers_in_line() {
        let marker = format!("{}7", HEREDOC_MARKER_PREFIX);
        let line = format!("cat <<< {} | wc -l", marker);
        assert_eq!(markers_in_line(&line), vec![marker]);
        assert!(markers_in_line("cat <<< plain").is_empty());
    }

    #[test]
    fn test_split_buffer_hides_body_from_the_parser() {
        // the apostrophe is body text: what the parser sees stays balanced
        let (cmds, open) = split_buffer("cat << EOF\n>> it's fine\n>> EOF");
        assert_eq!(cmds, "cat << EOF\n");
        assert!(!open);

        let (_, open) = split_buffer("cat << EOF\n>> it's fine");
        assert!(open);

        // buffers without a heredoc reach the parser untouched
        let (cmds, open) = split_buffer("echo 'one\n>> two'");
        assert_eq!(cmds, "echo 'one\n>> two'");
        assert!(!open);
    }
}
