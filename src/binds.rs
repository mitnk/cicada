use std::io;
use linefeed::{Function, Reader, Terminal};

pub struct UpKeyFunction;
pub const SEQ_UP_KEY: &'static str = "\x1b[A";

impl<Term: Terminal> Function<Term> for UpKeyFunction {
    fn execute(&self, reader: &mut Reader<Term>, _count: i32, _ch: char) -> io::Result<()> {
        assert_eq!(reader.sequence(), SEQ_UP_KEY);
        let line = reader.buffer().to_string();
        let mut record = String::new();
        for x in reader.history().rev() {
            let s = x.to_string();
            if s.starts_with(line.as_str()) {
                record = s.clone();
                break;
            }
        }
        let pos = reader.cursor();
        reader.delete_range(..pos).expect("delete_range error.");
        reader.insert_str(record.as_str())
    }
}
