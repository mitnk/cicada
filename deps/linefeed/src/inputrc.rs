//! Parses configuration files in the format of GNU Readline `inputrc`

use std::char::from_u32;
use std::fs::File;
use std::io::{stderr, Read, Write};
use std::path::Path;
use std::str::{Chars, Lines};

use crate::chars::{ctrl, meta, parse_char_name};
use crate::command::Command;

/// Parsed configuration directive
#[derive(Clone, Debug)]
pub enum Directive {
    /// Bind construct; `"input-sequence": command-or-macro`
    Bind(String, Command),
    /// Conditional construct;
    /// (`$if name=value` or `$if value`) *directives*
    /// (optional `$else` *directives*) `$endif`
    Conditional{
        /// Value name; if `None`, value refers to application name
        name: Option<String>,
        /// Value to compare
        value: String,
        /// Group of directives evaluated when condition is true
        then_group: Vec<Directive>,
        /// Group of directives evaluated when condition is false
        else_group: Vec<Directive>,
    },
    /// Set variable; `set name value`
    SetVariable(String, String),
}

/// Parses the named file and returns contained directives.
///
/// If the file cannot be opened, `None` is returned and an error is printed
/// to `stderr`. If any errors are encountered during parsing, they are printed
/// to `stderr`.
pub fn parse_file<P: ?Sized>(filename: &P) -> Option<Vec<Directive>>
        where P: AsRef<Path> {
    let filename = filename.as_ref();

    let mut f = match File::open(filename) {
        Ok(f) => f,
        Err(e) => {
            let _ = writeln!(stderr(), "linefeed: {}: {}", filename.display(), e);
            return None;
        }
    };

    let mut buf = String::new();

    if let Err(e) = f.read_to_string(&mut buf) {
        let _ = writeln!(stderr(), "{}: {}", filename.display(), e);
        return None;
    }

    Some(parse_text(filename, &buf))
}

/// Parses some text and returns contained directives.
///
/// If any errors are encountered during parsing, they are printed to `stderr`.
pub fn parse_text<P: ?Sized>(name: &P, line: &str) -> Vec<Directive>
        where P: AsRef<Path> {
    let mut p = Parser::new(name.as_ref(), line);
    p.parse()
}

struct Parser<'a> {
    lines: Lines<'a>,
    filename: &'a Path,
    line_num: usize,
}

