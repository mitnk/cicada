//! Provides a low-level terminal interface

use std::io;
use std::time::Duration;

use mortal::{self, PrepareConfig, PrepareState, TerminalReadGuard, TerminalWriteGuard};
use crate::sys;

pub use mortal::{CursorMode, Signal, SignalSet, Size};

/// Default `Terminal` interface
pub struct DefaultTerminal(mortal::Terminal);

/// Represents the result of a `Terminal` read operation
pub enum RawRead {
    /// `n` bytes were read from the device
    Bytes(usize),
    /// The terminal window was resized
    Resize(Size),
    /// A signal was received while waiting for input
    Signal(Signal),
}

/// Defines a low-level interface to the terminal
pub trait Terminal: Sized + Send + Sync {
    // TODO: When generic associated types are implemented (and stabilized),
    // boxed trait objects may be replaced by `Reader` and `Writer`.
    /// Returned by `prepare`; passed to `restore` to restore state.
    type PrepareState;
    /*
    /// Holds an exclusive read lock and provides read operations
    type Reader: TerminalReader;
    /// Holds an exclusive write lock and provides write operations
    type Writer: TerminalWriter;
    */

    /// Returns the name of the terminal.
    fn name(&self) -> &str;

    /// Acquires a lock on terminal read operations and returns a value holding
    /// that lock and granting access to such operations.
    ///
    /// The lock must not be released until the returned value is dropped.
    fn lock_read<'a>(&'a self) -> Box<dyn TerminalReader<Self> + 'a>;

    /// Acquires a lock on terminal write operations and returns a value holding
    /// that lock and granting access to such operations.
    ///
    /// The lock must not be released until the returned value is dropped.
    fn lock_write<'a>(&'a self) -> Box<dyn TerminalWriter<Self> + 'a>;
}

/// Holds a lock on `Terminal` read operations
pub trait TerminalReader<Term: Terminal> {
    /// Prepares the terminal for line reading and editing operations.
    ///
    /// If `block_signals` is `true`, the terminal will be configured to treat
    /// special characters that would otherwise be interpreted as signals as
    /// their literal value.
    ///
    /// If `block_signals` is `false`, a signal contained in the `report_signals`
    /// set may be returned.
    ///
    /// # Notes
    ///
    /// This method may be called more than once. However, if the state values
    /// are not restored in reverse order in which they were created,
    /// the state of the underlying terminal device becomes undefined.
    fn prepare(&mut self, block_signals: bool, report_signals: SignalSet)
        -> io::Result<Term::PrepareState>;

    /// Like `prepare`, but called when the write lock is already held.
    ///
    /// # Safety
    ///
    /// This method must be called with a `TerminalWriter` instance returned
    /// by the same `Terminal` instance to which this `TerminalReader` belongs.
    unsafe fn prepare_with_lock(&mut self, lock: &mut dyn TerminalWriter<Term>,
            block_signals: bool, report_signals: SignalSet)
            -> io::Result<Term::PrepareState>;

    /// Restores the terminal state using the given state data.
    fn restore(&mut self, state: Term::PrepareState) -> io::Result<()>;

    /// Like `restore`, but called when the write lock is already held.
    ///
    /// # Safety
    ///
    /// This method must be called with a `TerminalWriter` instance returned
    /// by the same `Terminal` instance to which this `TerminalReader` belongs.
    unsafe fn restore_with_lock(&mut self, lock: &mut dyn TerminalWriter<Term>,
            state: Term::PrepareState) -> io::Result<()>;

    /// Reads some input from the terminal and appends it to the given buffer.
    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<RawRead>;

    /// Waits `timeout` for user input. If `timeout` is `None`, waits indefinitely.
    ///
    /// Returns `Ok(true)` if input becomes available within the given timeout
    /// or if a signal is received.
    ///
    /// Returns `Ok(false)` if the timeout expires before input becomes available.
    fn wait_for_input(&mut self, timeout: Option<Duration>) -> io::Result<bool>;
}

/// Holds a lock on `Terminal` write operations
pub trait TerminalWriter<Term: Terminal> {
    /// Returns the size of the terminal window
    fn size(&self) -> io::Result<Size>;

    /// Presents a clear terminal screen, with cursor at first row, first column.
    ///
    /// If the terminal possesses a scrolling window over a buffer, this shall
    /// have the effect of moving the visible window down such that it shows
    /// an empty view of the buffer, preserving some or all of existing buffer
    /// contents, where possible.
    fn clear_screen(&mut self) -> io::Result<()>;

    /// Clears characters on the line occupied by the cursor, beginning with the
    /// cursor and ending at the end of the line. Also clears all characters on
    /// all lines after the cursor.
    fn clear_to_screen_end(&mut self) -> io::Result<()>;

    /// Moves the cursor up `n` cells; `n` may be zero.
    fn move_up(&mut self, n: usize) -> io::Result<()>;

