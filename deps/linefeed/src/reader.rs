//! Provides access to terminal read operations

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::mem::replace;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::slice;
use std::sync::{Arc, MutexGuard};
use std::time::{Duration, Instant};

use mortal::SequenceMap;

use crate::command::{Category, Command};
use crate::complete::{Completer, Completion, DummyCompleter};
use crate::function::Function;
use crate::inputrc::{parse_file, Directive};
use crate::interface::Interface;
use crate::prompter::Prompter;
use crate::sys::path::{env_init_file, system_init_file, user_init_file};
use crate::terminal::{
    RawRead, Signal, SignalSet, Size,
    Terminal, TerminalReader,
};
use crate::util::{first_char, match_name};
use crate::variables::{Variable, Variables, VariableIter};

/// Default set of string characters
pub const STRING_CHARS: &str = "\"'";

/// Default set of word break characters
pub const WORD_BREAK_CHARS: &str = " \t\n\"\\'`@$><=;|&{(";

/// Indicates the start of a series of invisible characters in the prompt
pub const START_INVISIBLE: char = '\x01';

/// Indicates the end of a series of invisible characters in the prompt
pub const END_INVISIBLE: char = '\x02';

/// Maximum size of kill ring
const MAX_KILLS: usize = 10;

/// Provides access to data related to reading and processing user input.
///
/// Holds a lock on terminal read operations.
/// See [`Interface`] for more information about concurrent operations.
///
/// An instance of this type can be constructed using the
/// [`Interface::lock_reader`] method.
///
/// [`Interface`]: ../interface/struct.Interface.html
/// [`Interface::lock_reader`]: ../interface/struct.Interface.html#method.lock_reader
pub struct Reader<'a, Term: 'a + Terminal> {
    iface: &'a Interface<Term>,
    lock: ReadLock<'a, Term>,
}

pub(crate) struct Read<Term: Terminal> {
    /// Application name
    pub application: Cow<'static, str>,

    /// Pending input
    pub input_buffer: Vec<u8>,
    /// Pending macro sequence
    pub macro_buffer: String,

    pub bindings: SequenceMap<Cow<'static, str>, Command>,
    pub functions: HashMap<Cow<'static, str>, Arc<dyn Function<Term>>>,

    /// Current input sequence
    pub sequence: String,
    /// Whether newline has been received
    pub input_accepted: bool,

    /// Whether overwrite mode is currently active
    pub overwrite_mode: bool,
    /// Characters appended while in overwrite mode
    pub overwritten_append: usize,
    /// Characters overwritten in overwrite mode
    pub overwritten_chars: String,

    /// Configured completer
    pub completer: Arc<dyn Completer<Term>>,
    /// Character appended to completions
    pub completion_append_character: Option<char>,
    /// Current set of possible completions
    pub completions: Option<Vec<Completion>>,
    /// Current "menu-complete" entry being viewed:
    pub completion_index: usize,
    /// Start of the completed word
    pub completion_start: usize,
    /// Start of the inserted prefix of a completed word
    pub completion_prefix: usize,

    pub string_chars: Cow<'static, str>,
    pub word_break: Cow<'static, str>,

    pub last_cmd: Category,
    pub last_yank: Option<(usize, usize)>,
    pub kill_ring: VecDeque<String>,

    pub catch_signals: bool,
    pub ignore_signals: SignalSet,
    pub report_signals: SignalSet,
    pub last_resize: Option<Size>,
    pub last_signal: Option<Signal>,

    variables: Variables,

    pub state: InputState,
    pub max_wait_duration: Option<Duration>,
}

pub(crate) struct ReadLock<'a, Term: 'a + Terminal> {
    term: Box<dyn TerminalReader<Term> + 'a>,
    data: MutexGuard<'a, Read<Term>>,
}