enum Token<'a> {
    /// Colon; `:`
    Colon,
    /// Equal; `=`
    Equal,
    /// Conditional or other special directive; `$word`
    SpecialWord(&'a str),
    /// Double-quoted string; `"foo"`
    String(String),
    /// Bare word; `foo`
    Word(&'a str),
    /// Invalid token
    Invalid,
}

impl<'a> Parser<'a> {
    pub fn new(filename: &'a Path, text: &'a str) -> Parser<'a> {
        Parser{
            lines: text.lines(),
            filename: filename,
            line_num: 0,
        }
    }

    fn next_line(&mut self) -> Option<&'a str> {
        self.lines.next().map(|line| {
            self.line_num += 1;
            line.trim()
        })
    }

    fn parse(&mut self) -> Vec<Directive> {
        let mut dirs = Vec::new();

        while let Some(line) = self.next_line() {
            if line.starts_with('#') {
                continue;
            }

            let mut tokens = Tokens::new(line);

            if let Some(Token::SpecialWord("include")) = tokens.next() {
                let path = tokens.line;

                if let Some(d) = parse_file(Path::new(path)) {
                    dirs.extend(d);
                }

                continue;
            }

            if let Some(dir) = self.parse_line(line) {
                dirs.push(dir);
            }
        }

        dirs
    }

    fn parse_conditional(&mut self) -> (Vec<Directive>, Vec<Directive>) {
        let mut then_group = Vec::new();
        let mut else_group = Vec::new();
        let mut parse_else = false;

        loop {
            let line = match self.next_line() {
                Some(line) => line,
                None => {
                    self.error("missing $endif directive");
                    break;
                }
            };

            if line.starts_with('#') {
                continue;
            }

            let mut tokens = Tokens::new(line);

            let start = match tokens.next() {
                Some(tok) => tok,
                None => continue
            };

            match start {
                Token::SpecialWord("else") => {
                    if parse_else {
                        self.error("duplicate $else directive");
                    } else {
                        parse_else = true;
                    }
                }
                Token::SpecialWord("endif") => {
                    break;
                }
                _ => {
                    if let Some(dir) = self.parse_line(line) {
                        if parse_else {
                            else_group.push(dir);
                        } else {
                            then_group.push(dir);
                        }
                    }
                }
            }
        }

        (then_group, else_group)
    }

    fn parse_line(&mut self, line: &str) -> Option<Directive> {
        let mut tokens = Tokens::new(line);

        let start = tokens.next()?;

        let dir = match start {
            Token::SpecialWord("if") => {
                let name = match tokens.next() {
                    Some(Token::Word(w)) => w,
                    _ => {
                        self.invalid();
                        return None;
                    }
                };

                let (name, value) = match tokens.next() {
                    Some(Token::Equal) => {
                        let value = match tokens.next() {
                            Some(Token::Word(w)) => w,
                            None => "",
                            _ => {
                                self.invalid();
                                return None;
                            }
                        };

                        (Some(name), value)
                    }
                    None => (None, name),
                    _ => {
                        self.invalid();
                        return None;
                    }
                };

                let (then_group, else_group) = self.parse_conditional();

                Directive::Conditional{
                    name: name.map(|s| s.to_owned()),
                    value: value.to_owned(),
                    then_group: then_group,
                    else_group: else_group,
                }
            }
            Token::SpecialWord("else") => {
                self.error("$else without matching $if directive");
                return None;
            }
            Token::SpecialWord("endif") => {
                self.error("$endif without matching $if directive");
                return None;
            }
            Token::String(seq) => {
                match tokens.next() {
                    Some(Token::Colon) => (),
                    _ => {
                        self.invalid();
                        return None;
                    }
                }

                match tokens.next() {
                    Some(Token::Word(value)) =>
                        Directive::Bind(seq, Command::from_string(value)),
                    Some(Token::String(out)) =>
                        Directive::Bind(seq, Command::Macro(out.to_owned().into())),
                    _ => {
                        self.invalid();
                        return None;
                    }
                }
            }
            Token::Word("set") => {
                let name = match tokens.next() {
                    Some(Token::Word(w)) => w,
                    _ => {
                        self.invalid();
                        return None;
                    }
                };

                let rest = tokens.line;

                let value = match tokens.next() {
                    Some(Token::String(s)) => s,
                    Some(Token::Word(_)) => rest.to_owned(),
                    _ => {
                        self.invalid();
                        return None;
                    }
                };

                Directive::SetVariable(name.to_owned(), value)
            }
            Token::Word(name) => {
                match tokens.next() {
                    Some(Token::Colon) => (),
                    _ => {
                        self.invalid();
                        return None;
                    }
                }

                let seq = match parse_char_name(name) {
                    Some(seq) => seq,
                    None => {
                        self.invalid();
                        return None;
                    }
                };

                match tokens.next() {
                    Some(Token::Word(value)) =>
                        Directive::Bind(seq, Command::from_string(value)),
                    Some(Token::String(macro_seq)) =>
                        Directive::Bind(seq, Command::Macro(macro_seq.to_owned().into())),
                    _ => {
                        self.invalid();
                        return None;
                    }
                }
            }
            _ => {
                self.invalid();
                return None;
            }
        };

        Some(dir)
    }

    fn error(&self, msg: &str) {
        let _ = writeln!(stderr(),
            "linefeed: {} line {}: {}", self.filename.display(), self.line_num, msg);
    }

    fn invalid(&self) {
        self.error("invalid directive");
    }
}

struct Tokens<'a> {
    line: &'a str,
}

