//! Implements an in-memory `Terminal` interface
//!
//! The main purpose of the in-memory terminal is for internal testing

use std::cmp::min;
use std::iter::repeat;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::terminal::{
    CursorMode, RawRead, SignalSet, Size,
    Terminal, TerminalReader, TerminalWriter,
};

/// Default size of a `MemoryTerminal` buffer
pub const DEFAULT_SIZE: Size = Size{
    columns: 80,
    lines: 24,
};

/// Implements an in-memory `Terminal` interface
///
/// The contents of a `MemoryTerminal` are shared. That is, cloning
/// a `MemoryTerminal` value will share the contained terminal buffer.
#[derive(Clone, Debug)]
pub struct MemoryTerminal {
    write: Arc<Mutex<Writer>>,
    read: Arc<Mutex<Reader>>,
}

#[derive(Debug)]
struct Writer {
    memory: Vec<char>,
    col: usize,
    line: usize,
    cursor_mode: CursorMode,
    size: Size,
}

#[derive(Debug)]
struct Reader {
    input: Vec<u8>,
    resize: Option<Size>,
}

/// Holds the lock on read operations of a `MemoryTerminal`.
pub struct MemoryReadGuard<'a>(MutexGuard<'a, Reader>);

/// Holds the lock on write operations of a `MemoryTerminal`.
pub struct MemoryWriteGuard<'a>(MutexGuard<'a, Writer>);

impl MemoryTerminal {
    /// Returns a new `MemoryTerminal` with the default buffer size.
    pub fn new() -> MemoryTerminal {
        MemoryTerminal::default()
    }

    /// Returns a new `MemoryTerminal` with the given buffer size.
    ///
    /// # Panics
    ///
    /// If either of the `lines` or `columns` fields are `0`.
    pub fn with_size(size: Size) -> MemoryTerminal {
        MemoryTerminal{
            read: Arc::new(Mutex::new(Reader::new())),
            write: Arc::new(Mutex::new(Writer::new(size))),
        }
    }

    /// Clears the terminal buffer and places the cursor at `(0, 0)`.
    pub fn clear_all(&self) {
        self.lock_writer().clear_all();
    }

    /// Clears all characters beginning at the cursor and ending at buffer end.
    pub fn clear_to_end(&self) {
        self.lock_writer().clear_to_end();
    }

    /// Clears the input buffer.
    pub fn clear_input(&self) {
        self.lock_reader().clear_input();
    }

    /// Returns whether any input remains to be read.
    pub fn has_input(&self) -> bool {
        self.lock_reader().has_input()
    }

    /// Returns an iterator over lines in the buffer.
    ///
    /// # Notes
    ///
    /// The returned iterator immutably borrows the contents of the
    /// `MemoryTerminal`. Attempting to perform a mutating operation on the
    /// parent `MemoryTerminal` while the `Lines` iterator lives will cause
    /// a panic.
    pub fn lines(&self) -> Lines {
        Lines{
            writer: self.lock_writer(),
            line: 0,
        }
    }

    /// Moves the cursor up `n` cells.
    pub fn move_up(&self, n: usize) {
        self.lock_writer().move_up(n);
    }

    /// Moves the cursor down `n` cells.
    pub fn move_down(&self, n: usize) {
        self.lock_writer().move_down(n);
    }

    /// Moves the cursor left `n` cells.
    pub fn move_left(&self, n: usize) {
        self.lock_writer().move_left(n);
    }

    /// Moves the cursor right `n` cells.
    pub fn move_right(&self, n: usize) {
        self.lock_writer().move_right(n);
    }

    /// Moves the cursor to the first column of the current line.
    pub fn move_to_first_column(&self) {
        self.lock_writer().move_to_first_column()
    }

    /// Pushes a character sequence to the back of the input queue.
    pub fn push_input(&self, s: &str) {
        self.lock_reader().push_input(s.as_bytes());
    }

    /// Reads some input from the input buffer.
    pub fn read_input(&self, buf: &mut [u8]) -> usize {
        self.lock_reader().read_input(buf)
    }

