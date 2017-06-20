use std::path::Path;
use std::rc::Rc;

use linefeed::Reader;
use linefeed::complete::{Completer, Completion};
use linefeed::terminal::Terminal;
use regex::Regex;

pub mod dots;
pub mod path;
pub struct CCDCompleter;

use tools;
use parsers;

fn for_cd(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"^ *cd +") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
}

fn for_bin(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"(^ *[a-zA-Z0-9_\.-]+$)|(^.+\| +[a-zA-Z0-9_\.-]+$)") {
        re = x;
    } else {
        return false;
    }
    re.is_match(line)
}

fn for_dots(line: &str) -> bool {
    let args = parsers::parser_line::parse_line(line);
    let len = args.len();
    if len == 0 {
        return false;
    }
    let dir = tools::get_user_completer_dir();
    let dot_file = format!("{}/{}.yaml", dir, args[0]);
    Path::new(dot_file.as_str()).exists()
}

impl<Term: Terminal> Completer<Term> for CCDCompleter {
    fn complete(
        &self,
        word: &str,
        reader: &Reader<Term>,
        start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        let line = reader.buffer();
        if for_bin(line) {
            let cpl = Rc::new(path::BinCompleter);
            return cpl.complete(word, reader, start, _end);
        }
        if for_cd(line) {
            let cpl = Rc::new(path::CdCompleter);
            return cpl.complete(word, reader, start, _end);
        }
        if for_dots(line) {
            let cpl = Rc::new(dots::DotsCompleter);
            if let Some(x) = cpl.complete(word, reader, start, _end) {
                if !x.is_empty() {
                    return Some(x);
                }
            }
        }
        let cpl = Rc::new(path::PathCompleter);
        cpl.complete(word, reader, start, _end)
    }
}
