//! Provides the main interface to interactive input reader

use std::borrow::Cow;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{
    self, BufRead, BufReader, BufWriter,
    Read as _, Seek, SeekFrom, Write as _,
};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::command::Command;
use crate::complete::Completer;
use crate::function::Function;
use crate::highlighting::Highlighter;
use crate::inputrc::Directive;
use crate::reader::{Read, Reader, ReadLock, ReadResult};
use crate::terminal::{DefaultTerminal, Signal, Terminal};
use crate::variables::Variable;
use crate::writer::{Write, Writer, WriteLock};

/// The main interface to input reading and other terminal operations
///
/// # Concurrency
///
/// Each `Interface` contains two internal locks to allow concurrent read and write
/// operations. The following types are used to hold a lock and to provide
/// access to a set of operations:
///
/// * [`Reader`] holds the read lock; it provides access to variables and bindings
///   and can initiate a [`read_line`] call. When `read_line` begins, the read
///   lock is acquired and it is held until the function returns. During the
///   `read_line` loop, the `Reader` waits for user input, reads input from the
///   terminal device, then acquires the write lock to process input and run
///   commands which may alter the prompt and the input buffer.
/// * [`Writer`] holds the write lock; it provides an interface to write
///   line-by-line output to the terminal device without interfering with the
///   prompt when a `read_line` may be in progress.
/// * [`Prompter`] holds both the read and write locks; it is created by the
///   `Reader` during the `read_line` loop and destroyed when the write lock is
///   released. It provides access to the current state of user input.
///
/// [`Reader`]: ../reader/struct.Reader.html
/// [`Writer`]: ../writer/struct.Writer.html
/// [`Prompter`]: ../prompter/struct.Prompter.html
/// [`read_line`]: #method.read_line
pub struct Interface<Term: Terminal> {
    term: Arc<Term>,
    read: Arc<Mutex<Read<Term>>>,
    write: Arc<Mutex<Write>>,
    highlighter: Option<Arc<dyn Highlighter + Send + Sync>>,
}

impl Interface<DefaultTerminal> {
    /// Creates a new `Interface` with the given application name.
    ///
    /// `application` is a string containing the name of the application.
    /// This can be used in user configurations to specify behavior for
    /// particular applications.
    ///
    /// The default terminal interface is used.
    pub fn new<T>(application: T) -> io::Result<Interface<DefaultTerminal>>
            where T: Into<Cow<'static, str>> {
        let term = DefaultTerminal::new()?;
        Interface::with_term(application, term)
    }
}

impl<Term: Terminal> Interface<Term> {
    /// Creates a new `Interface` instance with a particular terminal implementation.
    ///
    /// To use the default terminal interface, call `Interface::new` instead.
    pub fn with_term<T>(application: T, term: Term) -> io::Result<Interface<Term>>
            where T: Into<Cow<'static, str>> {
        let size = term.lock_write().size()?;
        let read = Read::new(&term, application.into());

        Ok(Interface{
            term: Arc::new(term),
            read: Arc::new(Mutex::new(read)),
            write: Arc::new(Mutex::new(Write::new(size))),
            highlighter: None,
        })
    }

    /// Acquires the read lock and returns a `Reader` instance.
    ///
    /// The `Reader` instance allows exclusive access to variables, bindings,
    /// and command implementations.
    pub fn lock_reader(&self) -> Reader<Term> {
        Reader::new(self, self.lock_read())
    }

    /// Acquires the write lock and returns a `Writer` instance.
    ///
    /// If a `read_line` call is in progress, this method will move the cursor
    /// to a new line after the prompt, allowing output to be written without
    /// corrupting the prompt text. The prompt will be redrawn when the `Writer`
    /// instance is dropped.
    ///
    /// To instead erase the prompt and write text, use [`lock_writer_erase`].
    ///
    /// [`lock_writer_erase`]: #method.lock_writer_erase
    pub fn lock_writer_append(&self) -> io::Result<Writer<Term>> {
        Writer::with_lock(self.lock_write()?, false)
    }

    /// Acquires the write lock and returns a `Writer` instance.
    ///
    /// If a `read_line` call is in progress, this method will erase the prompt,
    /// allowing output to be written without corrupting the prompt text.
    /// The prompt will be redrawn when the `Writer` instance is dropped.
    ///
    /// To instead write text after the prompt, use [`lock_writer_append`].
    ///
    /// [`lock_writer_append`]: #method.lock_writer_append
    pub fn lock_writer_erase(&self) -> io::Result<Writer<Term>> {
        Writer::with_lock(self.lock_write()?, true)
    }

    fn lock_read(&self) -> ReadLock<Term> {
        ReadLock::new(
            self.term.lock_read(),
            self.read.lock().expect("Interface::lock_read"))
    }

    pub(crate) fn lock_write(&self)
            -> io::Result<WriteLock<Term>> {
        let guard = self.write.lock().unwrap();
        let term_writer = self.term.lock_write();

        Ok(WriteLock::new(term_writer, guard, self.highlighter.clone()))
    }

    pub(crate) fn lock_write_data(&self) -> MutexGuard<Write> {
        self.write.lock().expect("Interface::lock_write_data")
    }
}

