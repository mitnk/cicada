pub fn parse_line(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let v = parse_args(line);
    for (_, r) in v {
        result.push(r);
    }
    result
}

pub fn parse_commands(line: &str) -> Vec<String> {
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
                result.push(token.trim().to_string());
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
                result.push(token.trim().to_string());
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

pub fn parse_args(line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut sep = String::new();
    // `sep_second` is for commands like this:
    //    export DIR=`brew --prefix openssl`/include
    // it only could have non-empty value when sep is empty.
    let mut sep_second = String::new();
    let mut token = String::new();
    let mut has_backslash = false;
    let mut new_round = true;
    for c in line.chars() {
        if c == '\\' {
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
                if c == '#' {
                    if has_backslash {
                        has_backslash = false;
                        token.push(c);
                        continue;
                    }
                    break;
                }
                token.push(c);
            }
            new_round = false;
            continue;
        }

        if c == ' ' {
            if has_backslash {
                has_backslash = false;
                token.push(c);
                continue;
            }
            if sep.is_empty() {
                if sep_second.is_empty() {
                    result.push((String::from(""), token));
                    sep = String::new();
                    sep_second = String::new();
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
        result.push((String::from(""), token));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use super::parse_line;
    use super::parse_commands;

    fn _assert_vec_tuple_eq(a: Vec<(String, String)>, b: Vec<(&str, &str)>) {
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            let (ref l, ref r) = *item;
            assert_eq!(l, b[i].0);
            assert_eq!(r, b[i].1);
        }
    }

    fn _assert_vec_str_eq(a: Vec<String>, b: Vec<&str>) {
        println!("a: {:?}", a);
        println!("b: {:?}", b);
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            assert_eq!(item, b[i]);
        }
    }

    #[test]
    fn test_parse_args() {
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
            ("echo \\#", vec![("", "echo"), ("", "#")]),
            (
                "echo 'hi $USER' |  wc  -l ",
                vec![
                    ("", "echo"),
                    ("'", "hi $USER"),
                    ("", "|"),
                    ("", "wc"),
                    ("", "-l"),
                ]
            ),
            (
                "echo `uname -m` | wc",
                vec![("", "echo"), ("`", "uname -m"), ("", "|"), ("", "wc")]
            ),
            (
                "echo `uname -m` | wc # test it",
                vec![("", "echo"), ("`", "uname -m"), ("", "|"), ("", "wc")]
            ),
            ("echo '`uname -m`'", vec![("", "echo"), ("'", "`uname -m`")]),
            ("'\"\"\"\"'", vec![("'", "\"\"\"\"")]),
            ("\"\'\'\'\'\"", vec![("\"", "''''")]),
            (
                "export DIR=`brew --prefix openssl`/include",
                vec![("", "export"), ("", "DIR=`brew --prefix openssl`/include")]
            ),
            (
                "export FOO=\"`date` and `go version`\"",
                vec![("", "export"), ("", "FOO=\"`date` and `go version`\"")]
            ),
        ];
        for (left, right) in v {
            println!("\ninput: {:?}", left);
            println!("expected: {:?}", right);
            let args = parse_args(left);
            println!("real: {:?}", args);
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
            ("ls a\\ c", vec!["ls", "a c"]),
            ("ls \"abc\"", vec!["ls", "abc"]),
            ("ls \"Hi 你好\"", vec!["ls", "Hi 你好"]),
            ("echo \"\"", vec!["echo", ""]),
            ("echo \"hi $USER\"", vec!["echo", "hi $USER"]),
            ("echo 'hi $USER'", vec!["echo", "hi $USER"]),
            (
                "echo 'hi $USER' |  wc  -l ",
                vec!["echo", "hi $USER", "|", "wc", "-l"]
            ),
            ("echo `uname -m` | wc", vec!["echo", "uname -m", "|", "wc"]),
            (
                "echo `uptime` | wc # testing",
                vec!["echo", "uptime", "|", "wc"]
            ),
            ("awk -F \"[ ,.\\\"]+\"", vec!["awk", "-F", "[ ,.\"]+"]),
            ("echo foo\\|bar", vec!["echo", "foo|bar"]),
            ("echo \"foo\\|bar\"", vec!["echo", "foo\\|bar"]),
            ("echo 'foo\\|bar'", vec!["echo", "foo\\|bar"]),
        ];

        for (left, right) in v {
            _assert_vec_str_eq(parse_line(left), right);
        }
    }

    #[test]
    fn test_parse_commands() {
        let v = vec![
            ("ls", vec!["ls"]),
            ("ls &", vec!["ls &"]),
            ("ls -lh", vec!["ls -lh"]),
            (
                "awk -F \" \" '{print $1}' README.md",
                vec!["awk -F \" \" '{print $1}' README.md"]
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
                vec!["echo foo", "&&", "echo bar", "&&", "echo baz"]
            ),
            ("echo foo || echo bar", vec!["echo foo", "||", "echo bar"]),
            (
                "echo foo && echo bar; echo end",
                vec!["echo foo", "&&", "echo bar", ";", "echo end"]
            ),
        ];

        for (left, right) in v {
            _assert_vec_str_eq(parse_commands(left), right);
        }
    }
}