impl<'a> Tokens<'a> {
    fn new(line: &str) -> Tokens {
        Tokens{
            line: line,
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        let ch = self.line.chars().next()?;

        let tok = match ch {
            ':' => {
                self.line = self.line[1..].trim_start();
                Token::Colon
            }
            '=' => {
                self.line = self.line[1..].trim_start();
                Token::Equal
            }
            '$' => {
                let (word, rest) = parse_word(&self.line[1..]);
                self.line = rest.trim_start();
                Token::SpecialWord(word)
            }
            '"' => {
                let (tok, rest) = parse_string(self.line);
                self.line = rest.trim_start();
                tok
            }
            _ => {
                let (word, rest) = parse_word(self.line);
                self.line = rest.trim_start();
                Token::Word(word)
            }
        };

        Some(tok)
    }
}

fn parse_escape(chars: &mut Chars) -> Option<String> {
    let ch = chars.next()?;

    let esc = match ch {
        'C'  => {
            match chars.next() {
                Some('-') => (),
                _ => return None
            }
            ctrl(chars.next()?)
        }
        'M'  => {
            match chars.next() {
                Some('-') => (),
                _ => return None
            }
            return Some(meta(chars.next()?));
        }
        'e'  => '\x1b',
        '\\' => '\\',
        '"'  => '"',
        '\'' => '\'',
        'a'  => '\x07',
        'b'  => '\x08',
        'd'  => '\x7f',
        'f'  => '\x0c',
        'n'  => '\n',
        'r'  => '\r',
        't'  => '\t',
        'u'  => {
            match chars.next() {
                Some('{') => (),
                _ => return None
            }

            let mut n = 0;

            for _ in 0..6 {
                match chars.clone().next().and_then(|ch| ch.to_digit(16)) {
                    Some(digit) => {
                        chars.next();
                        n *= 16;
                        n += digit;
                    }
                    None => break
                }
            }

            match chars.next() {
                Some('}') => (),
                _ => return None
            }

            from_u32(n)?
        }
        'v'  => '\x0b',
        'x'  => {
            let mut n = 0;

            for _ in 0..2 {
                // Peek the next character
                let digit = chars.clone().next()?.to_digit(16)? as u8;

                // Consume if valid
                chars.next();

                n <<= 4;
                n |= digit;
            }

            n as char
        }
        '0' ..= '3' => {
            let mut n = ch as u8 - b'0';

            for _ in 0..2 {
                // Peek the next character
                let digit = chars.clone().next()?.to_digit(8)? as u8;

                // Consume if valid
                chars.next();

                n <<= 3;
                n |= digit;
            }

            n as char
        }
        _ => return None
    };

    Some(esc.to_string())
}

fn parse_string(s: &str) -> (Token, &str) {
    let mut chars = s.chars();
    let mut res = String::new();

    // Skip open quote
    chars.next();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => return (Token::String(res), chars.as_str()),
            '\\' => {
                match parse_escape(&mut chars) {
                    Some(esc) => {
                        res.push_str(&esc);
                    }
                    None => break
                }
            }
            ch => res.push(ch)
        }
    }

    (Token::Invalid, "")
}

fn parse_word(s: &str) -> (&str, &str) {
    let mut chars = s.char_indices();

    loop {
        let mut clone = chars.clone();

        match clone.next() {
            Some((ind, ch)) if ch == ':' || ch == '"' || ch == '=' ||
                    ch.is_whitespace() => {
                return (&s[..ind], &s[ind..]);
            }
            None => {
                return (s, "");
            }
            _ => ()
        }

        chars = clone;
    }
}

#[cfg(test)]
mod test {
    use super::{Directive, parse_text};
    use crate::command::Command;

    fn one<T>(v: Vec<T>) -> T {
        assert_eq!(v.len(), 1);
        v.into_iter().next().unwrap()
    }

    #[test]
    fn test_parse() {
        assert_matches!(one(parse_text("<test>", "Ctrl-k: kill-line")),
            Directive::Bind(ref seq, Command::KillLine)
                if seq == "\x0b");
        assert_matches!(one(parse_text("<test>", r#""foo": "bar""#)),
            Directive::Bind(ref seq, Command::Macro(ref mac))
                if seq == "foo" && mac == "bar");

        assert_matches!(one(parse_text("<test>", "set foo bar")),
            Directive::SetVariable(ref name, ref val)
                if name == "foo" && val == "bar");

        assert_matches!(one(parse_text("<test>", r#""\xab": "a""#)),
            Directive::Bind(ref seq, Command::Macro(ref mac))
                if seq == "\u{ab}" && mac == "a");

        assert_matches!(one(parse_text("<test>", r#""\u{2022}": "b""#)),
            Directive::Bind(ref seq, Command::Macro(ref mac))
                if seq == "\u{2022}" && mac == "b");

        assert_matches!(one(parse_text("<test>", r#""\123": "c""#)),
            Directive::Bind(ref seq, Command::Macro(ref mac))
                if seq == "S" && mac == "c");
    }

    #[test]
    fn test_conditional() {
        let cond = one(parse_text("<test>", "
            $if foo=bar
                set var 123
            $endif
            "));

        assert_matches!(cond,
            Directive::Conditional{name: Some(ref name), ref value, ..}
                if name == "foo" && value == "bar");

        if let Directive::Conditional{then_group, else_group, ..} = cond {
            assert_matches!(one(then_group),
                Directive::SetVariable(ref name, ref val)
                    if name == "var" && val == "123");
            assert!(else_group.is_empty());
        }
    }

    #[test]
    fn test_conditional_else() {
        let cond = one(parse_text("<test>", "
            $if foo
                set var 123
            $else
                Tab: tab-insert
            $endif
            "));

        assert_matches!(cond,
            Directive::Conditional{name: None, ref value, ..}
                if value == "foo");

        if let Directive::Conditional{then_group, else_group, ..} = cond {
            assert_matches!(one(then_group),
                Directive::SetVariable(ref name, ref val)
                    if name == "var" && val == "123");

            assert_matches!(one(else_group),
                Directive::Bind(ref seq, Command::TabInsert)
                    if seq == "\t");
        }
    }
}
