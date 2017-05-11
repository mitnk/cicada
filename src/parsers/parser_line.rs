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

    #[test]
    fn test_parser_args() {
        assert_eq!(
            parser_args("ls"),
            vec![
                (String::from(""), String::from("ls")),
            ]
        );
        assert_eq!(
            parser_args("  ls  "),
            vec![
                (String::from(""), String::from("ls")),
            ]
        );
        assert_eq!(
            parser_args("ls -lh"),
            vec![
                ("".to_string(), "ls".to_string()),
                ("".to_string(), "-lh".to_string()),
            ]
        );
        assert_eq!(
            parser_args(" ls    -lh  "),
            vec![
                ("".to_string(), "ls".to_string()),
                ("".to_string(), "-lh".to_string()),
            ]
        );
        assert_eq!(
            parser_args("ls 'abc'"),
            vec![
                ("".to_string(), "ls".to_string()),
                ("\'".to_string(), "abc".to_string()),
            ]
        );
        assert_eq!(
            parser_args("ls \"abc\""),
            vec![
                ("".to_string(), "ls".to_string()),
                ("\"".to_string(), "abc".to_string()),
            ]
        );
        assert_eq!(
            parser_args("echo \"hi $USER\""),
            vec![
                ("".to_string(), "echo".to_string()),
                ("\"".to_string(), "hi $USER".to_string()),
            ]
        );
        assert_eq!(
            parser_args("echo \'hi $USER\'"),
            vec![
                ("".to_string(), "echo".to_string()),
                ("\'".to_string(), "hi $USER".to_string()),
            ]
        );
        assert_eq!(
            parser_args(" echo  \'hi $USER\'  |  wc  -l "),
            vec![
                ("".to_string(), "echo".to_string()),
                ("\'".to_string(), "hi $USER".to_string()),
                ("".to_string(), "|".to_string()),
                ("".to_string(), "wc".to_string()),
                ("".to_string(), "-l".to_string()),
            ]
        );
        assert_eq!(
            parser_args("echo `uname -m` | wc"),
            vec![
                ("".to_string(), "echo".to_string()),
                ("`".to_string(), "uname -m".to_string()),
                ("".to_string(), "|".to_string()),
                ("".to_string(), "wc".to_string()),
            ]
        );
        assert_eq!(
            parser_args("echo '`uname -m`'"),
            vec![
                ("".to_string(), "echo".to_string()),
                ("'".to_string(), "`uname -m`".to_string()),
            ]
        );
        assert_eq!(
            parser_args("'\"\"\"\"'"),
            vec![
                ("'".to_string(), "\"\"\"\"".to_string()),
            ]
        );
        assert_eq!(
            parser_args("\"\'\'\'\'\""),
            vec![
                ("\"".to_string(), "''''".to_string()),
            ]
        );
    }
}
