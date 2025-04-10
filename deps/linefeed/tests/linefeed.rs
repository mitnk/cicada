extern crate linefeed;

#[macro_use] extern crate assert_matches;

use std::env::set_var;
use std::io;
use std::sync::Arc;

use linefeed::{Command, Completer, Completion, Interface, Prompter, ReadResult};
use linefeed::memory::MemoryTerminal;
use linefeed::terminal::{Size, Terminal};

const UP_ARROW: &str = "\x1b[A";
const DOWN_ARROW: &str = "\x1b[B";
const RIGHT_ARROW: &str = "\x1b[C";
const LEFT_ARROW: &str = "\x1b[D";
const HOME: &str = "\x1b[H";
const INSERT: &str = "\x1b[2~";
const DELETE: &str = "\x1b[3~";

fn test(input: &str) -> (MemoryTerminal, Interface<MemoryTerminal>) {
    let term = MemoryTerminal::with_size(Size{columns: 20, lines: 5});

    term.push_input(input);

    // Skip reading inputrc configurations
    set_var("INPUTRC", "");

    let interface = Interface::with_term("test", term.clone()).unwrap();

    interface.set_prompt("$ ").unwrap();

    (term, interface)
}

fn assert_lines(term: &MemoryTerminal, tests: &[&str]) {
    let mut lines = term.lines();
    let mut tests = tests.iter();

    while let Some(line) = lines.next() {
        let test = match tests.next() {
            Some(test) => test,
            None => ""
        };

        let end = match line.iter().rposition(|&ch| ch != ' ') {
            Some(pos) => pos + 1,
            None => 0
        };

        if line[..end].iter().cloned().ne(test.chars()) {
            let line = line[..end].iter().cloned().collect::<String>();
            panic!("terminal line doesn't match: line={:?}; test={:?}", line, test);
        }
    }
}

fn assert_read<T: Terminal>(r: &Interface<T>, line: &str) {
    assert_matches!(r.read_line(), Ok(ReadResult::Input(ref s)) if s == line);
}

#[test]
fn test_eof() {
    let (term, r) = test("\x04");

    assert_matches!(r.read_line(), Ok(ReadResult::Eof));

    term.push_input("foo\x04\n");
    assert_read(&r, "foo");

    assert_lines(&term, &["$", "$ foo"]);
}

#[test]
fn test_quote() {
    let (term, r) = test("\x16\x03\n");

    assert_read(&r, "\x03");
    assert_lines(&term, &["$ ^C"]);
}

#[test]
fn test_insert() {
    let (term, r) = test("abc\n");

    assert_read(&r, "abc");
    assert_lines(&term, &["$ abc"]);
}