/// Returned from [`read_line`] to indicate user input
///
/// [`read_line`]: ../interface/struct.Interface.html#method.read_line
#[derive(Debug)]
pub enum ReadResult {
    /// User issued end-of-file
    Eof,
    /// User input received
    Input(String),
    /// Reported signal was received
    Signal(Signal),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum InputState {
    Inactive,
    NewSequence,
    ContinueSequence{
        expiry: Option<Instant>,
    },
    Number,
    CharSearch{
        n: usize,
        backward: bool,
    },
    TextSearch,
    CompleteIntro,
    CompleteMore(usize),
    QuotedInsert(usize),
}

impl<'a, Term: 'a + Terminal> Reader<'a, Term> {
    pub(crate) fn new(iface: &'a Interface<Term>, lock: ReadLock<'a, Term>)
            -> Reader<'a, Term> {
        Reader{iface, lock}
    }

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
    pub fn read_line(&mut self) -> io::Result<ReadResult> {
        loop {
            if let Some(res) = self.read_line_step(None)? {
                return Ok(res);
            }
        }
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
    pub fn read_line_step(&mut self, timeout: Option<Duration>)
            -> io::Result<Option<ReadResult>> {
        self.initialize_read_line()?;

        let state = self.prepare_term()?;
        let res = self.read_line_step_impl(timeout);
        self.lock.term.restore(state)?;

        res
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
    pub fn cancel_read_line(&mut self) -> io::Result<()> {
        self.end_read_line()
    }

    fn initialize_read_line(&mut self) -> io::Result<()> {
        if !self.lock.is_active() {
            self.prompter().start_read_line()?;
        }
        Ok(())
    }

    fn read_line_step_impl(&mut self, timeout: Option<Duration>)
            -> io::Result<Option<ReadResult>> {
        let do_read = if self.lock.is_input_available() {
            // This branch will be taken only if a macro has buffered some input.
            // We check for input with a zero duration to see if the user has
            // entered Ctrl-C, e.g. to interrupt an infinitely recursive macro.
            self.lock.term.wait_for_input(Some(Duration::from_secs(0)))?
        } else {
            let timeout = limit_duration(timeout, self.lock.max_wait_duration);
            self.lock.term.wait_for_input(timeout)?
        };

        if do_read {
            self.lock.read_input()?;
        }

        if let Some(size) = self.lock.take_resize() {
            self.handle_resize(size)?;
        }

        if let Some(sig) = self.lock.take_signal() {
            if self.lock.report_signals.contains(sig) {
                return Ok(Some(ReadResult::Signal(sig)));
            }
            if !self.lock.ignore_signals.contains(sig) {
                self.handle_signal(sig)?;
            }
        }

        // Acquire the write lock and process all available input
        {
            let mut prompter = self.prompter();

            prompter.check_expire_timeout()?;

            // If the macro buffer grows in size while input is being processed,
            // we end this step and let the caller try again. This is to allow
            // reading Ctrl-C to interrupt (perhaps infinite) macro execution.
            let mut macro_len = prompter.read.data.macro_buffer.len();

            while prompter.read.is_input_available() {
                if let Some(ch) = prompter.read.read_char()? {
                    if let Some(r) = prompter.handle_input(ch)? {
                        prompter.end_read_line()?;
                        return Ok(Some(r));
                    }
                }

                let new_macro_len = prompter.read.data.macro_buffer.len();

                if new_macro_len != 0 && new_macro_len >= macro_len {
                    break;
                }

                macro_len = new_macro_len;
            }
        }

        Ok(None)
    }

    fn end_read_line(&mut self) -> io::Result<()> {
        if self.lock.is_active() {
            self.prompter().end_read_line()?;
        }
        Ok(())
    }

    fn prepare_term(&mut self) -> io::Result<Term::PrepareState> {
        if self.read_next_raw() {
            self.lock.term.prepare(true, SignalSet::new())
        } else {
            let mut signals = self.lock.report_signals.union(self.lock.ignore_signals);

            if self.lock.catch_signals {
                // Ctrl-C is always intercepted (unless we're catching no signals).
                // By default, linefeed handles it by clearing the current input state.
                signals.insert(Signal::Interrupt);
            }

            let block_signals = !self.lock.catch_signals;

            self.lock.term.prepare(block_signals, signals)
        }
    }

    fn read_next_raw(&self) -> bool {
        match self.lock.state {
            InputState::QuotedInsert(_) => true,
            _ => false
        }
    }

    /// Sets the input buffer to the given string.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// # Notes
    ///
    /// To prevent invalidating the cursor, this method sets the cursor
    /// position to the end of the new buffer.
    pub fn set_buffer(&mut self, buf: &str) -> io::Result<()> {
        if self.lock.is_active() {
            self.prompter().set_buffer(buf)
        } else {
            self.iface.lock_write_data().set_buffer(buf);
            Ok(())
        }
    }

    /// Sets the cursor position in the input buffer.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// # Panics
    ///
    /// If the given position is out of bounds or not on a `char` boundary.
    pub fn set_cursor(&mut self, pos: usize) -> io::Result<()> {
        if self.lock.is_active() {
            self.prompter().set_cursor(pos)
        } else {
            self.iface.lock_write_data().set_cursor(pos);
            Ok(())
        }
    }

    /// Sets the prompt that will be displayed when `read_line` is called.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// # Notes
    ///
    /// If `prompt` contains any terminal escape sequences (e.g. color codes),
    /// such escape sequences should be immediately preceded by the character
    /// `'\x01'` and immediately followed by the character `'\x02'`.
    pub fn set_prompt(&mut self, prompt: &str) -> io::Result<()> {
        self.prompter().set_prompt(prompt)
    }

    /// Adds a line to history.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn add_history(&self, line: String) {
        if !self.lock.is_active() {
            if let Ok(mut lock) = self.iface.lock_write() {
                lock.add_history(line);
            }
        }
    }

    /// Adds a line to history, unless it is identical to the most recent entry.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn add_history_unique(&self, line: String) {
        if !self.lock.is_active() {
            if let Ok(mut lock) = self.iface.lock_write() {
                lock.add_history_unique(line);
            }
        }
    }

    /// Removes all history entries.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn clear_history(&self) {
        if !self.lock.is_active() {
            if let Ok(mut lock) = self.iface.lock_write() {
                lock.clear_history();
            }
        }
    }

    /// Removes the history entry at the given index.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// If the index is out of bounds, this method has no effect.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn remove_history(&self, idx: usize) {
        if !self.lock.is_active() {
            if let Ok(mut lock) = self.iface.lock_write() {
                lock.remove_history(idx);
            }
        }
    }

    /// Sets the maximum number of history entries.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// If `n` is less than the current number of history entries,
    /// the oldest entries are truncated to meet the given requirement.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn set_history_size(&self, n: usize) {
        if !self.lock.is_active() {
            if let Ok(mut lock) = self.iface.lock_write() {
                lock.set_history_size(n);
            }
        }
    }

    /// Truncates history to the only the most recent `n` entries.
    ///
    /// This method internally acquires the `Interface` write lock.
    ///
    /// If a `read_line` call is in progress, this method has no effect.
    pub fn truncate_history(&self, n: usize) {
        if !self.lock.is_active() {
            if let Ok(mut lock) = self.iface.lock_write() {
                lock.truncate_history(n);
            }
        }
    }

    /// Returns the application name
    pub fn application(&self) -> &str {
        &self.lock.application
    }

    /// Sets the application name
    pub fn set_application<T>(&mut self, application: T)
            where T: Into<Cow<'static, str>> {
        self.lock.application = application.into();
    }

    /// Returns a reference to the current completer instance.
    pub fn completer(&self) -> &Arc<dyn Completer<Term>> {
        &self.lock.completer
    }

    /// Replaces the current completer, returning the previous instance.
    pub fn set_completer(&mut self, completer: Arc<dyn Completer<Term>>)
            -> Arc<dyn Completer<Term>> {
        replace(&mut self.lock.completer, completer)
    }

    /// Returns the value of the named variable or `None`
    /// if no such variable exists.
    pub fn get_variable(&self, name: &str) -> Option<Variable> {
        self.lock.get_variable(name)
    }

    /// Sets the value of the named variable and returns the previous
    /// value.
    ///
    /// If `name` does not refer to a variable or the `value` is not
    /// a valid value for the variable, `None` is returned.
    pub fn set_variable(&mut self, name: &str, value: &str) -> Option<Variable> {
        self.lock.set_variable(name, value)
    }

    /// Returns an iterator over stored variables.
    pub fn variables(&self) -> VariableIter {
        self.lock.variables.iter()
    }

    /// Returns whether to "blink" matching opening parenthesis character
    /// when a closing parenthesis character is entered.
    ///
    /// The default value is `false`.
    pub fn blink_matching_paren(&self) -> bool {
        self.lock.blink_matching_paren
    }

    /// Sets the `blink-matching-paren` variable.
    pub fn set_blink_matching_paren(&mut self, set: bool) {
        self.lock.blink_matching_paren = set;
    }

    /// Returns whether `linefeed` will catch certain signals.
    pub fn catch_signals(&self) -> bool {
        self.lock.catch_signals
    }

    /// Sets whether `linefeed` will catch certain signals.
    ///
    /// This setting is `true` by default. It can be disabled to allow the
    /// host program to handle signals itself.
    pub fn set_catch_signals(&mut self, enabled: bool) {
        self.lock.catch_signals = enabled;
    }

    /// Returns whether the given `Signal` is ignored.
    pub fn ignore_signal(&self, signal: Signal) -> bool {
        self.lock.ignore_signals.contains(signal)
    }

    /// Sets whether the given `Signal` will be ignored.
    pub fn set_ignore_signal(&mut self, signal: Signal, set: bool) {
        if set {
            self.lock.ignore_signals.insert(signal);
            self.lock.report_signals.remove(signal);
        } else {
            self.lock.ignore_signals.remove(signal);
        }
    }

    /// Returns whether the given `Signal` is to be reported.
    pub fn report_signal(&self, signal: Signal) -> bool {
        self.lock.report_signals.contains(signal)
    }

    /// Sets whether to report the given `Signal`.
    ///
    /// When a reported signal is received via the terminal, it will be returned
    /// from `Interface::read_line` as `Ok(Signal(signal))`.
    pub fn set_report_signal(&mut self, signal: Signal, set: bool) {
        if set {
            self.lock.report_signals.insert(signal);
            self.lock.ignore_signals.remove(signal);
        } else {
            self.lock.report_signals.remove(signal);
        }
    }

    /// Returns whether Tab completion is disabled.
    ///
    /// The default value is `false`.
    pub fn disable_completion(&self) -> bool {
        self.lock.disable_completion
    }

    /// Sets the `disable-completion` variable.
    pub fn set_disable_completion(&mut self, disable: bool) {
        self.lock.disable_completion = disable;
    }

    /// When certain control characters are pressed, a character sequence
    /// equivalent to this character will be echoed.
    ///
    /// The default value is `true`.
    pub fn echo_control_characters(&self) -> bool {
        self.lock.echo_control_characters
    }

    /// Sets the `echo-control-characters` variable.
    pub fn set_echo_control_characters(&mut self, echo: bool) {
        self.lock.echo_control_characters = echo;
    }

    /// Returns the character, if any, that is appended to a successful completion.
    pub fn completion_append_character(&self) -> Option<char> {
        self.lock.completion_append_character
    }

    /// Sets the character, if any, that is appended to a successful completion.
    pub fn set_completion_append_character(&mut self, ch: Option<char>) {
        self.lock.completion_append_character = ch;
    }

    /// Returns the width of completion listing display.
    ///
    /// If this value is greater than the terminal width, terminal width is used
    /// instead.
    ///
    /// The default value is equal to `usize::max_value()`.
    pub fn completion_display_width(&self) -> usize {
        self.lock.completion_display_width
    }

    /// Sets the `completion-display-width` variable.
    pub fn set_completion_display_width(&mut self, n: usize) {
        self.lock.completion_display_width = n;
    }

    /// Returns the minimum number of completion items that require user
    /// confirmation before listing.
    ///
    /// The default value is `100`.
    pub fn completion_query_items(&self) -> usize {
        self.lock.completion_query_items
    }

    /// Sets the `completion-query-items` variable.
    pub fn set_completion_query_items(&mut self, n: usize) {
        self.lock.completion_query_items = n;
    }

    /// Returns the timeout to wait for further user input when an ambiguous
    /// sequence has been entered. If the value is `None`, wait is indefinite.
    ///
    /// The default value 500 milliseconds.
    pub fn keyseq_timeout(&self) -> Option<Duration> {
        self.lock.keyseq_timeout
    }

    /// Sets the `keyseq-timeout` variable.
    pub fn set_keyseq_timeout(&mut self, timeout: Option<Duration>) {
        self.lock.keyseq_timeout = timeout;
    }

    /// Returns whether to list possible completions one page at a time.
    ///
    /// The default value is `true`.
    pub fn page_completions(&self) -> bool {
        self.lock.page_completions
    }

    /// Sets the `page-completions` variable.
    pub fn set_page_completions(&mut self, set: bool) {
        self.lock.page_completions = set;
    }

    /// Returns whether to list completions horizontally, rather than down
    /// the screen.
    ///
    /// The default value is `false`.
    pub fn print_completions_horizontally(&self) -> bool {
        self.lock.print_completions_horizontally
    }

    /// Sets the `print-completions-horizontally` variable.
    pub fn set_print_completions_horizontally(&mut self, set: bool) {
        self.lock.print_completions_horizontally = set;
    }

    /// Returns the set of characters that delimit strings.
    pub fn string_chars(&self) -> &str {
        &self.lock.string_chars
    }

    /// Sets the set of characters that delimit strings.
    pub fn set_string_chars<T>(&mut self, chars: T)
            where T: Into<Cow<'static, str>> {
        self.lock.string_chars = chars.into();
    }

    /// Returns the set of characters that indicate a word break.
    pub fn word_break_chars(&self) -> &str {
        &self.lock.word_break
    }

    /// Sets the set of characters that indicate a word break.
    pub fn set_word_break_chars<T>(&mut self, chars: T)
            where T: Into<Cow<'static, str>> {
        self.lock.word_break = chars.into();
    }

    /// Returns an iterator over bound sequences
    pub fn bindings(&self) -> BindingIter {
        self.lock.bindings()
    }

    /// Binds a sequence to a command.
    ///
    /// Returns the previously bound command.
    pub fn bind_sequence<T>(&mut self, seq: T, cmd: Command) -> Option<Command>
            where T: Into<Cow<'static, str>> {
        self.lock.bind_sequence(seq, cmd)
    }

    /// Binds a sequence to a command, if and only if the given sequence
    /// is not already bound to a command.
    ///
    /// Returns `true` if a new binding was created.
    pub fn bind_sequence_if_unbound<T>(&mut self, seq: T, cmd: Command) -> bool
            where T: Into<Cow<'static, str>> {
        self.lock.bind_sequence_if_unbound(seq, cmd)
    }

    /// Removes a binding for the given sequence.
    ///
    /// Returns the previously bound command.
    pub fn unbind_sequence(&mut self, seq: &str) -> Option<Command> {
        self.lock.unbind_sequence(seq)
    }

    /// Defines a named function to which sequences may be bound.
    ///
    /// The name should consist of lowercase ASCII letters and numbers,
    /// containing no spaces, with words separated by hyphens. However,
    /// this is not a requirement.
    ///
    /// Returns the function previously defined with the same name.
    pub fn define_function<T>(&mut self, name: T, cmd: Arc<dyn Function<Term>>)
            -> Option<Arc<dyn Function<Term>>> where T: Into<Cow<'static, str>> {
        self.lock.define_function(name, cmd)
    }

    /// Removes a function defined with the given name.
    ///
    /// Returns the defined function.
    pub fn remove_function(&mut self, name: &str) -> Option<Arc<dyn Function<Term>>> {
        self.lock.remove_function(name)
    }

    pub(crate) fn evaluate_directives(&mut self, term: &Term, dirs: Vec<Directive>) {
        self.lock.data.evaluate_directives(term, dirs)
    }

    pub(crate) fn evaluate_directive(&mut self, term: &Term, dir: Directive) {
        self.lock.data.evaluate_directive(term, dir)
    }

    fn prompter<'b>(&'b mut self) -> Prompter<'b, 'a, Term> {
        Prompter::new(
            &mut self.lock,
            self.iface.lock_write().expect("Failed to acquire write lock"))
    }

    fn handle_resize(&mut self, size: Size) -> io::Result<()> {
        self.prompter().handle_resize(size)
    }

    fn handle_signal(&mut self, sig: Signal) -> io::Result<()> {
        self.prompter().handle_signal(sig)
    }
}

