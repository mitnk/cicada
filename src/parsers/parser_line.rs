use regex::Regex;

use tools;
use types::Command;
use types::Tokens;

pub fn line_to_plain_tokens(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let v = cmd_to_tokens(line);
    for (_, r) in v {
        result.push(r);
    }
    result
}

pub fn tokens_to_args(tokens: &Vec<(String, String)>) -> Vec<String> {
    let mut result = Vec::new();
    for s in tokens {
        result.push(s.1.clone());
    }
    result
}

/// Parse command line for multiple commands. Examples:
/// >>> line_to_cmds("echo foo && echo bar; echo end");
/// vec!["echo foo", "&&", "echo bar", ";", "echo end"]
/// >>> line_to_cmds("man awk | grep version");
/// vec!["man awk | grep version"]
pub fn line_to_cmds(line: &str) -> Vec<String> {
    // Special characters: http://tldp.org/LDP/abs/html/special-chars.html
    let mut result = Vec::new();
    let mut sep = String::new();
    let mut token = String::new();
    let len = line.len();
    for (i, c) in line.chars().enumerate() {
        if c == '#' {
            if sep.is_empty() {
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
        if c == '&' || c == '|' {
            // needs watch ahead here
            if sep.is_empty() {
                if i + 1 == len {
                    // for bg commands, e.g. `ls &`
                    token.push(c);
                    continue;
                } else {
                    let c_next;
                    match line.chars().nth(i + 1) {
                        Some(x) => c_next = x,
                        None => {
                            println!("chars nth error - should never happen");
                            continue;
                        }
                    }
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
        if c == ';' {
            if sep.is_empty() {
                let _token = token.trim().to_string();
                if !_token.is_empty() {
                    result.push(_token);
                }
                result.push(String::from(";"));
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

/// parse command line to tokens
/// >>> cmd_to_tokens("echo 'hi yoo' | grep \"hi\"");
/// vec![
///     ("", "echo"),
///     ("'", "hi yoo"),
///     ("", "|"),
///     ("", "grep"),
///     ("\"", "hi"),
/// ]
#[allow(cyclomatic_complexity)]
pub fn cmd_to_tokens(line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut sep = String::new();
    // `sep_second` is for commands like this:
    //    export DIR=`brew --prefix openssl`/include
    // it only could have non-empty value when sep is empty.
    let mut sep_second = String::new();
    let mut token = String::new();
    let mut has_backslash = false;
    let mut end_token_when_hit_space = false;
    let mut new_round = true;
    let mut skip_next = false;
    let count_chars = line.chars().count();
    for (i, c) in line.chars().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if c == '\\' && sep != "\'" {
            if !has_backslash {
                has_backslash = true;
            } else {
                has_backslash = false;
                token.push(c);
            }
            continue;
        }

        if new_round {
            if c == ' ' {
                continue;
            } else if c == '"' || c == '\'' || c == '`' {
                sep = c.to_string();
            } else {
                sep = String::new();

                // handle inline comments
                if c == '#' {
                    if has_backslash {
                        has_backslash = false;
                        token.push(c);
                        continue;
                    }
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
                } else {
                    token.push(c);
                }
            }
            new_round = false;
            continue;
        }

        if c == '|' && !has_backslash && sep.is_empty() {
            result.push((String::from(""), token));
            result.push((String::from(""), "|".to_string()));
            sep = String::new();
            sep_second = String::new();
            token = String::new();
            new_round = true;
            continue;
        }

        if c == ' ' {
            if end_token_when_hit_space {
                end_token_when_hit_space = false;
                result.push((String::from(""), token));
                token = String::new();
                new_round = true;
                continue;
            }

            if has_backslash {
                has_backslash = false;
                token.push(c);
                end_token_when_hit_space = true;
                continue;
            }
            if sep.is_empty() {
                if sep_second.is_empty() {
                    result.push((String::from(""), token));
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

            if sep.is_empty() {
                if c == '\'' || c == '"' {
                    sep = c.to_string();
                    continue;
                }

                token.push(c);
                if sep_second.is_empty() {
                    sep_second = c.to_string();
                } else if sep_second == c.to_string() {
                    sep_second = String::new();
                }
                continue;
            } else if sep == c.to_string() {
                result.push((c.to_string(), token));
                sep = String::new();
                sep_second = String::new();
                token = String::new();
                new_round = true;
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
    if !token.is_empty() {
        result.push((sep, token));
    }
    result
}

pub fn cmd_to_with_redirects(tokens: &Tokens) -> Result<Command, String> {
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

            let s3 = format!("{}{}{}", sep, word, sep);
            if tools::re_contains(&to_be_continued_s1, r"^\d+$") {
                if to_be_continued_s1 != "1" && to_be_continued_s1 != "2" {
                    return Err(String::from("Bad file descriptor #3"));
                }
                let s1 = to_be_continued_s1.clone();
                let s2 = to_be_continued_s2.clone();
                redirects.push((s1, s2, s3));
            } else {
                if to_be_continued_s1 != "" {
                    tokens_new.push((sep.clone(), to_be_continued_s1.to_string()));
                }
                redirects.push(("1".to_string(), to_be_continued_s2.clone(), s3));
            }

            to_be_continued = false;
            continue;
        }

        let ptn1 = r"^([^>]*)(>>?)([^>]+)$";
        let ptn2 = r"^([^>]*)(>>?)$";
        if !tools::re_contains(word, r">") {
            tokens_new.push(token.clone());
        } else if tools::re_contains(word, ptn1) {
            let re;
            if let Ok(x) = Regex::new(ptn1) {
                re = x;
            } else {
                return Err(String::from("Failed to build Regex"));
            }

            if let Some(caps) = re.captures(&word) {
                let s1 = caps.get(1).unwrap().as_str();
                let s2 = caps.get(2).unwrap().as_str();
                let s3 = caps.get(3).unwrap().as_str();
                if s3.starts_with('&') && s3 != "&1" && s3 != "&2" {
                    return Err(String::from("Bad file descriptor #1"));
                }

                if tools::re_contains(s1, r"^\d+$") {
                    if s1 != "1" && s1 != "2" {
                        return Err(String::from("Bad file descriptor #2"));
                    }
                    redirects.push((s1.to_string(), s2.to_string(), s3.to_string()));
                } else {
                    if s1 != "" {
                        tokens_new.push((sep.clone(), s1.to_string()));
                    }
                    redirects.push((String::from("1"), s2.to_string(), s3.to_string()));
                }
            }
        } else if tools::re_contains(word, ptn2) {
            let re;
            if let Ok(x) = Regex::new(ptn2) {
                re = x;
            } else {
                return Err(String::from("Failed to build Regex"));
            }

            if let Some(caps) = re.captures(&word) {
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

    Ok(Command {
        tokens: tokens_new,
        redirects,
    })
}

#[allow(dead_code)]
fn is_valid_cmd(cmd: &str) -> bool {
    if let Some(c) = cmd.chars().nth(0) {
        if c == '|' {
            return false;
        }
    }
    if let Some(c) = cmd.chars().rev().nth(0) {
        if c == '|' {
            return false;
        }
    }

    let tokens = cmd_to_tokens(cmd);
    let mut found_pipe = false;
    let len = tokens.len();
    for (i, token) in tokens.iter().enumerate() {
        let sep = &token.0;
        if !sep.is_empty() {
            found_pipe = false;
            continue;
        }
        let value = &token.1;
        if value == "|" {
            if found_pipe {
                return false;
            }
            found_pipe = true;
        }
        if value == "&" && i != len - 1 {
            return false;
        }
    }
    true
}

#[allow(dead_code)]
pub fn is_valid_input(line: &str) -> bool {
    let cmd_splitors = vec![";", "||", "&&"];

    let mut cmds = line_to_cmds(line);
    let mut len = cmds.len();
    if len == 0 {
        return false;
    }
    let _cmds = cmds.clone();
    let mut last = &_cmds[len - 1];
    if len >= 1 && last == ";" {
        cmds.pop();
        len = cmds.len();
        if len == 0 {
            return false;
        }
        last = &cmds[len - 1];
    }

    let mut last_cmd_is_cmd_sep = false;
    for cmd in &cmds {
        if cmd_splitors.contains(&cmd.as_str()) {
            if last_cmd_is_cmd_sep {
                return false;
            }
            last_cmd_is_cmd_sep = true;
            continue;
        } else {
            last_cmd_is_cmd_sep = false;
        }
        if !is_valid_cmd(cmd) {
            return false;
        }
    }

    if cmd_splitors.contains(&last.as_str()) {
        return false;
    }

    if len > 1 {
        for sep in cmd_splitors {
            if cmds.contains(&sep.to_string()) {
                if let Some(pos) = cmds.iter().position(|x| x == sep) {
                    if pos < len - 1 && cmds[pos + 1] == sep {
                        return false;
                    }
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::cmd_to_tokens;
    use super::is_valid_input;
    use super::line_to_cmds;
    use super::line_to_plain_tokens;

    fn _assert_vec_tuple_eq(a: Vec<(String, String)>, b: Vec<(&str, &str)>) {
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
    fn test_line_to_tokens() {
        let v = vec![
            ("ls", vec![("", "ls")]),
            ("  ls   ", vec![("", "ls")]),
            ("ls ' a '", vec![("", "ls"), ("'", " a ")]),
            ("ls -lh", vec![("", "ls"), ("", "-lh")]),
            ("  ls   -lh   ", vec![("", "ls"), ("", "-lh")]),
            ("ls 'abc'", vec![("", "ls"), ("'", "abc")]),
            ("ls \"Hi 你好\"", vec![("", "ls"), ("\"", "Hi 你好")]),
            ("ls \"abc\"", vec![("", "ls"), ("\"", "abc")]),
            ("echo \"\"", vec![("", "echo"), ("\"", "")]),
            ("echo \"hi $USER\"", vec![("", "echo"), ("\"", "hi $USER")]),
            ("echo 'hi $USER'", vec![("", "echo"), ("'", "hi $USER")]),
            ("echo '###'", vec![("", "echo"), ("'", "###")]),
            ("echo a\\ bc", vec![("", "echo"), ("", "a bc")]),
            ("echo a\\ b cd", vec![("", "echo"), ("", "a b"), ("", "cd")]),
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
                vec![("", "export"), ("\"", "FOO=`date` and `go version`")],
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
        ];
        for (left, right) in v {
            println!("\ninput: {:?}", left);
            println!("expected: {:?}", right);
            let args = cmd_to_tokens(left);
            println!("real    : {:?}", args);
            _assert_vec_tuple_eq(args, right);
        }
    }

    #[test]
    fn test_parse_line() {
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
            (
                "echo a\\ b c",
                vec!["echo", "a b", "c"],
            ),
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
            (
                "man awk| awk -F \"[ ,.\\\"]+\" 'foo' |sort -k2nr|head",
                vec!["man awk| awk -F \"[ ,.\\\"]+\" 'foo' |sort -k2nr|head"],
            ),
            (";", vec![";"]),
            ("||", vec!["||"]),
            ("&&", vec!["&&"]),
        ];

        for (left, right) in v {
            _assert_vec_str_eq(line_to_cmds(left), right);
        }
    }

    #[test]
    fn test_is_valid_input() {
        let invalid_list = vec![
            "foo |",
            "foo ||",
            "foo &&",
            "foo|",
            "foo | ",
            "| foo",
            "foo ; ; bar",
            "foo && && bar",
            "foo || || bar",
            "foo | | bar",
            "foo && ; bar",
            "foo || && bar",
            "foo | || bar",
            "foo ; | bar",
            "foo | ; bar",
            "foo | && bar",
            "foo | ; bar",
            "& foo",
            "foo & bar",
            "",
            ";",
            "||",
            "&&",
            "|",
        ];
        for line in &invalid_list {
            let valid = is_valid_input(line);
            if valid {
                println!("'{}' should be invalid", line);
            }
            assert!(!valid);
        }

        let valid_list = vec![
            "foo",
            "foo bar",
            "foo;",
            "foo ;",
            "foo | bar",
            "foo; bar",
            "foo && bar",
            "foo || bar",
            "foo &",
            "echo 'foo & bar'",
            "echo `foo | | bar`",
        ];
        for line in &valid_list {
            assert!(is_valid_input(line));
        }
    }
}
