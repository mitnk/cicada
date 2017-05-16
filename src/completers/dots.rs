use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use linefeed::Reader;
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion};
use linefeed::complete::Suffix;
use linefeed::complete::escape;
use linefeed::complete::unescape;
use linefeed::complete::escaped_word_start;

use yaml_rust::YamlLoader;
use yaml_rust::yaml;

use tools;
use parsers;

/// Performs completion by searching dotfiles
pub struct DotsCompleter;

impl<Term: Terminal> Completer<Term> for DotsCompleter {
    fn complete(&self,
                word: &str,
                reader: &Reader<Term>,
                _start: usize,
                _end: usize)
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
    let args = parsers::parser_line::parse_line(line);
    if args.is_empty() {
        return res;
    }
    let dir = tools::get_user_completer_dir();
    let dot_file = format!("{}/{}.yaml", dir, args[0]);
    let dot_file = dot_file.as_str();
    if !Path::new(dot_file).exists() {
        return res;
    }
    let mut sub_cmd = "";
    if args.len() >= 2 && !args[1].starts_with("-") && line.ends_with(" ") {
        sub_cmd = args[1].as_str();
    }
    if args.len() >= 3 && !args[1].starts_with("-") {
        sub_cmd = args[1].as_str();
    }

    let mut f = File::open(dot_file).expect("cicada: open dot_file error");
    let mut s = String::new();
    f.read_to_string(&mut s)
        .expect("cicada: read_to_string error");

    let docs;
    match YamlLoader::load_from_str(&s) {
        Ok(x) => {
            docs = x;
        }
        Err(_) => {
            println_stderr!("\ncicada: Bad Yaml file: {}?", dot_file);
            return res;
        }
    }
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
                            res.push(Completion {
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
                                            res.push(Completion {
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
                                                                res.push(Completion {
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