impl<'a, Term: 'a + Terminal> ReadLock<'a, Term> {
    pub fn new(term: Box<dyn TerminalReader<Term> + 'a>, data: MutexGuard<'a, Read<Term>>)
            -> ReadLock<'a, Term> {
        ReadLock{term, data}
    }

    /// Reads the next character of input.
    ///
    /// Performs a non-blocking read from the terminal, if necessary.
    ///
    /// If non-input data was received (e.g. a signal) or insufficient input
    /// is available, `Ok(None)` is returned.
    pub fn read_char(&mut self) -> io::Result<Option<char>> {
        if let Some(ch) = self.macro_pop() {
            Ok(Some(ch))
        } else if let Some(ch) = self.decode_input()? {
            Ok(Some(ch))
        } else {
            Ok(None)
        }
    }

    fn read_input(&mut self) -> io::Result<()> {
        match self.term.read(&mut self.data.input_buffer)? {
            RawRead::Bytes(_) => (),
            RawRead::Resize(new_size) => {
                self.last_resize = Some(new_size);
            }
            RawRead::Signal(sig) => {
                self.last_signal = Some(sig);
            }
        }

        Ok(())
    }

    fn is_input_available(&self) -> bool {
        !self.data.macro_buffer.is_empty() || match self.peek_input() {
            Ok(Some(_)) | Err(_) => true,
            Ok(None) => false
        }
    }

    fn macro_pop(&mut self) -> Option<char> {
        if self.data.macro_buffer.is_empty() {
            None
        } else {
            Some(self.data.macro_buffer.remove(0))
        }
    }

    fn decode_input(&mut self) -> io::Result<Option<char>> {
        let res = self.peek_input();

        if let Ok(Some(ch)) = res {
            self.data.input_buffer.drain(..ch.len_utf8());
        }

        res
    }

    fn peek_input(&self) -> io::Result<Option<char>> {
        if self.data.input_buffer.is_empty() {
            Ok(None)
        } else {
            first_char(&self.data.input_buffer)
        }
    }

    pub fn reset_data(&mut self) {
        self.data.reset_data();
    }
}