/// ## Locking
///
/// The following methods internally acquire the read lock.
///
/// The lock is released before the method returns.
///
/// If the read lock is already held, e.g. because a `read_line` call is in
/// progress, the method will block until the lock is released.
impl<Term: Terminal> Interface<Term> {
    /// Interactively reads a line from the terminal device.
    ///
    /// User input is collected until one of the following conditions is met:
    ///
    /// * If the user issues an end-of-file, `ReadResult::Eof` is returned.
    /// * When the user inputs a newline (`'\n'`), the resulting input
    ///   (not containing a trailing newline character) is returned as
    ///   `ReadResult::Input(_)`.
    /// * When a reported signal (see [`set_report_signal`]) is received,
    ///   it is returned as `ReadResult::Signal(_)`. The `read_line` operation may
    ///   then be either resumed with another call to `read_line` or ended by
    ///   calling [`cancel_read_line`].
    ///
    /// [`cancel_read_line`]: #method.cancel_read_line
    /// [`set_report_signal`]: #method.set_report_signal
    pub fn read_line(&self) -> io::Result<ReadResult> {
        self.lock_reader().read_line()
    }

    /// Performs one step of the interactive `read_line` loop.
    ///
    /// This method can be used to drive the `read_line` process asynchronously.
    /// It will wait for input only up to the specified duration, then process
    /// any available input from the terminal.
    ///
    /// If the user completes the input process, `Ok(Some(result))` is returned.
    /// Otherwise, `Ok(None)` is returned to indicate that the interactive loop
    /// may continue.
    ///
    /// The interactive prompt may be cancelled prematurely using the
    /// [`cancel_read_line`] method.
    ///
    /// See [`read_line`] for details on the return value.
    ///
    /// [`cancel_read_line`]: #method.cancel_read_line
    /// [`read_line`]: #method.read_line
    pub fn read_line_step(&self, timeout: Option<Duration>)
            -> io::Result<Option<ReadResult>> {
        self.lock_reader().read_line_step(timeout)
    }

    /// Cancels an in-progress `read_line` operation.
    ///
    /// This method will reset internal data structures to their original state
    /// and move the terminal cursor to a new, empty line.
    ///
    /// This method is called to prematurely end the interactive loop when
    /// using the [`read_line_step`] method.
    ///
    /// It is not necessary to call this method if using the [`read_line`] method.
    ///
    /// [`read_line`]: #method.read_line
    /// [`read_line_step`]: #method.read_line_step
    pub fn cancel_read_line(&self) -> io::Result<()> {
        self.lock_reader().cancel_read_line()
    }

    /// Returns a clone of the current completer instance.
    pub fn completer(&self) -> Arc<dyn Completer<Term>> {
        self.lock_reader().completer().clone()
    }

    /// Replaces the current completer, returning the previous instance.
    pub fn set_completer(&self, completer: Arc<dyn Completer<Term>>)
            -> Arc<dyn Completer<Term>> {
        self.lock_reader().set_completer(completer)
    }

    /// Returns the value of the named variable or `None`
    /// if no such variable exists.
    pub fn get_variable(&self, name: &str) -> Option<Variable> {
        self.lock_reader().get_variable(name)
    }

    /// Sets the value of the named variable and returns the previous
    /// value.
    ///
    /// If `name` does not refer to a variable or the `value` is not
    /// a valid value for the variable, `None` is returned.
    pub fn set_variable(&self, name: &str, value: &str) -> Option<Variable> {
        self.lock_reader().set_variable(name, value)
    }

    /// Returns whether the given `Signal` is ignored.
    pub fn ignore_signal(&self, signal: Signal) -> bool {
        self.lock_reader().ignore_signal(signal)
    }

    /// Sets whether the given `Signal` will be ignored.
    pub fn set_ignore_signal(&self, signal: Signal, set: bool) {
        self.lock_reader().set_ignore_signal(signal, set)
    }

    /// Returns whether the given `Signal` is reported.
    pub fn report_signal(&self, signal: Signal) -> bool {
        self.lock_reader().report_signal(signal)
    }

