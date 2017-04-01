use std::rc::Rc;

use linefeed::Reader;
use linefeed::complete::{Completer, Completion};
use linefeed::terminal::Terminal;
use regex::Regex;

pub mod path;
pub struct DemoCompleter;

fn for_bin(line: &str) -> bool {
    let re;
    if let Ok(x) = Regex::new(r"(^ *[a-zA-Z0-9_\.-]+$)|(^.+\| +[a-zA-Z0-9_\.-]+$)") {
        re = x;
    } else {
        return false;
    }
    return re.is_match(line);
}

impl<Term: Terminal> Completer<Term> for DemoCompleter {
    fn complete(&self, word: &str, reader: &Reader<Term>,
            start: usize, _end: usize) -> Option<Vec<Completion>> {
        let line = reader.buffer();
        if for_bin(line) {
            let cpl = Rc::new(path::BinCompleter);
            return cpl.complete(word, reader, start, _end);
        } else {
            let cpl = Rc::new(path::PathCompleter);
            return cpl.complete(word, reader, start, _end);
        }
    }
}