struct TestCompleter(Vec<&'static str>);

impl<Term: Terminal> Completer<Term> for TestCompleter {
    fn complete(&self, _word: &str, _reader: &Prompter<Term>,
            _start: usize, _end: usize) -> Option<Vec<Completion>> {
        Some(self.0.clone().into_iter()
            .map(|s| Completion::simple(s.to_owned())).collect())
    }
}

#[test]
fn test_complete() {
    let (term, r) = test("hi foo\t\t\n");

    r.set_completer(Arc::new(TestCompleter(vec!["foobar", "foobaz"])));

    assert_read(&r, "hi fooba");
    assert_lines(&term, &["$ hi fooba", "foobar  foobaz", "$ hi fooba"]);

    term.clear_all();
    term.push_input("hi foo\x1b?\n");

    assert_read(&r, "hi foo");
    assert_lines(&term, &["$ hi foo", "foobar  foobaz", "$ hi foo"]);

    term.clear_all();
    term.push_input("hi foo\x1b*\n");

    assert_read(&r, "hi foobar foobaz ");
    assert_lines(&term, &["$ hi foobar foobaz"]);
}

fn fn_foo<Term: Terminal>(reader: &mut Prompter<Term>, count: i32, ch: char)
        -> io::Result<()> {
    assert_eq!(count, 1);
    assert_eq!(ch, '\x18');
    assert!(!reader.explicit_arg());

    reader.insert_str("foo")
}

fn fn_bar<Term: Terminal>(reader: &mut Prompter<Term>, count: i32, ch: char)
        -> io::Result<()> {
    assert_eq!(count, 2);
    assert_eq!(ch, '\x19');
    assert!(reader.explicit_arg());

    reader.insert_str("bar")
}

#[test]
fn test_function() {
    let (term, r) = test("");

    r.define_function("fn-foo", Arc::new(fn_foo));
    r.bind_sequence("\x18", Command::from_str("fn-foo"));

    r.define_function("fn-bar", Arc::new(fn_bar));
    r.bind_sequence("\x19", Command::from_str("fn-bar"));

    term.push_input("\x18\n");
    assert_read(&r, "foo");

    term.push_input("\x1b2\x19\n");
    assert_read(&r, "bar");

    assert_lines(&term, &["$ foo", "$ bar"]);
}

#[test]
fn test_macro() {
    let (term, r) = test("");

    r.bind_sequence("A", Command::Macro("foo"     .into()));
    r.bind_sequence("B", Command::Macro("barCquux".into()));
    r.bind_sequence("C", Command::Macro("baz"     .into()));

    term.push_input("A\n");
    assert_read(&r, "foo");

    term.push_input("B\n");
    assert_read(&r, "barbazquux");

    assert_lines(&term, &["$ foo", "$ barbazquux"]);
}

#[test]
fn test_comment() {
    let (term, r) = test("lol\x1b#");

    assert_read(&r, "#lol");
    assert_lines(&term, &["$ #lol"]);

    term.clear_all();
    term.push_input("#wut\x1b-\x1b#");

    assert_read(&r, "wut");
    assert_lines(&term, &["$ wut"]);
}

#[test]
fn test_arrows() {
    let (term, r) = test("abcde");

    term.push_input(LEFT_ARROW);
    term.push_input("x");
    term.push_input(HOME);
    term.push_input("y");
    term.push_input(RIGHT_ARROW);
    term.push_input("z\n");
    assert_read(&r, "yazbcdxe");

    term.push_input("abcde");
    term.push_input("\x1b3");
    term.push_input(LEFT_ARROW);
    term.push_input("x\n");
    assert_read(&r, "abxcde");

    assert_lines(&term, &["$ yazbcdxe", "$ abxcde"]);
}

#[test]
fn test_digit() {
    let (term, r) = test("");

    term.push_input("\x1b10.\n");
    assert_read(&r, "..........");

    assert_lines(&term, &["$ .........."]);
}

#[test]
fn test_search_char() {
    let (term, r) = test("lolwut");

    term.push_input("\x1b\x1dw.");
    term.push_input("\x1b2\x1b\x1dl,");
    term.push_input("\x1do:\n");
    assert_read(&r, ",l:ol.wut");

    term.push_input("alpha");
    term.push_input(HOME);
    term.push_input("\x1dax\n");
    assert_read(&r, "alphxa");

    assert_lines(&term, &["$ ,l:ol.wut", "$ alphxa"]);
}

#[test]
fn test_delete() {
    let (term, r) = test("sup");

    term.push_input(LEFT_ARROW);
    term.push_input(LEFT_ARROW);
    term.push_input(DELETE);
    term.push_input("\n");

    assert_read(&r, "sp");

    term.push_input("sup");
    term.push_input(LEFT_ARROW);
    term.push_input(LEFT_ARROW);
    term.push_input("\x7f\n");

    assert_read(&r, "up");

    assert_lines(&term, &["$ sp", "$ up"]);
}

#[test]
fn test_history() {
    let (term, r) = test("");

    r.add_history("alpha".to_owned());
    r.add_history("bravo".to_owned());
    r.add_history("charlie".to_owned());
    r.add_history("delta".to_owned());

    term.push_input(UP_ARROW);
    term.push_input("\n");

    assert_read(&r, "delta");

    term.push_input(UP_ARROW);
    term.push_input(UP_ARROW);
    term.push_input("\n");

    assert_read(&r, "charlie");

    term.push_input("foo");
    term.push_input(UP_ARROW);
    term.push_input(DOWN_ARROW);
    term.push_input("\n");

    assert_read(&r, "foo");

    assert_lines(&term, &["$ delta", "$ charlie", "$ foo"]);
}

#[test]
fn test_history_mod() {
    let (term, r) = test("");

    r.add_history("alpha".to_owned());
    r.add_history("bravo".to_owned());
    r.add_history("charlie".to_owned());
    r.add_history("delta".to_owned());

    term.push_input(UP_ARROW);
    term.push_input("x");
    term.push_input(UP_ARROW);
    term.push_input("x\n");

    assert_read(&r, "charliex");

    term.push_input(UP_ARROW);
    term.push_input("\n");

    assert_read(&r, "deltax");

    term.push_input(UP_ARROW);
    term.push_input(UP_ARROW);
    term.push_input("\n");

    assert_read(&r, "charlie");

    assert_lines(&term, &["$ charliex", "$ deltax", "$ charlie"]);
}

#[test]
fn test_kill() {
    let (term, r) = test("foo bar baz\x1b\x7f\x1b\x7f\n");

    assert_read(&r, "foo ");

    term.push_input("\x19\n");
    assert_read(&r, "bar baz");

    assert_lines(&term, &["$ foo", "$ bar baz"]);

    term.clear_all();
    term.push_input("alpha beta gamma\x1b\x7f");
    term.push_input(" \x7f"); // Make kill commands nonconsecutive
    term.push_input("\x1b\x7f\n");

    assert_read(&r, "alpha ");

    term.push_input("\x19\x19\x1by\n");

    assert_read(&r, "beta gamma");

    assert_lines(&term, &["$ alpha", "$ beta gamma"]);
}

#[test]
fn test_overwrite() {
    let (term, r) = test("foo");

    term.push_input(LEFT_ARROW);
    term.push_input(LEFT_ARROW);
    term.push_input(INSERT);
    term.push_input("xxx\n");

    assert_read(&r, "fxxx");
    assert_lines(&term, &["$ fxxx"]);
}

#[test]
fn test_transpose_chars() {
    let (term, r) = test("");

    term.push_input("abc\x14x\n");
    assert_read(&r, "acbx");

    term.push_input("abcde");
    term.push_input(HOME);
    term.push_input(RIGHT_ARROW);
    term.push_input("\x14\x14\n");
    assert_read(&r, "bcade");

    term.push_input("abcde");
    term.push_input("\x1b-3\x14x\n");
    assert_read(&r, "aexbcd");

    term.push_input("abcde");
    term.push_input(HOME);
    term.push_input(RIGHT_ARROW);
    term.push_input("\x1b3\x14x\n");
    assert_read(&r, "bcdaxe");

    assert_lines(&term, &["$ acbx", "$ bcade", "$ aexbcd", "$ bcdaxe"]);
}

#[test]
fn test_transpose_words() {
    let (term, r) = test("");

    term.resize(Size{lines: 7, columns: 40});

    term.push_input("a bb ccc");
    term.push_input("\x1btx\n");
    assert_read(&r, "a ccc bbx");

    term.push_input("a bb ccc");
    term.push_input("\x1b-\x1btx\n");
    assert_read(&r, "a cccx bb");

    term.push_input("a bb ccc");
    term.push_input(HOME);
    term.push_input(RIGHT_ARROW);
    term.push_input("\x1btx\n");
    assert_read(&r, "bb ax ccc");

    term.push_input("a bb ccc");
    term.push_input(HOME);
    term.push_input(RIGHT_ARROW);
    term.push_input("\x1bt\x1btx\n");
    assert_read(&r, "bb ccc ax");

    term.push_input("a bb ccc dddd eeeee");
    term.push_input(HOME);
    term.push_input(RIGHT_ARROW);
    term.push_input("\x1b3\x1btx\n");
    assert_read(&r, "bb ccc dddd ax eeeee");

    term.push_input("a bb ccc dddd eeeee");
    term.push_input("\x1b-3\x1btx\n");
    assert_read(&r, "a eeeeex bb ccc dddd");

    assert_lines(&term, &["$ a ccc bbx", "$ a cccx bb", "$ bb ax ccc",
        "$ bb ccc ax", "$ bb ccc dddd ax eeeee", "$ a eeeeex bb ccc dddd"]);
}

#[test]
fn test_search_history() {
    let (term, r) = test("");

    term.resize(Size{lines: 10, columns: 10});

    r.add_history("foo".into());

    term.push_input("\x12f\n");
    assert_read(&r, "foo");

    r.add_history("bar veryverylonginput".into());

    term.push_input("\x12b\n");
    assert_read(&r, "bar veryverylonginput");

    assert_lines(&term, &[
        "$ foo",
        "$ bar very",
        "verylongin",
        "put",
    ]);
}

#[test]
fn test_history_search() {
    let (term, r) = test("");

    term.resize(Size{lines: 10, columns: 10});

    r.bind_sequence("\x01", Command::HistorySearchBackward);
    r.bind_sequence("\x02", Command::HistorySearchForward);

    r.add_history("foo".into());
    r.add_history("fab".into());
    r.add_history("fun".into());

    term.push_input("f\x01\n");
    assert_read(&r, "fun");

    term.push_input("f\x01\x01\n");
    assert_read(&r, "fab");

    term.push_input("f\x01\x01\x01\n");
    assert_read(&r, "foo");

    term.push_input("f\x01\x01\x02\n");
    assert_read(&r, "fun");

    term.push_input("f\x01\x02\x02\n");
    assert_read(&r, "fun");

    assert_lines(&term, &[
        "$ fun",
        "$ fab",
        "$ foo",
        "$ fun",
        "$ fun",
    ]);
}