    /// Sets whether the given `Signal` is reported.
    pub fn set_report_signal(&self, signal: Signal, set: bool) {
        self.lock_reader().set_report_signal(signal, set)
    }

    /// Binds a sequence to a command.
    ///
    /// Returns the previously bound command.
    pub fn bind_sequence<T>(&self, seq: T, cmd: Command) -> Option<Command>
            where T: Into<Cow<'static, str>> {
        self.lock_reader().bind_sequence(seq, cmd)
    }

    /// Binds a sequence to a command, if and only if the given sequence
    /// is not already bound to a command.
    ///
    /// Returns `true` if a new binding was created.
    pub fn bind_sequence_if_unbound<T>(&self, seq: T, cmd: Command) -> bool
            where T: Into<Cow<'static, str>> {
        self.lock_reader().bind_sequence_if_unbound(seq, cmd)
    }

    /// Removes a binding for the given sequence.
    ///
    /// Returns the previously bound command.
    pub fn unbind_sequence(&self, seq: &str) -> Option<Command> {
        self.lock_reader().unbind_sequence(seq)
    }

    /// Defines a named function to which sequences may be bound.
    ///
    /// The name should consist of lowercase ASCII letters and numbers,
    /// containing no spaces, with words separated by hyphens. However,
    /// this is not a requirement.
    ///
    /// Returns the function previously defined with the same name.
    pub fn define_function<T>(&self, name: T, cmd: Arc<dyn Function<Term>>)
            -> Option<Arc<dyn Function<Term>>> where T: Into<Cow<'static, str>> {
        self.lock_reader().define_function(name, cmd)
    }

    /// Removes a function defined with the given name.
    ///
    /// Returns the defined function.
    pub fn remove_function(&self, name: &str) -> Option<Arc<dyn Function<Term>>> {
        self.lock_reader().remove_function(name)
    }

    /// Evaluates a series of configuration directives.
    pub fn evaluate_directives(&self, dirs: Vec<Directive>) {
        self.lock_reader().evaluate_directives(&self.term, dirs)
    }

    /// Evaluates a single configuration directive.
    pub fn evaluate_directive(&self, dir: Directive) {
        self.lock_reader().evaluate_directive(&self.term, dir)
    }
}

/// ## Locking
///
/// The following methods internally acquire the write lock.
///
/// The lock is released before the method returns.
///
/// If the write lock is already held, the method will block until it is released.
impl<Term: Terminal> Interface<Term> {
    /// Returns the current input buffer.
    pub fn buffer(&self) -> String {
        self.lock_write().map(|lock| lock.buffer.to_owned()).unwrap_or_default()
    }

    /// Returns the current number of history entries.
    pub fn history_len(&self) -> usize {
        self.lock_write().map(|lock| lock.history_len()).unwrap_or(0)
    }

    /// Returns the maximum number of history entries.
    ///
    /// Not to be confused with [`history_len`], which returns the *current*
    /// number of history entries.
    ///
    /// [`history_len`]: #method.history_len
    pub fn history_size(&self) -> usize {
        self.lock_write().map(|lock| lock.history_size()).unwrap_or(0)
    }

    /// Save history entries to the specified file.
    ///
    /// If the file does not exist, it is created and all history entries are
    /// written to the new file.
    ///
    /// If the file does exist, entries added since the last call to `save_history`
    /// (or since the start of the application) are appended to the named file.
    ///
    /// If the file would contain more than `self.history_size()` entries,
    /// it is first truncated, discarding the oldest entries.
    pub fn save_history<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let mut w = self.lock_write()?;

        if !path.exists() || w.history_size() == !0 {
            self.append_history(path, &w)?;
        } else {
            self.rewrite_history(path, &w)?;
        }