impl<'a, Term: 'a + Terminal> Deref for ReadLock<'a, Term> {
    type Target = Read<Term>;

    fn deref(&self) -> &Read<Term> {
        &self.data
    }
}

impl<'a, Term: 'a + Terminal> DerefMut for ReadLock<'a, Term> {
    fn deref_mut(&mut self) -> &mut Read<Term> {
        &mut self.data
    }
}

impl<Term: Terminal> Deref for Read<Term> {
    type Target = Variables;

    fn deref(&self) -> &Variables {
        &self.variables
    }
}

impl<Term: Terminal> DerefMut for Read<Term> {
    fn deref_mut(&mut self) -> &mut Variables {
        &mut self.variables
    }
}

impl<Term: Terminal> Read<Term> {
    pub fn new(term: &Term, application: Cow<'static, str>) -> Read<Term> {
        let mut r = Read{
            application,

            bindings: default_bindings(),
            functions: HashMap::new(),

            input_buffer: Vec::new(),
            macro_buffer: String::new(),

            sequence: String::new(),
            input_accepted: false,

            overwrite_mode: false,
            overwritten_append: 0,
            overwritten_chars: String::new(),

            completer: Arc::new(DummyCompleter),
            completion_append_character: Some(' '),
            completions: None,
            completion_index: 0,
            completion_start: 0,
            completion_prefix: 0,

            string_chars: STRING_CHARS.into(),
            word_break: WORD_BREAK_CHARS.into(),

            last_cmd: Category::Other,
            last_yank: None,
            kill_ring: VecDeque::with_capacity(MAX_KILLS),

            catch_signals: true,
            ignore_signals: SignalSet::new(),
            report_signals: SignalSet::new(),
            last_resize: None,
            last_signal: None,

            variables: Variables::default(),

            state: InputState::Inactive,
            max_wait_duration: None,
        };

        r.read_init(term);
        r
    }