    /// Changes the size of the terminal buffer.
    /// The buffer will be truncated or filled with spaces, as necessary.
    ///
    /// # Panics
    ///
    /// If either of the `lines` or `columns` fields are `0` or if the area
    /// exceeds `usize` maximum.
    pub fn resize(&self, new_size: Size) {
        self.lock_writer().resize(new_size);
        self.lock_reader().resize(new_size);
    }

    /// Moves the contents of the buffer up `n` lines.
    /// The first `n` lines of text will be erased.
    pub fn scroll_up(&self, n: usize) {
        self.lock_writer().scroll_up(n);
    }

    /// Returns the `(line, column)` position of the cursor.
    pub fn cursor(&self) -> (usize, usize) {
        let r = self.lock_writer();
        (r.line, r.col)
    }

    /// Sets the cursor mode.
    pub fn set_cursor_mode(&self, mode: CursorMode) {
        self.lock_writer().set_cursor_mode(mode);
    }

    /// Returns the cursor mode.
    pub fn cursor_mode(&self) -> CursorMode {
        self.lock_writer().cursor_mode()
    }

    /// Returns the size of the terminal buffer.
    pub fn size(&self) -> Size {
        self.lock_writer().size
    }

    /// Writes some text into the buffer.
    ///
    /// If the text extends beyond the length of the current line without a
    /// newline character (`'\n'`), the extraneous text will be dropped.
    pub fn write(&self, s: &str) {
        self.lock_writer().write(s);
    }

    fn lock_reader(&self) -> MutexGuard<Reader> {
        self.read.lock().unwrap()
    }

    fn lock_writer(&self) -> MutexGuard<Writer> {
        self.write.lock().unwrap()
    }
}

impl Default for MemoryTerminal {
    fn default() -> MemoryTerminal {
        MemoryTerminal::with_size(DEFAULT_SIZE)
    }
}

impl Reader {
    fn new() -> Reader {
        Reader{
            input: Vec::new(),
            resize: None,
        }
    }

    fn has_input(&mut self) -> bool {
        self.resize.is_some() || !self.input.is_empty()
    }

    fn clear_input(&mut self) {
        self.input.clear();
    }

    fn push_input(&mut self, bytes: &[u8]) {
        self.input.extend(bytes);
    }

    fn read_input(&mut self, buf: &mut [u8]) -> usize {
        let n = min(buf.len(), self.input.len());

        buf[..n].copy_from_slice(&self.input[..n]);
        let _ = self.input.drain(..n);
        n
    }

    fn resize(&mut self, size: Size) {
        self.resize = Some(size);
    }
}

impl Writer {
    fn new(size: Size) -> Writer {
        assert!(size.lines != 0 && size.columns != 0,
            "zero-area terminal buffer: {:?}", size);

        let n_chars = size.lines * size.columns;

        Writer{
            memory: vec![' '; n_chars],
            col: 0,
            line: 0,
            cursor_mode: CursorMode::Normal,
            size: size,
        }
    }

    fn clear_all(&mut self) {
        for ch in &mut self.memory {
            *ch = ' ';
        }
        self.col = 0;
        self.line = 0;
    }

    fn clear_to_end(&mut self) {
        let idx = self.index();

        for ch in &mut self.memory[idx..] {
            *ch = ' ';
        }
    }

    fn move_up(&mut self, n: usize) {
        self.line = self.line.saturating_sub(n);
    }

    fn move_down(&mut self, n: usize) {
        self.line = min(self.size.lines - 1, self.line + n);
    }

