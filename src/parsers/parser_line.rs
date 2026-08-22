use regex::Regex;

use crate::libs;
use crate::shell::DATA_MARK;
use crate::tools;
use crate::types::{LineInfo, Redirection, Tokens};

/// Take out the data marks the parser put on characters it wants later steps
/// to read as text (see `shell::DATA_MARK`).
///
/// Expansion takes its own marks out when it is done with them. This is for
/// the callers that never run expansion -- completion, and anything else
/// reading a word to show it or match it against a file name -- which want
/// the word as the line spelled it.
pub fn strip_marks(text: &str) -> String {
    if text.contains(DATA_MARK) {
        return text.replace(DATA_MARK, "");
    }
    text.to_string()
}

/// Mark `c` as text and put it in `token` (see `shell::DATA_MARK`).
///
/// A mark always applies to the character right after it, so two in a row
/// would mean the first is marking the second and the character after them
/// is bare. Never emitting a second one keeps marking idempotent, which is
/// what lets already-marked text be parsed again -- `scripting::expand_args`
/// rebuilds a line from tokens that may carry marks, and that line comes back
/// through here.
fn push_marked(token: &mut String, c: char) {
    if !token.ends_with(DATA_MARK) {
        token.push(DATA_MARK);
    }
    token.push(c);
}

pub fn line_to_plain_tokens(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let linfo = parse_line(line);
    for (_, r) in linfo.tokens {
        result.push(strip_marks(&r));
    }
    result
}

pub fn tokens_to_args(tokens: &Tokens) -> Vec<String> {
    let mut result = Vec::new();
    for s in tokens {
        result.push(s.1.clone());
    }
    result
}

pub fn tokens_to_line(tokens: &Tokens) -> String {
    let mut result = String::new();
    for t in tokens {
        if t.0.is_empty() {
            result.push_str(&t.1);
        } else {
            let s = tools::wrap_sep_string(&t.0, &t.1);
            result.push_str(&s);
        }
        result.push(' ');
    }
    if result.ends_with(' ') {
        let len = result.len();
        result.truncate(len - 1);
    }
    result
}