    pub fn bindings(&self) -> BindingIter {
        BindingIter(self.bindings.sequences().iter())
    }

    pub fn variables(&self) -> VariableIter {
        self.variables.iter()
    }

    fn take_resize(&mut self) -> Option<Size> {
        self.last_resize.take()
    }

    fn take_signal(&mut self) -> Option<Signal> {
        self.last_signal.take()
    }

    pub fn queue_input(&mut self, seq: &str) {
        self.macro_buffer.insert_str(0, seq);
    }

    pub fn is_active(&self) -> bool {
        match self.state {
            InputState::Inactive => false,
            _ => true
        }
    }

    pub fn reset_data(&mut self) {
        self.state = InputState::NewSequence;
        self.input_accepted = false;
        self.overwrite_mode = false;
        self.overwritten_append = 0;
        self.overwritten_chars.clear();
        self.sequence.clear();

        self.completions = None;

        self.last_cmd = Category::Other;
        self.last_yank = None;

        self.last_resize = None;
        self.last_signal = None;
    }

    pub fn bind_sequence<T>(&mut self, seq: T, cmd: Command) -> Option<Command>
            where T: Into<Cow<'static, str>> {
        self.bindings.insert(seq.into(), cmd)
    }

    pub fn bind_sequence_if_unbound<T>(&mut self, seq: T, cmd: Command) -> bool
            where T: Into<Cow<'static, str>> {
        use mortal::sequence::Entry;

        match self.bindings.entry(seq.into()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(ent) => {
                ent.insert(cmd);
                true
            }
        }
    }

    pub fn unbind_sequence(&mut self, seq: &str) -> Option<Command> {
        self.bindings.remove(seq)
            .map(|(_, cmd)| cmd)
    }

    pub fn define_function<T>(&mut self, name: T, cmd: Arc<dyn Function<Term>>)
            -> Option<Arc<dyn Function<Term>>> where T: Into<Cow<'static, str>> {
        self.functions.insert(name.into(), cmd)
    }

    pub fn remove_function(&mut self, name: &str) -> Option<Arc<dyn Function<Term>>> {
        self.functions.remove(name)
    }

    fn read_init(&mut self, term: &Term) {
        if let Some(path) = env_init_file() {
            // If `INPUTRC` is present, even if invalid, parse nothing else.
            // Thus, an empty `INPUTRC` will inhibit loading configuration.
            self.read_init_file_if_exists(term, Some(path));
        } else {
            if !self.read_init_file_if_exists(term, user_init_file()) {
                self.read_init_file_if_exists(term, system_init_file());
            }
        }
    }

    fn read_init_file_if_exists(&mut self, term: &Term, path: Option<PathBuf>) -> bool {
        match path {
            Some(ref path) if path.exists() => {
                self.read_init_file(term, path);
                true
            }
            _ => false
        }
    }

    fn read_init_file(&mut self, term: &Term, path: &Path) {
        if let Some(dirs) = parse_file(path) {
            self.evaluate_directives(term, dirs);
        }
    }

    /// Evaluates a series of configuration directives.
    pub(crate) fn evaluate_directives(&mut self, term: &Term, dirs: Vec<Directive>) {
        for dir in dirs {
            self.evaluate_directive(term, dir);
        }
    }

    /// Evaluates a single configuration directive.
    pub(crate) fn evaluate_directive(&mut self, term: &Term, dir: Directive) {
        match dir {
            Directive::Bind(seq, cmd) => {
                self.bind_sequence(seq, cmd);
            }
            Directive::Conditional{name, value, then_group, else_group} => {
                let name = name.as_ref().map(|s| &s[..]);

                if self.eval_condition(term, name, &value) {
                    self.evaluate_directives(term, then_group);
                } else {
                    self.evaluate_directives(term, else_group);
                }
            }
            Directive::SetVariable(name, value) => {
                self.set_variable(&name, &value);
            }
        }
    }

    fn eval_condition(&self, term: &Term, name: Option<&str>, value: &str) -> bool {
        match name {
            None => self.application == value,
            Some("lib") => value == "linefeed",
            Some("mode") => value == "emacs",
            Some("term") => self.term_matches(term, value),
            _ => false
        }
    }

    fn term_matches(&self, term: &Term, value: &str) -> bool {
        match_name(term.name(), value)
    }
}