    fn move_left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
    }

    fn move_right(&mut self, n: usize) {
        self.col = min(self.size.columns - 1, self.col + n);
    }

    fn move_to_first_column(&mut self) {
        self.col = 0;
    }

    fn resize(&mut self, new_size: Size) {
        if self.size != new_size {
            let n_chars = new_size.lines.checked_mul(new_size.columns)
                .unwrap_or_else(|| panic!("terminal size too large: {:?}", new_size));

            assert!(n_chars != 0, "zero-area terminal buffer: {:?}", new_size);

            let mut new_buf = Vec::with_capacity(n_chars);

            let (n_copy, n_extra) = if new_size.columns > self.size.columns {
                (self.size.columns, new_size.columns - self.size.columns)
            } else {
                (new_size.columns, 0)
            };

            for line in self.memory.chunks(self.size.columns).take(new_size.lines) {
                new_buf.extend(&line[..n_copy]);
                new_buf.extend(repeat(' ').take(n_extra));
            }

            if new_size.lines > self.size.lines {
                let n_lines = new_size.lines - self.size.lines;
                new_buf.extend(repeat(' ').take(n_lines * new_size.columns));
            }

            debug_assert_eq!(new_buf.len(), n_chars);

            self.col = min(self.col, new_size.columns);
            self.line = min(self.line, new_size.lines);
            self.size = new_size;
            self.memory = new_buf;
        }
    }

    fn scroll_up(&mut self, n: usize) {
        let chars = min(self.memory.len(), self.size.columns * n);
        self.memory.drain(..chars);
        self.memory.extend(repeat(' ').take(chars));
        self.line = self.line.saturating_sub(n);
    }

    fn set_cursor_mode(&mut self, mode: CursorMode) {
        self.cursor_mode = mode;
    }

    fn cursor_mode(&self) -> CursorMode {
        self.cursor_mode
    }

    fn write(&mut self, s: &str) {
        for ch in s.chars() {
            if ch == '\n' {
                self.advance_line();
            } else if ch == '\r' {
                self.col = 0;
            } else {
                self.write_char(ch);
            }
        }
    }

    fn advance_line(&mut self) {
        self.line += 1;
        self.col = 0;
        if self.line == self.size.lines {
            self.scroll_up(1);
        }
    }

    fn write_char(&mut self, ch: char) {
        if self.col >= self.size.columns {
            self.advance_line();
        }

        let idx = self.index();
        self.memory[idx] = ch;
        self.col += 1;
    }

    fn index(&self) -> usize {
        self.line * self.size.columns + self.col
    }
}

/// Iterator over lines in a `MemoryTerminal` buffer.
///
/// Note that while this value behaves as an iterator, it cannot implement
/// the `Iterator` trait because its yielded values borrow `self`.
pub struct Lines<'a> {
    writer: MutexGuard<'a, Writer>,
    line: usize,
}

impl<'a> Lines<'a> {
    /// Returns the next line in the buffer.
    pub fn next(&mut self) -> Option<&[char]> {
        if self.line >= self.writer.size.lines {
            None
        } else {
            let start = self.writer.size.columns * self.line;
            self.line += 1;
            let end = self.writer.size.columns * self.line;

            Some(&self.writer.memory[start..end])
        }
    }

    /// Returns the number of lines remaining in the iterator.
    pub fn lines_remaining(&self) -> usize {
        self.writer.size.lines - self.line
    }
}

impl Terminal for MemoryTerminal {
    // No preparation needed for in-memory terminal
    type PrepareState = ();
    //type Reader = MemoryReadGuard;
    //type Writer = MemoryWriteGuard;

    fn name(&self) -> &str { "memory-terminal" }

    fn lock_read<'a>(&'a self) -> Box<dyn TerminalReader<Self> + 'a> {
        Box::new(MemoryReadGuard(self.lock_reader()))
    }

    fn lock_write<'a>(&'a self) -> Box<dyn TerminalWriter<Self> + 'a> {
        Box::new(MemoryWriteGuard(self.lock_writer()))
    }
}

impl<'a> TerminalReader<MemoryTerminal> for MemoryReadGuard<'a> {
    fn wait_for_input(&mut self, _timeout: Option<Duration>) -> io::Result<bool> {
        Ok(!self.0.input.is_empty())
    }

    fn prepare(&mut self, _block_signals: bool, _report_signals: SignalSet)
            -> io::Result<()> { Ok(()) }

    unsafe fn prepare_with_lock(&mut self,
            _lock: &mut dyn TerminalWriter<MemoryTerminal>,
            _block_signals: bool, _report_signals: SignalSet)
            -> io::Result<()> { Ok(()) }

    fn restore(&mut self, _state: ()) -> io::Result<()> { Ok(()) }

    unsafe fn restore_with_lock(&mut self,
            _lock: &mut dyn TerminalWriter<MemoryTerminal>, _state: ())
            -> io::Result<()> { Ok(()) }

    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<RawRead> {
        if let Some(size) = self.0.resize.take() {
            return Ok(RawRead::Resize(size));
        }

        buf.reserve(16);

        let cap = buf.capacity();
        let len = buf.len();
        let n;

        unsafe {
            buf.set_len(cap);
            n = self.0.read_input(&mut buf[len..]);
            buf.set_len(len + n);
        }

        Ok(RawRead::Bytes(n))
    }
}