    /// Moves the cursor down `n` cells; `n` may be zero.
    fn move_down(&mut self, n: usize) -> io::Result<()>;

    /// Moves the cursor left `n` cells; `n` may be zero.
    fn move_left(&mut self, n: usize) -> io::Result<()>;

    /// Moves the cursor right `n` cells; `n` may be zero.
    fn move_right(&mut self, n: usize) -> io::Result<()>;

    /// Moves the cursor to the first column of the current line
    fn move_to_first_column(&mut self) -> io::Result<()>;

    /// Set the current cursor mode
    fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()>;

    /// Writes output to the terminal.
    ///
    /// For each carriage return `'\r'` written to the terminal, the cursor
    /// should be moved to the first column of the current line.
    ///
    /// For each newline `'\n'` written to the terminal, the cursor should
    /// be moved to the first column of the following line.
    ///
    /// The terminal interface shall not automatically move the cursor to the next
    /// line when `write` causes a character to be written to the final column.
    fn write(&mut self, s: &str) -> io::Result<()>;

    /// Flushes any currently buffered output data.
    ///
    /// `TerminalWriter` instances may not buffer data on all systems.
    ///
    /// Data must be flushed when the `TerminalWriter` instance is dropped.
    fn flush(&mut self) -> io::Result<()>;
}

impl DefaultTerminal {
    /// Opens access to the terminal device associated with standard output.
    pub fn new() -> io::Result<DefaultTerminal> {
        mortal::Terminal::new().map(DefaultTerminal)
    }

    /// Opens access to the terminal device associated with standard error.
    pub fn stderr() -> io::Result<DefaultTerminal> {
        mortal::Terminal::stderr().map(DefaultTerminal)
    }

    unsafe fn cast_writer<'a>(writer: &'a mut dyn TerminalWriter<Self>)
            -> &'a mut TerminalWriteGuard<'a> {
        &mut *(writer as *mut _ as *mut TerminalWriteGuard)
    }
}

impl Terminal for DefaultTerminal {
    type PrepareState = PrepareState;

    fn name(&self) -> &str {
        self.0.name()
    }

    fn lock_read<'a>(&'a self) -> Box<dyn TerminalReader<Self> + 'a> {
        Box::new(self.0.lock_read().unwrap())
    }

    fn lock_write<'a>(&'a self) -> Box<dyn TerminalWriter<Self> + 'a> {
        Box::new(self.0.lock_write().unwrap())
    }
}

impl<'a> TerminalReader<DefaultTerminal> for TerminalReadGuard<'a> {
    fn prepare(&mut self, block_signals: bool, report_signals: SignalSet)
            -> io::Result<PrepareState> {
        self.prepare(PrepareConfig{
            block_signals,
            enable_control_flow: !block_signals,
            enable_keypad: false,
            report_signals,
            .. PrepareConfig::default()
        })
    }

    unsafe fn prepare_with_lock(&mut self,
            lock: &mut dyn TerminalWriter<DefaultTerminal>,
            block_signals: bool, report_signals: SignalSet)
            -> io::Result<PrepareState> {
        let lock = DefaultTerminal::cast_writer(lock);

        self.prepare_with_lock(lock, PrepareConfig{
            block_signals,
            enable_control_flow: !block_signals,
            enable_keypad: false,
            report_signals,
            .. PrepareConfig::default()
        })
    }

    fn restore(&mut self, state: PrepareState) -> io::Result<()> {
        self.restore(state)
    }

    unsafe fn restore_with_lock(&mut self,
            lock: &mut dyn TerminalWriter<DefaultTerminal>, state: PrepareState)
            -> io::Result<()> {
        let lock = DefaultTerminal::cast_writer(lock);
        self.restore_with_lock(lock, state)
    }

    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<RawRead> {
        sys::terminal_read(self, buf)
    }

    fn wait_for_input(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
        self.wait_event(timeout)
    }

}

impl<'a> TerminalWriter<DefaultTerminal> for TerminalWriteGuard<'a> {
    fn size(&self) -> io::Result<Size> {
        self.size()
    }

    fn clear_screen(&mut self) -> io::Result<()> {
        self.clear_screen()
    }

    fn clear_to_screen_end(&mut self) -> io::Result<()> {
        self.clear_to_screen_end()
    }

    fn move_up(&mut self, n: usize) -> io::Result<()> {
        self.move_up(n)
    }
    fn move_down(&mut self, n: usize) -> io::Result<()> {
        self.move_down(n)
    }
    fn move_left(&mut self, n: usize) -> io::Result<()> {
        self.move_left(n)
    }
    fn move_right(&mut self, n: usize) -> io::Result<()> {
        self.move_right(n)
    }

    fn move_to_first_column(&mut self) -> io::Result<()> {
        self.move_to_first_column()
    }

    fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        self.set_cursor_mode(mode)
    }

    fn write(&mut self, s: &str) -> io::Result<()> {
        self.write_str(s)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush()
    }
}
