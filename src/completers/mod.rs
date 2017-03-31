use std::rc::Rc;

use linefeed::Reader;
use linefeed::complete::{Completer, Completion, PathCompleter};
use linefeed::terminal::Terminal;

pub struct DemoCompleter;

impl<Term: Terminal> Completer<Term> for DemoCompleter {
    fn complete(&self, word: &str, reader: &Reader<Term>,
            start: usize, _end: usize) -> Option<Vec<Completion>> {
        let cpl = Rc::new(PathCompleter);
        return cpl.complete(word, reader, start, _end);
    }
}
