use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use linefeed::Reader;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion};
use linefeed::complete::Suffix;
use linefeed::complete::escape;
use linefeed::complete::unescape;
use linefeed::complete::escaped_word_start;

use shlex;
use yaml_rust::{YamlLoader};
use yaml_rust::yaml;

use tools;

/// Performs completion by searching dotfiles
pub struct DotsCompleter;

impl<Term: Terminal> Completer<Term> for DotsCompleter {
    fn complete(&self, word: &str, reader: &Reader<Term>, _start: usize, _end: usize)
            -> Option<Vec<Completion>> {
        let line = reader.buffer();
        Some(complete_dots(line, word))
    }

    fn word_start(&self, line: &str, end: usize, _reader: &Reader<Term>) -> usize {
        escaped_word_start(&line[..end])
    }

    fn quote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        escape(word)
    }

    fn unquote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        unescape(word)
    }
}

fn complete_dots(line: &str, word: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let args;
    if let Some(x) = shlex::split(line.trim()) {
        args = x;
    } else {
        return res;
    }
    if args.len() == 0 {
        return res;
    }
    let dir = tools::get_user_completer_dir();
    let dot_file = format!("{}/{}.yaml", dir, args[0]);
    if !Path::new(dot_file.as_str()).exists() {
        return res;
    }
    let mut sub_cmd = "";
    if args.len() >= 2 && !args[1].starts_with("-") && line.ends_with(" ") {
        sub_cmd = args[1].as_str();
    }
    if args.len() >= 3 && !args[1].starts_with("-") {
        sub_cmd = args[1].as_str();
    }

    let mut f = File::open(dot_file).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let docs = YamlLoader::load_from_str(&s).unwrap();
    for doc in &docs {
        match doc {
            &yaml::Yaml::Array(ref v) => {
                for x in v {
                    match x {
                        &yaml::Yaml::String(ref name) => {
                            if sub_cmd != "" || !name.starts_with(word) {
                                continue;
                            }

                            let display = None;
                            let suffix = Suffix::Default;
                            res.push(Completion{
                                completion: name.to_string(),
                                display: display,
                                suffix: suffix,
                            });
                        }
                        &yaml::Yaml::Hash(ref h) => {
                            for (k, v) in h.iter() {
                                // println!("k:{:?} v:{:?}", k, v);
                                match k {
                                    &yaml::Yaml::String(ref name) => {
                                        if sub_cmd != "" && sub_cmd != name {
                                            continue;
                                        }
                                        if sub_cmd == "" {
                                            if !name.starts_with(word) {
                                                continue;
                                            }

                                            let name = name.clone();
                                            let display = None;
                                            let suffix = Suffix::Default;
                                            res.push(Completion{
                                                completion: name,
                                                display: display,
                                                suffix: suffix,
                                            });
                                        } else {
                                            match v {
                                                &yaml::Yaml::Array(ref v) => {
                                                    for x in v {
                                                        match x {
                                                            &yaml::Yaml::String(ref name) => {
                                                                if !name.starts_with(word) {
                                                                    continue;
                                                                }

                                                                let name = name.clone();
                                                                let display = None;
                                                                let suffix = Suffix::Default;
                                                                res.push(Completion{
                                                                    completion: name,
                                                                    display: display,
                                                                    suffix: suffix,
                                                                });
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                println!("Found unknown yaml doc");
            }
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use nom::IResult;
    use super::complete_dots;

    #[test]
    fn dots_test() {
        complete_dots("abc");
        assert_eq!(1, 2);
    }
}
