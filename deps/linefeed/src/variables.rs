//! Contains types associated with user-configurable variables

use std::borrow::Cow;
use std::fmt;
use std::mem::replace;
use std::time::Duration;

/// Default `keyseq_timeout`, in milliseconds
const KEYSEQ_TIMEOUT_MS: u64 = 500;

/// Iterator over `Reader` variable values
#[derive(Clone)]
pub struct VariableIter<'a> {
    vars: &'a Variables,
    n: usize,
}

/// Represents a `Reader` variable of a given type
#[derive(Clone, Debug)]
pub enum Variable {
    /// Boolean variable
    Boolean(bool),
    /// Integer variable
    Integer(i32),
    /// String variable
    String(Cow<'static, str>),
}

impl From<bool> for Variable {
    fn from(b: bool) -> Variable {
        Variable::Boolean(b)
    }
}

impl From<i32> for Variable {
    fn from(i: i32) -> Variable {
        Variable::Integer(i)
    }
}

impl From<&'static str> for Variable {
    fn from(s: &'static str) -> Variable {
        Variable::String(s.into())
    }
}

impl From<Cow<'static, str>> for Variable {
    fn from(s: Cow<'static, str>) -> Variable {
        Variable::String(s)
    }
}

impl From<String> for Variable {
    fn from(s: String) -> Variable {
        Variable::String(s.into())
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Variable::Boolean(b) => f.write_str(if b { "on" } else { "off" }),
            Variable::Integer(n) => fmt::Display::fmt(&n, f),
            Variable::String(ref s) => fmt::Display::fmt(&s[..], f),
        }
    }
}

macro_rules! define_variables {
    ( $( $field:ident : $ty:ty => ( $name:expr , $conv:ident ,
            |$gr:ident| $getter:expr , |$sr:ident, $v:ident| $setter:expr ) , )+ ) => {
        static VARIABLE_NAMES: &[&str] = &[ $( $name ),+ ];

        pub(crate) struct Variables {
            $( pub $field : $ty ),*
        }

        impl Variables {
            pub fn get_variable(&self, name: &str) -> Option<Variable> {
                match name {
                    $( $name => {
                        let $gr = self;
                        Some(Variable::from($getter))
                    } )+
                    _ => None
                }
            }

            pub fn set_variable(&mut self, name: &str, value: &str)
                    -> Option<Variable> {
                match name {
                    $( $name => {
                        if let Some($v) = $conv(value) {
                            let $sr = self;
                            Some(Variable::from($setter))
                        } else {
                            None
                        }
                    } )+
                    _ => None
                }
            }

            pub fn iter(&self) -> VariableIter {
                VariableIter{vars: self, n: 0}
            }
        }

        impl<'a> Iterator for VariableIter<'a> {
            type Item = (&'static str, Variable);

            fn next(&mut self) -> Option<Self::Item> {
                let res = match VARIABLE_NAMES.get(self.n).cloned() {
                    $( Some($name) => ($name, {
                        let $gr = self.vars;
                        Variable::from($getter)
                    }) , )+
                    _ => return None
                };

                self.n += 1;
                Some(res)
            }
        }
    }
}

define_variables!{
    blink_matching_paren: bool => ("blink-matching-paren", parse_bool,
        |r| r.blink_matching_paren,
        |r, v| replace(&mut r.blink_matching_paren, v)),
    comment_begin: Cow<'static, str> => ("comment-begin", parse_string,
        |r| r.comment_begin.clone(),
        |r, v| replace(&mut r.comment_begin, v.into())),
    completion_display_width: usize => ("completion-display-width", parse_usize,
        |r| usize_as_i32(r.completion_display_width),
        |r, v| usize_as_i32(replace(&mut r.completion_display_width, v))),
    completion_query_items: usize => ("completion-query-items", parse_usize,
        |r| usize_as_i32(r.completion_query_items),
        |r, v| usize_as_i32(replace(&mut r.completion_query_items, v))),
    disable_completion: bool => ("disable-completion", parse_bool,
        |r| r.disable_completion,
        |r, v| replace(&mut r.disable_completion, v)),
    echo_control_characters: bool => ("echo-control-characters", parse_bool,
        |r| r.echo_control_characters,
        |r, v| replace(&mut r.echo_control_characters, v)),
    keyseq_timeout: Option<Duration> => ("keyseq-timeout", parse_duration,
        |r| as_millis(r.keyseq_timeout),
        |r, v| as_millis(replace(&mut r.keyseq_timeout, v))),
    page_completions: bool => ("page-completions", parse_bool,
        |r| r.page_completions,
        |r, v| replace(&mut r.page_completions, v)),
    print_completions_horizontally: bool => ("print-completions-horizontally", parse_bool,
        |r| r.print_completions_horizontally,
        |r, v| replace(&mut r.print_completions_horizontally, v)),
}

impl Default for Variables {
    fn default() -> Variables {
        Variables{
            blink_matching_paren: false,
            comment_begin: "#".into(),
            completion_display_width: usize::max_value(),
            completion_query_items: 100,
            disable_completion: false,
            echo_control_characters: true,
            keyseq_timeout: Some(Duration::from_millis(KEYSEQ_TIMEOUT_MS)),
            page_completions: true,
            print_completions_horizontally: false,
        }
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "0" => Some(false),
        "1" => Some(true),
        s if s.eq_ignore_ascii_case("off") => Some(false),
        s if s.eq_ignore_ascii_case("on") => Some(true),
        _ => None
    }
}

fn parse_string(s: &str) -> Option<String> {
    Some(s.to_owned())
}

fn as_millis(timeout: Option<Duration>) -> i32 {
    match timeout {
        Some(t) => {
            let s = (t.as_secs() * 1_000) as i32;
            let ms = (t.subsec_nanos() / 1_000_000) as i32;

            s + ms
        }
        None => -1
    }
}

fn parse_duration(s: &str) -> Option<Option<Duration>> {
    match s.parse::<i32>() {
        Ok(n) if n <= 0 => Some(None),
        Ok(n) => Some(Some(Duration::from_millis(n as u64))),
        Err(_) => Some(None)
    }
}

fn usize_as_i32(u: usize) -> i32 {
    match u {
        u if u > i32::max_value() as usize => -1,
        u => u as i32
    }
}

fn parse_usize(s: &str) -> Option<usize> {
    match s.parse::<i32>() {
        Ok(n) if n < 0 => Some(usize::max_value()),
        Ok(n) => Some(n as usize),
        Err(_) => None
    }
}