/// Parse command line for multiple commands.
///
/// ```ignore
/// let result1 = line_to_cmds("echo foo && echo bar; echo end");
/// // Returns vec!["echo foo", "&&", "echo bar", ";", "echo end"]
///
/// let result2 = line_to_cmds("man awk | grep version");
/// // Returns vec!["man awk | grep version"]
/// ```
pub fn line_to_cmds(line: &str) -> Vec<String> {
    // Special characters: http://tldp.org/LDP/abs/html/special-chars.html
    let mut result = Vec::new();
    let mut sep = String::new();
    let mut token = String::new();
    let mut has_backslash = false;
    // How many `$(` we are inside. A command inside a substitution is run by
    // the substitution itself, so the separators in it are not ours to split
    // on: `echo $(printf a; printf b)` is one command, not two.
    let mut depth_cmdsub = 0;
    let len = line.chars().count();
    for (i, c) in line.chars().enumerate() {
        if has_backslash {
            token.push('\\');
            token.push(c);
            has_backslash = false;
            continue;
        }

        if c == '\\' && sep != "'" {
            has_backslash = true;
            continue;
        }

        // Once inside a substitution every `(` counts, whatever opened it:
        // a grouping paren in `$(true; (true); echo b)` must not let its `)`
        // close the substitution early.
        if sep.is_empty() && c == '(' && (token.ends_with('$') || depth_cmdsub > 0) {
            depth_cmdsub += 1;
            token.push(c);
            continue;
        }
        if sep.is_empty() && c == ')' && depth_cmdsub > 0 {
            depth_cmdsub -= 1;
            token.push(c);
            continue;
        }

        // A `#` opens a comment only where a word could start. In the middle
        // of one it is an ordinary character, as in `echo a#b` or `${V#pat}`.
        if c == '#' {
            if sep.is_empty() && (token.is_empty() || token.ends_with([' ', '\t'])) {
                break;
            } else {
                token.push(c);
                continue;
            }
        }
        if c == '\'' || c == '"' || c == '`' {
            if sep.is_empty() {
                sep.push(c);
                token.push(c);
                continue;
            } else if sep == c.to_string() {
                token.push(c);
                sep = String::new();
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        if (c == '&' || c == '|') && depth_cmdsub > 0 {
            token.push(c);
            continue;
        }
        if c == '&' || c == '|' {
            // needs watch ahead here
            if sep.is_empty() {
                if i + 1 == len {
                    // for bg commands, e.g. `ls &`
                    token.push(c);
                    continue;
                } else {
                    let c_next = match line.chars().nth(i + 1) {
                        Some(x) => x,
                        None => {
                            println!("chars nth error - should never happen");
                            continue;
                        }
                    };

                    if c_next != c {
                        token.push(c);
                        continue;
                    }
                }
            }

            if sep.is_empty() {
                sep.push(c);
                continue;
            } else if c.to_string() == sep {
                let _token = token.trim().to_string();
                if !_token.is_empty() {
                    result.push(_token);
                }
                token = String::new();
                result.push(format!("{}{}", sep, sep));
                sep = String::new();
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        // A newline ends a command just like `;` does, so that program text
        // given on one string -- `cicada -c 'echo a\necho b'` or a here-doc
        // fed to a non-tty shell -- runs as the two commands it spells.
        if c == ';' || c == '\n' {
            if sep.is_empty() && depth_cmdsub == 0 {
                let _token = token.trim().to_string();
                let is_empty = _token.is_empty();
                if !is_empty {
                    result.push(_token);
                }
                // A blank line is not an empty command, while a `;` written on
                // its own is kept as the separator it is.
                if c == ';' || !is_empty {
                    result.push(String::from(";"));
                }
                token = String::new();
                continue;
            } else {
                token.push(c);
                continue;
            }
        }
        token.push(c);
    }
    if !token.is_empty() {
        result.push(token.trim().to_string());
    }
    result
}

/// Parse command line to tokens
///
/// ```ignore
/// let result = parse_line("echo 'hi yoo' | grep \"hi\"");
/// // Returns LineInfo {
/// //     tokens: vec![
/// //         ("", "echo"),
/// //         ("'", "hi yoo"),
/// //         ("", "|"),
/// //         ("", "grep"),
/// //         ("\"", "hi"),
/// //     ],
/// //     is_complete: true
/// // }
/// ```
// #[allow(clippy::cyclomatic_complexity)]
pub fn parse_line(line: &str) -> LineInfo {
    // FIXME: let rewrite this parse part and make it a separated lib
    let mut result = Vec::new();
    if tools::is_arithmetic(line) {
        for x in line.split(' ') {
            result.push((String::from(""), x.to_string()));
        }
        return LineInfo::new(result);
    }

    let mut sep = String::new();
    // `sep_second` is for commands like this:
    //    export DIR=`brew --prefix openssl`/include
    // it only could have non-empty value when sep is empty.
    let mut sep_second = String::new();
    let mut token = String::new();
    let mut has_backslash = false;
    let mut met_parenthesis = false;
    // How many `(` are open in the substitution we are in, and the quote
    // character open inside it (0 for none).
    let mut depth_parens = 0;
    let mut quote_in_parens = '\u{0}';
    let mut new_round = true;
    let mut skip_next = false;
    let mut has_dollar = false;
    let mut parens_left_ignored = false;

    // for cmds like: `ll foo\>bar end` -> `ll 'foo>bar' end`
    let mut sep_made = String::new();

    // using semi_ok makes quite dirty here
    // it is mainly for path completion like:
    // $ ls "foo b<TAB>
    // # then got `"foo bar"/`, then hit tab again:
    // $ ls "foo bar"/<TAB>
    // # should got:
    // $ ls "foo bar/the-single-file.txt"
    // also using semi_ok makes the following command works as expected:
    // $ touch "foo"/bar.txt  # create bar.txt under ./foo directory
    let mut semi_ok = false;
    let count_chars = line.chars().count();
    for (i, c) in line.chars().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        // `ll foo\>bar` -> `ll 'foo>bar' end`: an escaped redirection in an
        // unquoted word becomes a quoted one, so it is text and not an
        // operator. Only an unquoted word, though -- inside a quoted stretch
        // of a value there is no operator to defuse, and marking the word as
        // quoted would stop `V="a\>b"` from being an assignment at all.
        if has_backslash && sep.is_empty() && sep_second.is_empty() && (c == '>' || c == '<') {
            sep_made = String::from("'");
            token.push(c);
            has_backslash = false;
            continue;
        }

        if has_backslash {
            has_backslash = false;

            // Inside double quotes a backslash escapes only the characters
            // it has meaning for; anywhere else it is a character of its own:
            // "\a" --> \a, but "\"" --> ". A backtick is left out of that
            // set on purpose -- inside a substitution `\`` opens a nested
            // one, so there it is syntax rather than a character.
            if (sep == "\"" || sep_second == "\"") && !matches!(c, '"' | '$' | '\\') {
                token.push('\\');
                token.push(c);
                continue;
            }

            // `\|` at the start of a word would otherwise be read as a pipe,
            // because operators are recognized by comparing whole tokens.
            if new_round && sep.is_empty() && c == '|' && token.is_empty() {
                sep = String::from("\\");
                token = format!("{}", c);
                new_round = false;
                continue;
            }

            // An escaped `$` or quote is text. Saying so with a data mark,
            // rather than by keeping the backslash, is what makes it text for
            // *every* later step: expansion, substitution and quote removal
            // all step over a marked character, and `do_expansion` takes the
            // marks back out once none of them can mistake one for syntax.
            // Keeping the backslash instead is what made `\$b` lose its `$`
            // and `\$1` keep its backslash: each step had to recognize the
            // escape for itself, and they did not agree on what a name is.
            if c == '$' || ((c == '\'' || c == '"') && sep.is_empty()) {
                push_marked(&mut token, c);
            } else {
                token.push(c);
            }
            new_round = false;
            continue;
        }

        // Inside a single-quoted stretch of an assignment value -- the part of
        // `V=a'b c'` between the quotes -- every character is literal, so mark
        // the two that a later step could otherwise read as syntax. Without
        // this, `V='$HOME'x` would expand: the quotes tell `find_sub_start`
        // what is quoted, but nothing was telling `expand_env`.
        if sep.is_empty() && sep_second == "'" {
            if c == '\'' {
                sep_second = String::new();
                token.push(c);
                continue;
            }
            if c == '$' || c == '`' {
                push_marked(&mut token, c);
            } else {
                token.push(c);
            }
            continue;
        }

        if c == '$' {
            has_dollar = true;
        }

        // Inside a `$(...)` the quotes belong to the inner command, so track
        // which one is open. A `)` in `$(echo "a)b")` is then just a
        // character, and the substitution ends at the `)` that matches its
        // own `(`, however many are nested.
        if met_parenthesis && sep.is_empty() {
            if quote_in_parens != '\u{0}' {
                if c == quote_in_parens {
                    quote_in_parens = '\u{0}';
                }
                token.push(c);
                continue;
            }
            if c == '\'' || c == '"' || c == '`' {
                quote_in_parens = c;
                token.push(c);
                continue;
            }
        }

        // for cases like: echo $(foo bar)
        if c == '(' && sep.is_empty() {
            if !has_dollar && token.is_empty() {
                // temp solution for cmd like `(ls)`, `(ls -lh)`
                parens_left_ignored = true;
                continue;
            }
            met_parenthesis = true;
            depth_parens += 1;
        }
        if c == ')' {
            if parens_left_ignored && !has_dollar {
                // temp solution for cmd like `(ls)`, `(ls -lh)`
                if i == count_chars - 1
                    || (i + 1 < count_chars && line.chars().nth(i + 1).unwrap() == ' ')
                {
                    continue;
                }
            }
            if sep.is_empty() {
                depth_parens -= 1;
                if depth_parens <= 0 {
                    depth_parens = 0;
                    met_parenthesis = false;
                }
            }
        }

        if c == '\\' {
            // A backslash is literal inside single quotes, and inside a
            // backtick stretch it belongs to the command about to be run.
            // Everywhere else it escapes what comes next -- including inside
            // the double-quoted stretch of an assignment value, which is why
            // this asks `sep_second` and not just `sep`.
            //
            // A word that *began* with a single quote counts as inside it for
            // the whole word, even after the quote has closed: `echo 'x'\\`
            // keeps both backslashes, where bash keeps one. Telling the two
            // apart needs `semi_ok` to mean "outside the quotes now", and it
            // does not -- it is set again by the quote that *reopens* a
            // stretch, so `'a''b'` would lose the escapes it should keep.
            if sep == "'" || sep_second == "`" {
                token.push(c)
            } else {
                has_backslash = true;
            }
            continue;
        }

        if new_round {
            if c == ' ' {
                continue;
            } else if c == '"' || c == '\'' || c == '`' {
                sep = c.to_string();
                new_round = false;
                continue;
            }

            sep = String::new();

            if c == '#' {
                // handle inline comments -- but only at the start of a word,
                // so that `echo a#b` and `${V#pat}` keep their `#`
                break;
            }

            if c == '|' {
                if i + 1 < count_chars && line.chars().nth(i + 1).unwrap() == '|' {
                    result.push((String::from(""), "||".to_string()));
                    skip_next = true;
                } else {
                    result.push((String::from(""), "|".to_string()));
                }
                new_round = true;
                continue;
            }

            token.push(c);
            new_round = false;
            continue;
        }

        if c == '|' && !has_backslash {
            if semi_ok {
                if sep.is_empty() && !sep_made.is_empty() {
                    result.push((sep_made.to_string(), token));
                    sep_made = String::new();
                } else {
                    result.push((sep.to_string(), token));
                }
                result.push((String::from(""), "|".to_string()));
                sep = String::new();
                sep_second = String::new();
                token = String::new();
                new_round = true;
                semi_ok = false;
                continue;
            } else if !met_parenthesis && sep_second.is_empty() && sep.is_empty() {
                if sep.is_empty() && !sep_made.is_empty() {
                    result.push((sep_made.to_string(), token));
                    sep_made = String::new();
                } else {
                    result.push((String::from(""), token));
                }
                result.push((String::from(""), "|".to_string()));
                sep = String::new();
                sep_second = String::new();
                token = String::new();
                new_round = true;
                continue;
            }
        }

        if c == ' ' {
            if semi_ok {
                if sep.is_empty() && !sep_made.is_empty() {
                    result.push((sep_made.to_string(), token));
                    sep_made = String::new();
                } else {
                    result.push((sep.to_string(), token));
                }
                sep = String::new();
                sep_second = String::new();
                token = String::new();
                new_round = true;
                semi_ok = false;
                continue;
            }

            if has_backslash {
                has_backslash = false;
                token.push(c);
                continue;
            }

            if met_parenthesis {
                token.push(c);
                continue;
            }

            if sep == "\\" {
                result.push((String::from("\\"), token));
                token = String::new();
                new_round = true;
                continue;
            }

            if sep.is_empty() {
                if sep_second.is_empty() {
                    if sep.is_empty() && !sep_made.is_empty() {
                        result.push((sep_made.clone(), token));
                        sep_made = String::new();
                    } else {
                        result.push((String::from(""), token));
                    }
                    token = String::new();
                    new_round = true;
                    continue;
                } else {
                    token.push(c);
                    continue;
                }
            } else {
                token.push(c);
                continue;
            }
        }

        if c == '\'' || c == '"' || c == '`' {
            if has_backslash {
                has_backslash = false;
                token.push(c);
                continue;
            }

            if sep != c.to_string() && semi_ok {
                if sep.is_empty() && !sep_made.is_empty() {
                    result.push((sep_made.to_string(), token));
                    sep_made = String::new();
                } else {
                    result.push((sep.to_string(), token));
                }
                sep = String::new();
                sep_second = String::new();
                token = String::new();
                new_round = true;
                semi_ok = false;
                // do not use continue here!
            }

            if sep != c.to_string() && met_parenthesis {
                token.push(c);
                continue;
            }
            if sep.is_empty() && !sep_second.is_empty() && sep_second != c.to_string() {
                // A quote that the double-quoted stretch we are in does not
                // close is a character of the value: the `'` in `V="it's"`.
                // Mark it, so that substitution does not read it as quoting
                // either. Inside a backtick stretch the quote belongs to the
                // command about to be run, and is left alone.
                if sep_second == "\"" && c == '\'' {
                    push_marked(&mut token, c);
                } else {
                    token.push(c);
                }
                continue;
            }

            if sep.is_empty() {
                // `(?s)` so that `A="one\ntwo"` is still seen as an
                // assignment once its value has run past a newline.
                let is_an_env = libs::re::re_contains(&token, r"(?s)^[a-zA-Z0-9_]+=.*$");
                if !is_an_env && (c == '\'' || c == '"') {
                    sep = c.to_string();
                    continue;
                }

                // In an assignment value the quote opens a stretch instead of
                // the whole word, so it cannot be recorded in `sep`: it is
                // tracked in `sep_second` -- which keeps a space inside the
                // stretch from splitting the word -- and the character stays
                // in the text, where substitution reads it to tell quoted
                // from unquoted. `shell::remove_quotes` takes it out at the
                // end of expansion, which is what makes `V=a"x"c` set `axc`.
                token.push(c);
                if sep_second.is_empty() {
                    sep_second = c.to_string();
                } else if sep_second == c.to_string() {
                    sep_second = String::new();
                }
                continue;
            } else if sep == c.to_string() {
                semi_ok = true;
                continue;
            } else {
                token.push(c);
            }
        } else {
            if has_backslash {
                has_backslash = false;
                if sep == "\"" || sep == "'" {
                    token.push('\\');
                }
            }
            token.push(c);
        }
    }
    if !token.is_empty() || semi_ok {
        if sep.is_empty() && !sep_made.is_empty() {
            result.push((sep_made.clone(), token));
        } else {
            result.push((sep.clone(), token));
        }
    }

    let mut is_line_complete = true;
    if !result.is_empty() {
        let token_last = result[result.len() - 1].clone();
        if token_last.0.is_empty() && token_last.1 == "|" {
            is_line_complete = false;
        }
    }

    if !sep.is_empty() {
        is_line_complete = semi_ok;
    }
    if has_backslash {
        is_line_complete = false;
    }

    LineInfo {
        tokens: result,
        is_complete: is_line_complete,
    }
}

pub fn tokens_to_redirections(tokens: &Tokens) -> Result<(Tokens, Vec<Redirection>), String> {
    let mut tokens_new = Vec::new();
    let mut redirects = Vec::new();
    let mut to_be_continued = false;
    let mut to_be_continued_s1 = String::new();
    let mut to_be_continued_s2 = String::new();

    for token in tokens {
        let sep = &token.0;
        if !sep.is_empty() && !to_be_continued {
            tokens_new.push(token.clone());
            continue;
        }
        let word = &token.1;

        if to_be_continued {
            if sep.is_empty() && word.starts_with('&') {
                return Err(String::from("bad redirection syntax near &"));
            }

            let s3 = word.to_string();
            if libs::re::re_contains(&to_be_continued_s1, r"^\d+$") {
                if to_be_continued_s1 != "1" && to_be_continued_s1 != "2" {
                    return Err(String::from("Bad file descriptor #3"));
                }
                let s1 = to_be_continued_s1.clone();
                let s2 = to_be_continued_s2.clone();
                redirects.push((s1, s2, s3));
            } else {
                if !to_be_continued_s1.is_empty() {
                    tokens_new.push((sep.clone(), to_be_continued_s1.to_string()));
                }
                redirects.push(("1".to_string(), to_be_continued_s2.clone(), s3));
            }

            to_be_continued = false;
            continue;
        }

        let ptn1 = r"^([^>]*)(>>?)([^>]+)$";
        let ptn2 = r"^([^>]*)(>>?)$";
        if !libs::re::re_contains(word, r">") {
            tokens_new.push(token.clone());
        } else if libs::re::re_contains(word, ptn1) {
            let re;
            if let Ok(x) = Regex::new(ptn1) {
                re = x;
            } else {
                return Err(String::from("Failed to build Regex"));
            }

            if let Some(caps) = re.captures(word) {
                let s1 = caps.get(1).unwrap().as_str();
                let s2 = caps.get(2).unwrap().as_str();
                let s3 = caps.get(3).unwrap().as_str();
                if s3.starts_with('&') && s3 != "&1" && s3 != "&2" {
                    return Err(String::from("Bad file descriptor #1"));
                }

                if libs::re::re_contains(s1, r"^\d+$") {
                    if s1 != "1" && s1 != "2" {
                        return Err(String::from("Bad file descriptor #2"));
                    }
                    redirects.push((s1.to_string(), s2.to_string(), s3.to_string()));
                } else {
                    if !s1.is_empty() {
                        tokens_new.push((sep.clone(), s1.to_string()));
                    }
                    redirects.push((String::from("1"), s2.to_string(), s3.to_string()));
                }
            }
        } else if libs::re::re_contains(word, ptn2) {
            let re;
            if let Ok(x) = Regex::new(ptn2) {
                re = x;
            } else {
                return Err(String::from("Failed to build Regex"));
            }

            if let Some(caps) = re.captures(word) {
                let s1 = caps.get(1).unwrap().as_str();
                let s2 = caps.get(2).unwrap().as_str();

                to_be_continued = true;
                to_be_continued_s1 = s1.to_string();
                to_be_continued_s2 = s2.to_string();
            }
        }
    }

    if to_be_continued {
        return Err(String::from("redirection syntax error"));
    }

    Ok((tokens_new, redirects))
}

#[cfg(test)]
mod tests {
    use super::line_to_cmds;
    use super::line_to_plain_tokens;
    use super::parse_line;
    use super::tokens_to_line;
    use super::Tokens;

    fn _assert_vec_tuple_eq(a: Tokens, b: Vec<(&str, &str)>) {
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            let (ref l, ref r) = *item;
            assert_eq!(l, b[i].0);
            assert_eq!(r, b[i].1);
        }
    }

    fn _assert_vec_str_eq(a: Vec<String>, b: Vec<&str>) {
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            assert_eq!(item, b[i]);
        }
    }

    #[test]
    fn test_parse_line() {
        let v = vec![
            ("ls", vec![("", "ls")]),
            ("(ls)", vec![("", "ls")]),
            ("(ls -lh)", vec![("", "ls"), ("", "-lh")]),
            // quotes inside `$(...)` belong to the inner command, so a `)`
            // in them does not end the substitution
            (
                "echo $(echo \"a)b\")",
                vec![("", "echo"), ("", "$(echo \"a)b\")")],
            ),
            (
                "echo $(echo 'a)b')",
                vec![("", "echo"), ("", "$(echo 'a)b')")],
            ),
            (
                "echo $(echo $(echo N))",
                vec![("", "echo"), ("", "$(echo $(echo N))")],
            ),
            (
                "echo $(echo A)-$(echo B)",
                vec![("", "echo"), ("", "$(echo A)-$(echo B)")],
            ),
            ("  ls   ", vec![("", "ls")]),
            ("ls ' a '", vec![("", "ls"), ("'", " a ")]),
            ("ls -lh", vec![("", "ls"), ("", "-lh")]),
            ("  ls   -lh   ", vec![("", "ls"), ("", "-lh")]),
            ("ls 'abc'", vec![("", "ls"), ("'", "abc")]),
            ("ls \"Hi 你好\"", vec![("", "ls"), ("\"", "Hi 你好")]),
            ("ls \"abc\"", vec![("", "ls"), ("\"", "abc")]),
            ("echo \"\"", vec![("", "echo"), ("\"", "")]),
            ("echo \"\\\"\"", vec![("", "echo"), ("\"", "\"")]),
            ("echo \"\\a\"", vec![("", "echo"), ("\"", "\\a")]),
            ("echo \"hi $USER\"", vec![("", "echo"), ("\"", "hi $USER")]),
            ("echo 'hi $USER'", vec![("", "echo"), ("'", "hi $USER")]),
            ("echo '###'", vec![("", "echo"), ("'", "###")]),
            ("rd0 >", vec![("", "rd0"), ("", ">")]),
            ("rd1 \\>", vec![("", "rd1"), ("'", ">")]),
            (
                "rd2 foo > bar",
                vec![("", "rd2"), ("", "foo"), ("", ">"), ("", "bar")],
            ),
            ("rd3 foo>bar", vec![("", "rd3"), ("", "foo>bar")]),
            ("rd4 foo\\>bar", vec![("", "rd4"), ("'", "foo>bar")]),
            (
                "rd51 foo\\>bar end",
                vec![("", "rd51"), ("'", "foo>bar"), ("", "end")],
            ),
            (
                "rd52 foo\\>bar\\ baz",
                vec![("", "rd52"), ("'", "foo>bar baz")],
            ),
            (
                "rd6 foo\\>bar\\ baz end",
                vec![("", "rd6"), ("'", "foo>bar baz"), ("", "end")],
            ),
            ("echo a\\ bc", vec![("", "echo"), ("", "a bc")]),
            ("echo a\\ b cd", vec![("", "echo"), ("", "a b"), ("", "cd")]),
            (
                "mv a\\ b\\ c\\ d xy",
                vec![("", "mv"), ("", "a b c d"), ("", "xy")],
            ),
            ("echo \\#", vec![("", "echo"), ("", "#")]),
            (
                "echo 'hi $USER' |  wc  -l ",
                vec![
                    ("", "echo"),
                    ("'", "hi $USER"),
                    ("", "|"),
                    ("", "wc"),
                    ("", "-l"),
                ],
            ),
            (
                "echo `uname -m` | wc",
                vec![("", "echo"), ("`", "uname -m"), ("", "|"), ("", "wc")],
            ),
            (
                "echo `uname -m` | wc # test it",
                vec![("", "echo"), ("`", "uname -m"), ("", "|"), ("", "wc")],
            ),
            ("echo '`uname -m`'", vec![("", "echo"), ("'", "`uname -m`")]),
            ("'\"\"\"\"'", vec![("'", "\"\"\"\"")]),
            ("\"\'\'\'\'\"", vec![("\"", "''''")]),
            (
                "export DIR=`brew --prefix openssl`/include",
                vec![("", "export"), ("", "DIR=`brew --prefix openssl`/include")],
            ),
            (
                "export FOO=\"`date` and `go version`\"",
                vec![("", "export"), ("", "FOO=\"`date` and `go version`\"")],
            ),
            ("ps|wc", vec![("", "ps"), ("", "|"), ("", "wc")]),
            (
                "cat foo.txt|sort -n|wc",
                vec![
                    ("", "cat"),
                    ("", "foo.txt"),
                    ("", "|"),
                    ("", "sort"),
                    ("", "-n"),
                    ("", "|"),
                    ("", "wc"),
                ],
            ),
            (
                "man awk| awk -F \"[ ,.\\\"]+\" 'foo' |sort -k2nr|head",
                vec![
                    ("", "man"),
                    ("", "awk"),
                    ("", "|"),
                    ("", "awk"),
                    ("", "-F"),
                    ("\"", "[ ,.\"]+"),
                    ("\'", "foo"),
                    ("", "|"),
                    ("", "sort"),
                    ("", "-k2nr"),
                    ("", "|"),
                    ("", "head"),
                ],
            ),
            (
                "echo a || echo b",
                vec![("", "echo"), ("", "a"), ("", "||"), ("", "echo"), ("", "b")],
            ),
            (
                "echo \'{\\\"size\\\": 12}\'",
                vec![("", "echo"), ("\'", "{\\\"size\\\": 12}")],
            ),
            (
                "echo foo >/dev/null",
                vec![("", "echo"), ("", "foo"), ("", ">/dev/null")],
            ),
            (
                "ls foo 2>/dev/null",
                vec![("", "ls"), ("", "foo"), ("", "2>/dev/null")],
            ),
            (
                "ls foo 2> '/dev/null'",
                vec![("", "ls"), ("", "foo"), ("", "2>"), ("\'", "/dev/null")],
            ),
            (
                "ls > /dev/null 2>&1",
                vec![("", "ls"), ("", ">"), ("", "/dev/null"), ("", "2>&1")],
            ),
            (
                "ls > /dev/null 2>& 1",
                vec![
                    ("", "ls"),
                    ("", ">"),
                    ("", "/dev/null"),
                    ("", "2>&"),
                    ("", "1"),
                ],
            ),
            ("echo foo`date`", vec![("", "echo"), ("", "foo`date`")]),
            (
                "echo 123'foo bar'",
                vec![("", "echo"), ("\'", "123foo bar")],
            ),
            (
                "echo --author=\"Hugo Wang <w@mitnk.com>\"",
                vec![("", "echo"), ("\"", "--author=Hugo Wang <w@mitnk.com>")],
            ),
            (
                "Foo=\"abc\" sh foo.sh",
                vec![("", "Foo=\"abc\""), ("", "sh"), ("", "foo.sh")],
            ),
            (
                "Foo=\"a b c\" ./foo.sh",
                vec![("", "Foo=\"a b c\""), ("", "./foo.sh")],
            ),
            (
                "echo $(foo bar baz)",
                vec![("", "echo"), ("", "$(foo bar baz)")],
            ),
            (
                "echo A$(foo bar)B",
                vec![("", "echo"), ("", "A$(foo bar)B")],
            ),
            (
                "echo A$(foo bar | cat)B",
                vec![("", "echo"), ("", "A$(foo bar | cat)B")],
            ),
            (
                "echo A$(echo bar | awk '{print $1}')B",
                vec![("", "echo"), ("", "A$(echo bar | awk '{print $1}')B")],
            ),
            (
                "echo A`echo foo`B",
                vec![("", "echo"), ("", "A`echo foo`B")],
            ),
            (
                "echo A`echo foo | cat`B",
                vec![("", "echo"), ("", "A`echo foo | cat`B")],
            ),
            (
                "echo A`echo foo bar | awk '{print $2, $1}'`B",
                vec![
                    ("", "echo"),
                    ("", "A`echo foo bar | awk '{print $2, $1}'`B"),
                ],
            ),
            (
                "echo \"a b c\"|wc -l",
                vec![
                    ("", "echo"),
                    ("\"", "a b c"),
                    ("", "|"),
                    ("", "wc"),
                    ("", "-l"),
                ],
            ),
            ("echo \"abc\"/", vec![("", "echo"), ("\"", "abc/")]),
            (
                "echo \"abc\"/foo.txt",
                vec![("", "echo"), ("\"", "abc/foo.txt")],
            ),
            (
                "echo \"abc\"/\"def\"",
                vec![("", "echo"), ("\"", "abc/def")],
            ),
            ("echo \'abc\'/", vec![("", "echo"), ("\'", "abc/")]),
            ("echo \'abc\'/foo", vec![("", "echo"), ("\'", "abc/foo")]),
            ("echo 'abc'/'def'", vec![("", "echo"), ("'", "abc/def")]),
            (
                // here the behavior is not the same with bash
                // bash:   echo "foo"/'bar' -> foo/bar
                // cicada: echo "foo"/'bar' -> foo/ bar
                //                                 ^ an extra space
                // see also comments up above on semi_ok
                "echo \"abc\"/'def'",
                vec![("", "echo"), ("\"", "abc/"), ("'", "def")],
            ),
            ("echo \\a\\b\\c", vec![("", "echo"), ("", "abc")]),
            ("echo \\|", vec![("", "echo"), ("\\", "|")]),
            ("echo \\|\\|\\|", vec![("", "echo"), ("\\", "|||")]),
            ("echo a\\|b", vec![("", "echo"), ("", "a|b")]),
            ("foo \\| bar", vec![("", "foo"), ("\\", "|"), ("", "bar")]),
            ("echo \\| foo", vec![("", "echo"), ("\\", "|"), ("", "foo")]),
            ("echo | foo", vec![("", "echo"), ("", "|"), ("", "foo")]),
            (
                "echo \\foo \\bar",
                vec![("", "echo"), ("", "foo"), ("", "bar")],
            ),
            (
                // The escaped `$` is marked as data, so the `$(` is text
                // and the mark comes off in `do_expansion`.
                "echo \\$\\(date\\)",
                vec![("", "echo"), ("", "\u{0}$(date)")],
            ),
            ("ll foo\\#bar", vec![("", "ll"), ("", "foo#bar")]),
            (
                "(1 + 2) ^ 31",
                vec![("", "(1"), ("", "+"), ("", "2)"), ("", "^"), ("", "31")],
            ),
            ("1+2-3*(4/5.0)", vec![("", "1+2-3*(4/5.0)")]),
            (
                "alias c='printf \"\\ec\"'",
                vec![("", "alias"), ("", "c='printf \"\\ec\"'")],
            ),
            // An escaped `$` is marked as data instead of keeping its
            // backslash, so that every later step reads it as text.
            ("echo a\\$b", vec![("", "echo"), ("", "a\u{0}$b")]),
            ("echo \\$b", vec![("", "echo"), ("", "\u{0}$b")]),
            ("echo \"a\\$b\"", vec![("", "echo"), ("\"", "a\u{0}$b")]),
            ("echo a\\$1", vec![("", "echo"), ("", "a\u{0}$1")]),
            ("echo a\\$\\$", vec![("", "echo"), ("", "a\u{0}$\u{0}$")]),
            // Single quotes are literal, so the backslash stays a character.
            ("echo 'a\\$b'", vec![("", "echo"), ("'", "a\\$b")]),
            // So is an escaped quote: it is not the quoting of the word.
            ("echo a\\\"b", vec![("", "echo"), ("", "a\u{0}\"b")]),
            // An assignment value keeps its quotes here -- substitution reads
            // them to tell quoted from unquoted, and `shell::remove_quotes`
            // takes them out at the end of expansion.
            ("V=a\"x\"c", vec![("", "V=a\"x\"c")]),
            ("V=a'x'c", vec![("", "V=a'x'c")]),
            // ... and what they quote is marked, so that `V='$HOME'x` does
            // not expand the way `V=$HOME` does.
            ("V='$HOME'x", vec![("", "V='\u{0}$HOME'x")]),
            ("V='`d`'", vec![("", "V='\u{0}`d\u{0}`'")]),
            ("V=\"$HOME\"x", vec![("", "V=\"$HOME\"x")]),
            // A backslash inside the double-quoted stretch of a value escapes
            // what follows it, as it does in a fully double-quoted word.
            ("V=\"a\\$b\"", vec![("", "V=\"a\u{0}$b\"")]),
        ];
        for (left, right) in v {
            println!("\ninput: {:?}", left);
            println!("expected: {:?}", right);
            let linfo = parse_line(left);
            let tokens = linfo.tokens;
            println!("real    : {:?}", tokens);
            _assert_vec_tuple_eq(tokens, right);
        }
    }

    #[test]
    fn test_line_to_plain_tokens() {
        let v = vec![
            ("ls", vec!["ls"]),
            ("  ls   ", vec!["ls"]),
            ("ls -lh", vec!["ls", "-lh"]),
            ("ls 'abc'", vec!["ls", "abc"]),
            ("ls a c", vec!["ls", "a", "c"]),
            ("ls a\\ c", vec!["ls", "a c"]),
            ("ls \"abc\"", vec!["ls", "abc"]),
            ("ls \"Hi 你好\"", vec!["ls", "Hi 你好"]),
            ("echo \"\"", vec!["echo", ""]),
            ("echo \"hi $USER\"", vec!["echo", "hi $USER"]),
            ("echo 'hi $USER'", vec!["echo", "hi $USER"]),
            (
                "echo 'hi $USER' |  wc  -l ",
                vec!["echo", "hi $USER", "|", "wc", "-l"],
            ),
            ("echo `uname -m` | wc", vec!["echo", "uname -m", "|", "wc"]),
            (
                "echo `uptime` | wc # testing",
                vec!["echo", "uptime", "|", "wc"],
            ),
            ("awk -F \"[ ,.\\\"]+\"", vec!["awk", "-F", "[ ,.\"]+"]),
            ("echo foo\\|bar", vec!["echo", "foo|bar"]),
            ("echo \"foo\\|bar\"", vec!["echo", "foo\\|bar"]),
            ("echo 'foo\\|bar'", vec!["echo", "foo\\|bar"]),
            ("echo a || echo b", vec!["echo", "a", "||", "echo", "b"]),
            (
                "echo \'{\\\"size\\\": 12}\'",
                vec!["echo", "{\\\"size\\\": 12}"],
            ),
            (
                // that is: echo '{"q": "{\"size\": 12}"}'
                "echo \'{\"q\": \"{\\\"size\\\": 12}\"}\'",
                vec!["echo", "{\"q\": \"{\\\"size\\\": 12}\"}"],
            ),
            ("echo a\\ b c", vec!["echo", "a b", "c"]),
            ("mv a\\ b\\ c\\ d\\ e xy", vec!["mv", "a b c d e", "xy"]),
        ];

        for (left, right) in v {
            println!("\ninput: {:?}", left);
            println!("expected: {:?}", right);
            let real = line_to_plain_tokens(left);
            println!("real    : {:?}", real);
            _assert_vec_str_eq(real, right);
        }
    }

    #[test]
    fn test_line_to_cmds() {
        let v = vec![
            ("ls", vec!["ls"]),
            ("ls &", vec!["ls &"]),
            ("ls -lh", vec!["ls -lh"]),
            (
                "awk -F \" \" '{print $1}' README.md",
                vec!["awk -F \" \" '{print $1}' README.md"],
            ),
            ("ls | wc", vec!["ls | wc"]),
            ("echo #foo; echo bar", vec!["echo"]),
            ("echo foo; echo bar", vec!["echo foo", ";", "echo bar"]),
            ("echo 'foo; echo bar'", vec!["echo 'foo; echo bar'"]),
            ("echo \"foo; echo bar\"", vec!["echo \"foo; echo bar\""]),
            ("echo `foo; echo bar`", vec!["echo `foo; echo bar`"]),
            ("echo foo && echo bar", vec!["echo foo", "&&", "echo bar"]),
            (
                "echo foo && echo bar && echo baz",
                vec!["echo foo", "&&", "echo bar", "&&", "echo baz"],
            ),
            ("echo foo || echo bar", vec!["echo foo", "||", "echo bar"]),
            (
                "echo foo && echo bar; echo end",
                vec!["echo foo", "&&", "echo bar", ";", "echo end"],
            ),
            ("echo \"\\\"\"", vec!["echo \"\\\"\""]),
            (
                "man awk| awk -F \"[ ,.\\\"]+\" 'foo' |sort -k2nr|head",
                vec!["man awk| awk -F \"[ ,.\\\"]+\" 'foo' |sort -k2nr|head"],
            ),
            (";", vec![";"]),
            ("||", vec!["||"]),
            ("&&", vec!["&&"]),
            ("ls foo\\#bar", vec!["ls foo\\#bar"]),
            ("ls \\|\\|foo", vec!["ls \\|\\|foo"]),
            // a comment may follow a tab as well as a space
            ("echo hi\t# comment", vec!["echo hi"]),
            // a newline ends a command, like `;`
            ("echo foo\necho bar", vec!["echo foo", ";", "echo bar"]),
            ("echo foo\n\necho bar", vec!["echo foo", ";", "echo bar"]),
            ("echo 'foo\nbar'", vec!["echo 'foo\nbar'"]),
            ("echo \"foo\nbar\"", vec!["echo \"foo\nbar\""]),
            // separators inside `$(...)` belong to the substitution
            (
                "echo $(printf a; printf b)",
                vec!["echo $(printf a; printf b)"],
            ),
            (
                "echo $(printf a && printf b)",
                vec!["echo $(printf a && printf b)"],
            ),
            (
                "echo $(printf a\nprintf b)",
                vec!["echo $(printf a\nprintf b)"],
            ),
            ("echo $(echo x | wc -l)", vec!["echo $(echo x | wc -l)"]),
            (
                "echo $(echo $(printf a; printf b))",
                vec!["echo $(echo $(printf a; printf b))"],
            ),
            // a grouping paren inside `$(...)` does not close the substitution
            (
                "echo $(true; (true); echo b)",
                vec!["echo $(true; (true); echo b)"],
            ),
            (
                "echo $(printf a; printf b); echo end",
                vec!["echo $(printf a; printf b)", ";", "echo end"],
            ),
        ];

        for (left, right) in v {
            _assert_vec_str_eq(line_to_cmds(left), right);
        }
    }

    #[test]
    fn test_tokens_to_line() {
        let tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "a\"b".to_string()),
        ];
        let line_exp = "echo \"a\\\"b\"";
        assert_eq!(tokens_to_line(&tokens), line_exp);

        let tokens = vec![
            ("".to_string(), "echo".to_string()),
            ("\"".to_string(), "中文".to_string()),
        ];
        let line_exp = "echo \"中文\"";
        assert_eq!(tokens_to_line(&tokens), line_exp);
    }
    #[test]
    fn test_is_complete() {
        // A line the shell can run as it stands. `\\$b` used to be reported
        // as unfinished -- the escape left the word marked with a `\\` that
        // nothing closed -- so an interactive shell asked for a second line
        // instead of running it.
        for line in vec![
            "echo hi",
            "V=1",
            "echo a\\$b",
            "echo \\$b",
            "echo \\$",
            "echo \\$b\\$c",
            "echo a\\\"b",
            "V=a\\\"b",
            "V=a\"x\"c",
            "V=a'x'c",
            "echo \"a\"",
        ] {
            assert!(
                parse_line(line).is_complete,
                "should be complete: {:?}",
                line
            );
        }

        // A line still waiting for something: an unclosed quote, or a
        // backslash with nothing after it.
        //
        // A quote left open *inside* an assignment value -- `V='a` -- is not
        // among them, and is not new here: completeness is judged from `sep`,
        // which only records the quoting of a word that begins with a quote,
        // while a value's stretches are tracked in `sep_second`.
        for line in ["echo 'a", "echo \"a", "echo `a", "echo a\\"] {
            assert!(
                !parse_line(line).is_complete,
                "should be incomplete: {:?}",
                line
            );
        }
    }
}