/// Iterator over `Reader` bindings
pub struct BindingIter<'a>(slice::Iter<'a, (Cow<'static, str>, Command)>);

impl<'a> ExactSizeIterator for BindingIter<'a> {}

impl<'a> Iterator for BindingIter<'a> {
    type Item = (&'a str, &'a Command);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|&(ref s, ref cmd)| (&s[..], cmd))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n).map(|&(ref s, ref cmd)| (&s[..], cmd))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> DoubleEndedIterator for BindingIter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|&(ref s, ref cmd)| (&s[..], cmd))
    }
}

fn default_bindings() -> SequenceMap<Cow<'static, str>, Command> {
    use crate::command::Command::*;

    SequenceMap::from(vec![
        // Carriage return and line feed
        ("\r".into(), AcceptLine),
        ("\n".into(), AcceptLine),

        // Possible sequences for arrow keys, Home, End
        ("\x1b[A".into(), PreviousHistory),
        ("\x1b[B".into(), NextHistory),
        ("\x1b[C".into(), ForwardChar),
        ("\x1b[D".into(), BackwardChar),
        ("\x1b[H".into(), BeginningOfLine),
        ("\x1b[F".into(), EndOfLine),

        // More possible sequences for arrow keys, Home, End
        ("\x1bOA".into(), PreviousHistory),
        ("\x1bOB".into(), NextHistory),
        ("\x1bOC".into(), ForwardChar),
        ("\x1bOD".into(), BackwardChar),
        ("\x1bOH".into(), BeginningOfLine),
        ("\x1bOF".into(), EndOfLine),

        // Possible sequences for Insert, Delete
        ("\x1b[2~".into(), OverwriteMode),
        ("\x1b[3~".into(), DeleteChar),

        // Basic commands
        ("\x01"    .into(), BeginningOfLine),           // Ctrl-A
        ("\x02"    .into(), BackwardChar),              // Ctrl-B
        ("\x04"    .into(), DeleteChar),                // Ctrl-D
        ("\x05"    .into(), EndOfLine),                 // Ctrl-E
        ("\x06"    .into(), ForwardChar),               // Ctrl-F
        ("\x07"    .into(), Abort),                     // Ctrl-G
        ("\x08"    .into(), BackwardDeleteChar),        // Ctrl-H
        ("\x0b"    .into(), KillLine),                  // Ctrl-K
        ("\x0c"    .into(), ClearScreen),               // Ctrl-L
        ("\x0e"    .into(), NextHistory),               // Ctrl-N
        ("\x10"    .into(), PreviousHistory),           // Ctrl-P
        ("\x12"    .into(), ReverseSearchHistory),      // Ctrl-R
        ("\x14"    .into(), TransposeChars),            // Ctrl-T
        ("\x15"    .into(), BackwardKillLine),          // Ctrl-U
        ("\x16"    .into(), QuotedInsert),              // Ctrl-V
        ("\x17"    .into(), UnixWordRubout),            // Ctrl-W
        ("\x19"    .into(), Yank),                      // Ctrl-Y
        ("\x1d"    .into(), CharacterSearch),           // Ctrl-]
        ("\x7f"    .into(), BackwardDeleteChar),        // Rubout
        ("\x1b\x08".into(), BackwardKillWord),          // Escape, Ctrl-H
        ("\x1b\x1d".into(), CharacterSearchBackward),   // Escape, Ctrl-]
        ("\x1b\x7f".into(), BackwardKillWord),          // Escape, Rubout
        ("\x1bb"   .into(), BackwardWord),              // Escape, b
        ("\x1bd"   .into(), KillWord),                  // Escape, d
        ("\x1bf"   .into(), ForwardWord),               // Escape, f
        ("\x1bt"   .into(), TransposeWords),            // Escape, t
        ("\x1by"   .into(), YankPop),                   // Escape, y
        ("\x1b#"   .into(), InsertComment),             // Escape, #
        ("\x1b<"   .into(), BeginningOfHistory),        // Escape, <
        ("\x1b>"   .into(), EndOfHistory),              // Escape, >

        // Completion commands
        ("\t"   .into(), Complete),             // Tab
        ("\x1b?".into(), PossibleCompletions),  // Escape, ?
        ("\x1b*".into(), InsertCompletions),    // Escape, *

        // Digit commands
        ("\x1b-".into(), DigitArgument),    // Escape, -
        ("\x1b0".into(), DigitArgument),    // Escape, 0
        ("\x1b1".into(), DigitArgument),    // Escape, 1
        ("\x1b2".into(), DigitArgument),    // Escape, 2
        ("\x1b3".into(), DigitArgument),    // Escape, 3
        ("\x1b4".into(), DigitArgument),    // Escape, 4
        ("\x1b5".into(), DigitArgument),    // Escape, 5
        ("\x1b6".into(), DigitArgument),    // Escape, 6
        ("\x1b7".into(), DigitArgument),    // Escape, 7
        ("\x1b8".into(), DigitArgument),    // Escape, 8
        ("\x1b9".into(), DigitArgument),    // Escape, 9
    ])
}

fn limit_duration(dur: Option<Duration>, max: Option<Duration>) -> Option<Duration> {
    match (dur, max) {
        (dur, None) | (None, dur) => dur,
        (Some(dur), Some(max)) => Some(dur.min(max)),
    }
}