impl<'a> TerminalWriter<MemoryTerminal> for MemoryWriteGuard<'a> {
    fn size(&self) -> io::Result<Size> {
        Ok(self.0.size)
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        self.0.clear_all();
        Ok(())
    }

    fn clear_to_screen_end(&mut self) -> io::Result<()> {
        self.0.clear_to_end();
        Ok(())
    }

    fn move_up(&mut self, n: usize) -> io::Result<()> {
        self.0.move_up(n);
        Ok(())
    }

    fn move_down(&mut self, n: usize) -> io::Result<()> {
        self.0.move_down(n);
        Ok(())
    }

    fn move_left(&mut self, n: usize) -> io::Result<()> {
        self.0.move_left(n);
        Ok(())
    }

    fn move_right(&mut self, n: usize) -> io::Result<()> {
        self.0.move_right(n);
        Ok(())
    }

    fn move_to_first_column(&mut self) -> io::Result<()> {
        self.0.move_to_first_column();
        Ok(())
    }

    fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        self.0.set_cursor_mode(mode);
        Ok(())
    }

    fn write(&mut self, s: &str) -> io::Result<()> {
        self.0.write(s);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[cfg(test)]
mod test {
    use super::MemoryTerminal;
    use crate::terminal::Size;

    fn assert_lines(mem: &MemoryTerminal, tests: &[&str]) {
        let mut lines = mem.lines();
        let mut test_iter = tests.iter();

        while let Some(line) = lines.next() {
            let test = test_iter.next().unwrap();
            assert!(line.iter().cloned().eq(test.chars()),
                "mem: {:?}; tests: {:?}", mem.lock_writer().memory, tests);
        }
    }

    #[test]
    fn test_memory_term() {
        let mem = MemoryTerminal::with_size(Size{lines: 3, columns: 4});

        assert_lines(&mem, &["    "; 3]);

        mem.write("ab");
        assert_lines(&mem, &["ab  ", "    ", "    "]);

        mem.write("c\nd");
        assert_lines(&mem, &["abc ", "d   ", "    "]);

        mem.write("efg\nhi");
        assert_lines(&mem, &["abc ", "defg", "hi  "]);

        mem.write("\njk\n");
        assert_lines(&mem, &["hi  ", "jk  ", "    "]);

        mem.write("\n\n\n\n\nlmno");
        assert_lines(&mem, &["    ", "    ", "lmno"]);

        mem.move_up(1);
        mem.move_left(3);
        mem.write("xx");
        assert_lines(&mem, &["    ", " xx ", "lmno"]);

        mem.clear_all();
        mem.write("xyz");
        assert_lines(&mem, &["xyz ", "    ", "    "]);

        mem.write("\nabcd");
        assert_lines(&mem, &["xyz ", "abcd", "    "]);

        mem.move_to_first_column();
        mem.write("ab");
        mem.clear_to_end();
        assert_lines(&mem, &["xyz ", "ab  ", "    "]);

        mem.move_to_first_column();
        mem.move_down(1);
        mem.write("c");
        mem.move_right(1);
        mem.write("d");
        assert_lines(&mem, &["xyz ", "ab  ", "c d "]);
    }

    #[test]
    fn test_resize() {
        let mem = MemoryTerminal::with_size(Size{lines: 3, columns: 4});

        assert_lines(&mem, &["    "; 3]);

        mem.write("xxxx\nxxxx\nxxxx");
        assert_lines(&mem, &["xxxx"; 3]);

        mem.resize(Size{lines: 4, columns: 3});
        assert_lines(&mem, &["xxx", "xxx", "xxx", "   "]);

        mem.clear_all();
        mem.write("yyy\nyyy\nyyy\nyyy");
        assert_lines(&mem, &["yyy"; 4]);

        mem.resize(Size{lines: 3, columns: 4});
        assert_lines(&mem, &["yyy ", "yyy ", "yyy "]);
    }
}