        w.reset_new_history();
        Ok(())
    }

    fn append_history<P: AsRef<Path>>(&self, path: P, w: &WriteLock<Term>)
            -> io::Result<()> {
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path.as_ref())?;

        self.append_history_to(&file, w)
    }

    fn append_history_to(&self, file: &File, w: &WriteLock<Term>) -> io::Result<()> {
        let mut wtr = BufWriter::new(file);

        for entry in w.new_history() {
            wtr.write_all(entry.as_bytes())?;
            wtr.write_all(b"\n")?;
        }

        wtr.flush()
    }

    fn rewrite_history<P: AsRef<Path>>(&self, path: P, w: &WriteLock<Term>)
            -> io::Result<()> {
        fn nth_line(s: &str, n: usize) -> Option<usize> {
            let start = s.as_ptr() as usize;

            s.lines().nth(n)
                .map(|s| s.as_ptr() as usize - start)
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path.as_ref())?;

        let mut hist = String::new();

        file.read_to_string(&mut hist)?;

        let n_lines = hist.lines().count();
        let n = n_lines.saturating_sub(
            w.history_size() - w.new_history_entries());

        if n != 0 {
            if let Some(pos) = nth_line(&hist, n) {
                file.seek(SeekFrom::Start(0))?;
                file.write_all(hist[pos..].as_bytes())?;

                let n = file.seek(SeekFrom::Current(0))?;
                file.set_len(n)?;
            }
        }

        self.append_history_to(&file, w)
    }

    /// Load history entries from the specified file.
    pub fn load_history<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut writer = self.lock_write()?;

        let file = File::open(&path)?;
        let rdr = BufReader::new(file);

        for line in rdr.lines() {
            writer.add_history(line?);
        }

        writer.reset_new_history();

        Ok(())
    }

    /// Writes formatted text to the terminal display.
    ///
    /// This method enables `Interface` to be used as the receiver to
    /// the [`writeln!`] macro.
    ///
    /// If the text contains any unprintable characters (e.g. escape sequences),
    /// those characters will be escaped before printing.
    ///
    /// # Notes
    ///
    /// If this method is called during a [`read_line`] call, the prompt will
    /// first be erased, then restored after the given string is printed.
    /// Therefore, the written text should end with a newline. If the `writeln!`
    /// macro is used, a newline is automatically added to the end of the text.
    ///
    /// To instead write text after the prompt, use [`lock_writer_append`].
    ///
    /// [`read_line`]: #method.read_line
    /// [`writeln!`]: https://doc.rust-lang.org/std/macro.writeln.html
    /// [`lock_writer_append`]: #method.lock_writer_append
    pub fn write_fmt(&self, args: fmt::Arguments) -> io::Result<()> {
        let s = args.to_string();
        self.write_str(&s)
    }

    fn write_str(&self, line: &str) -> io::Result<()> {
        self.lock_writer_erase()?.write_str(line)
    }
}

/// ## Locking
///
/// The following methods internally acquire both the read and write locks.
///
/// The locks are released before the method returns.
///
/// If either lock is already held, the method will block until it is released.
impl<Term: Terminal> Interface<Term> {
    /// Sets the prompt that will be displayed when `read_line` is called.
    ///
    /// # Notes
    ///
    /// If `prompt` contains any terminal escape sequences (e.g. color codes),
    /// such escape sequences should be immediately preceded by the character
    /// `'\x01'` and immediately followed by the character `'\x02'`.
    pub fn set_prompt(&self, prompt: &str) -> io::Result<()> {
        self.lock_reader().set_prompt(prompt)
    }

    /// Sets the input buffer to the given string.
    ///
    /// # Notes
    ///
    /// To prevent invalidating the cursor, this method sets the cursor
    /// position to the end of the new buffer.
    pub fn set_buffer(&self, buf: &str) -> io::Result<()> {
        self.lock_reader().set_buffer(buf)
    }

    /// Sets the cursor position in the input buffer.
    ///
    /// # Panics
    ///
    /// If the given position is out of bounds or not on a `char` boundary.
    pub fn set_cursor(&self, pos: usize) -> io::Result<()> {
        self.lock_reader().set_cursor(pos)
    }

    // History methods don't appear to require a read lock, but do acquire
    // it nonetheless because any operation that truncates history may interefere
    // with an ongoing `read_line` call. Therefore, the read lock is acquired
    // to check whether a `read_line` call is in progress.

    /// Adds a line to history.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn add_history(&self, line: String) {
        self.lock_reader().add_history(line);
    }

    /// Adds a line to history, unless it is identical to the most recent entry.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn add_history_unique(&self, line: String) {
        self.lock_reader().add_history_unique(line);
    }

    /// Removes all history entries.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn clear_history(&self) {
        self.lock_reader().clear_history();
    }

    /// Removes the history entry at the given index.
    ///
    /// If the index is out of bounds, this method has no effect.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn remove_history(&self, idx: usize) {
        self.lock_reader().remove_history(idx);
    }

    /// Sets the maximum number of history entries.
    ///
    /// If `n` is less than the current number of history entries,
    /// the oldest entries are truncated to meet the given requirement.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn set_history_size(&self, n: usize) {
        self.lock_reader().set_history_size(n);
    }

    /// Truncates history to the only the most recent `n` entries.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn truncate_history(&self, n: usize) {
        self.lock_reader().truncate_history(n);
    }

    /// Sets the syntax highlighter for the input line.
    pub fn set_highlighter(&mut self, highlighter: Arc<dyn Highlighter + Send + Sync>) {
        self.highlighter = Some(highlighter);
    }
}
