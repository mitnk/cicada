pub fn parser_args(line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut sep = String::new();
    let mut token = String::new();
    for c in line.chars() {
        if c == ' ' {
            if !sep.is_empty() {
                token.push(c);
                continue;
            }
            if token.is_empty() {
                continue;
            } else if sep.is_empty() {
                result.push((String::from(""), token));
                token = String::new();
                continue;
            } else {
                continue;
            }
        }
        if c == '\'' || c == '"' || c == '`' {
            if sep == "" {
                sep.push(c);
                continue;
            } else if sep == c.to_string() {
                result.push((c.to_string(), token));
                sep = String::new();
                token = String::new();
                continue;
            } else {
                token.push(c);
            }
        } else {
            token.push(c);
        }
    }
    if !token.is_empty() {
        result.push((String::from(""), token));
    }
    return result;
}

#[cfg(test)]
mod tests {
    use super::parser_args;

    fn _assert_vec_eq(a: Vec<(String, String)>, b: Vec<(&str, &str)>) {
        assert_eq!(a.len(), b.len());
        for (i, item) in a.iter().enumerate() {
            let (ref l, ref r) = *item;
            assert_eq!(l, b[i].0);
            assert_eq!(r, b[i].1);
        }
    }

    #[test]
    fn test_parser_args() {
        let v = vec![
            ("ls", vec![("", "ls")]),
            ("  ls   ", vec![("", "ls")]),
            ("ls -lh", vec![("", "ls"), ("", "-lh")]),
            ("  ls   -lh   ", vec![("", "ls"), ("", "-lh")]),
            ("ls 'abc'", vec![("", "ls"), ("'", "abc")]),
            ("ls \"abc\"", vec![("", "ls"), ("\"", "abc")]),
            ("echo \"hi $USER\"", vec![("", "echo"), ("\"", "hi $USER")]),
            ("echo 'hi $USER'", vec![("", "echo"), ("'", "hi $USER")]),
            ("echo 'hi $USER' |  wc  -l ", vec![("", "echo"),
                                                ("'", "hi $USER"),
                                                ("", "|"),
                                                ("", "wc"),
                                                ("", "-l")]),
            ("echo `uname -m` | wc", vec![("", "echo"),
                                                ("`", "uname -m"),
                                                ("", "|"),
                                                ("", "wc")]),
            ("echo '`uname -m`'", vec![("", "echo"), ("'", "`uname -m`")]),
            ("'\"\"\"\"'", vec![("'", "\"\"\"\"")]),
            ("\"\'\'\'\'\"", vec![("\"", "''''")]),
        ];
        for (left, right) in v {
            _assert_vec_eq(parser_args(left), right);
        }
    }
}
