use std::io;
use linefeed::{Function, Reader, Terminal};

pub struct UpKeyFunction;
pub struct DownKeyFunction;
pub const SEQ_UP_KEY: &'static str = "\x1b[A";
pub const SEQ_DOWN_KEY: &'static str = "\x1b[B";

impl<Term: Terminal> Function<Term> for UpKeyFunction {
    fn execute(&self, reader: &mut Reader<Term>, _count: i32, _ch: char) -> io::Result<()> {
        let len = reader.history_len();
        let history_index = if let Some(x) = reader.history_index() {
            x
        } else {
            len
        };
        let line = if history_index == len {
            reader.buffer().to_string()
        } else {
            reader.backup_buffer().to_string()
        };
        let mut n = len;
        for (i, x) in reader.history().rev().enumerate() {
            let s = x.to_string();
            if s.starts_with(line.as_str()) {
                n = len - i - 1;
                if history_index == len {
                    // first time typing <UP> key; return directly
                    break;
                } else if n < history_index {
                    // means it's not the first search
                    break;
                }
            }
        }
        if n < len {
            reader
                .select_history_entry(Some(n))
                .expect("select_history_entry error");
        }
        Ok(())
    }
}

impl<Term: Terminal> Function<Term> for DownKeyFunction {
    fn execute(&self, reader: &mut Reader<Term>, _count: i32, _ch: char) -> io::Result<()> {
        let len = reader.history_len();
        let history_index = if let Some(x) = reader.history_index() {
            x
        } else {
            len
        };
        if history_index == len {
            return Ok(());
        }
        let line = reader.backup_buffer().to_string();
        let mut n = len;
        for (i, x) in reader.history().enumerate() {
            let s = x.to_string();
            if s.starts_with(line.as_str()) {
                if i > history_index {
                    n = i;
                    break;
                }
            }
        }
        if n < len {
            reader
                .select_history_entry(Some(n))
                .expect("select_history_entry error");
        }
        Ok(())
    }
}
