use std::borrow::Cow;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use linefeed::complete::escape;
use linefeed::complete::escaped_word_start;
use linefeed::complete::unescape;
use linefeed::complete::Suffix;
use linefeed::complete::{Completer, Completion};
use linefeed::prompter::Prompter;
use linefeed::terminal::Terminal;
use yaml_rust::{Yaml, YamlLoader};
use yaml_rust::yaml::Hash;

use crate::execute;
use crate::parsers;
use crate::tools;

/// Performs completion by searching dotfiles
pub struct DotsCompleter;

impl<Term: Terminal> Completer<Term> for DotsCompleter {
    fn complete(
        &self,
        word: &str,
        reader: &Prompter<Term>,
        _start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let line = reader.buffer();
        Some(complete_dots(line, word))
    }

    fn word_start(&self, line: &str, end: usize, _reader: &Prompter<Term>) -> usize {
        escaped_word_start(&line[..end])
    }

    fn quote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        escape(word)
    }

    fn unquote<'a>(&self, word: &'a str) -> Cow<'a, str> {
        unescape(word)
    }
}

fn get_dot_file(line: &str) -> (String, String) {
    let args = parsers::parser_line::line_to_plain_tokens(line);
    let dir = tools::get_user_completer_dir();
    let dot_file = format!("{}/{}.yaml", dir, args[0]);
    if !Path::new(&dot_file).exists() {
        return (String::new(), String::new());
    }
    let sub_cmd = if (args.len() >= 3 && !args[1].starts_with('-'))
        || (args.len() >= 2 && !args[1].starts_with('-') && line.ends_with(' '))
    {
        args[1].as_str()
    } else {
        ""
    };

    (dot_file, sub_cmd.to_string())
}

fn handle_lv1_string(res: &mut Vec<Completion>,
                     value: &str, word: &str) {
    if !value.starts_with(word) && !value.starts_with('`') {
        return;
    }

    let linfo = parsers::parser_line::parse_line(value);
    let tokens = linfo.tokens;
    if tokens.len() == 1 && tokens[0].0 == "`" {
        log!("run subcmd: {:?}", &tokens[0].1);
        let cr = execute::run(&tokens[0].1);
        let v: Vec<&str> = cr.stdout.split(|c| c == '\n' || c == ' ').collect();
        for s in v {
            if s.trim().is_empty() {
                continue;
            }
            handle_lv1_string(res, s, word);
        }
        return;
    }

    let display = None;
    let suffix = Suffix::Default;
    res.push(Completion {
        completion: value.to_string(),
        display,
        suffix,
    });
}

fn handle_lv1_hash(res: &mut Vec<Completion>,
                   h: &Hash, word: &str) {
    for v in h.values() {
        if let Yaml::Array(ref arr) = v {
            for s in arr {
                if let Yaml::String(value) = s {
                    if !value.starts_with(word) && !value.starts_with('`') {
                        continue;
                    }
                    handle_lv1_string(res, value, word);
                }
            }
        }
    }
}

fn complete_dots(line: &str, word: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    if line.trim().is_empty() {
        return res;
    }
    let (dot_file, sub_cmd) = get_dot_file(line);
    if dot_file.is_empty() {
        return res;
    }

    let mut f;
    match File::open(&dot_file) {
        Ok(x) => f = x,
        Err(e) => {
            println_stderr!("\ncicada: open dot_file error: {:?}", e);
            return res;
        }
    }

    let mut s = String::new();
    match f.read_to_string(&mut s) {
        Ok(_) => {}
        Err(e) => {
            println_stderr!("\ncicada: read_to_string error: {:?}", e);
            return res;
        }
    }

    let docs = match YamlLoader::load_from_str(&s) {
        Ok(x) => x,
        Err(e) => {
            println_stderr!("\ncicada: Bad Yaml file: {}: {:?}", dot_file, e);
            return res;
        }
    };

    for doc in docs.iter() {
        match *doc {
            Yaml::Array(ref v) => {
                for x in v {
                    match *x {
                        Yaml::String(ref name) => {
                            if !sub_cmd.is_empty() {
                                continue;
                            }
                            handle_lv1_string(&mut res, name, word);
                        }
                        Yaml::Hash(ref h) => {
                            if sub_cmd.is_empty() {
                                for k in h.keys() {
                                    if let Yaml::String(value) = k {
                                        handle_lv1_string(&mut res, value, word);
                                    }
                                }
                            } else {
                                let key = Yaml::from_str(&sub_cmd);
                                if !h.contains_key(&key) {
                                    continue;
                                }
                                handle_lv1_hash(&mut res, h, word);
                            }
                        }
                        _ => {
                            println_stderr!("\nThis yaml file is in bad format: {}", dot_file);
                        }
                    }
                }
            }
            _ => {
                println_stderr!("\nThis yaml file is in bad format: {}", dot_file);
            }
        }
    }
    res
}
