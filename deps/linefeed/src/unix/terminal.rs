use std::io;
use std::time::Duration;

use mortal::{Event, TerminalReadGuard};
use mortal::unix::TerminalExt;

use crate::terminal::RawRead;

pub fn terminal_read(term: &mut TerminalReadGuard, buf: &mut Vec<u8>) -> io::Result<RawRead> {
    let mut buffer = [0; 1024];

    match term.read_raw(&mut buffer, Some(Duration::new(0, 0)))? {
        None => Ok(RawRead::Bytes(0)),
        Some(Event::Raw(n)) => {
            buf.extend(&buffer[..n]);
            Ok(RawRead::Bytes(n))
        }
        Some(Event::Resize(size)) => Ok(RawRead::Resize(size)),
        Some(Event::Signal(sig)) => Ok(RawRead::Signal(sig)),
        _ => unreachable!()
    }
}
