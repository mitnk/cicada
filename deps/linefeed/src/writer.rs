//! Provides access to terminal write operations

#![allow(dead_code)] // XXX

use std::borrow::Cow::{self, Borrowed, Owned};
use std::collections::{vec_deque, VecDeque};
use std::fmt;
use std::io;
use std::iter::Skip;
use std::mem::swap;
use std::ops::{Deref, DerefMut, Range};
use std::sync::MutexGuard;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::str;

use crate::chars::{is_ctrl, unctrl, ESCAPE, RUBOUT};
use crate::highlighting::{Highlighter, Style};
use crate::reader::{START_INVISIBLE, END_INVISIBLE};
use crate::terminal::{CursorMode, Size, Terminal, TerminalWriter};
use crate::util::{
    backward_char, forward_char, backward_search_char, forward_search_char,
    filter_visible, is_combining_mark, is_wide, RangeArgument,
};

/// Duration to wait for input when "blinking"
pub(crate) const BLINK_DURATION: Duration = Duration::from_millis(500);

const COMPLETE_MORE: &'static str = "--More--";

/// Default maximum history size
const MAX_HISTORY: usize = !0;

/// Tab column interval
const TAB_STOP: usize = 8;

// Length of "(arg: "
const PROMPT_NUM_PREFIX: usize = 6;
// Length of ") "
const PROMPT_NUM_SUFFIX: usize = 2;

// Length of "(i-search)`"
const PROMPT_SEARCH_PREFIX: usize = 11;
// Length of "failed "
const PROMPT_SEARCH_FAILED_PREFIX: usize = 7;
// Length of "reverse-"
const PROMPT_SEARCH_REVERSE_PREFIX: usize = 8;
// Length of "': "
const PROMPT_SEARCH_SUFFIX: usize = 3;

/// Provides an interface to write line-by-line output to the terminal device.
///
/// Holds a lock on terminal write operations.
/// See [`Interface`] for more information about concurrent operations.
///
/// An instance of this type can be constructed using either the
/// [`Interface::lock_writer_append`] or the [`Interface::lock_writer_erase`]
/// method.
///
/// [`Interface`]: ../interface/struct.Interface.html
/// [`Interface::lock_writer_append`]: ../interface/struct.Interface.html#method.lock_writer_append
/// [`Interface::lock_writer_erase`]: ../interface/struct.Interface.html#method.lock_writer_erase
pub struct Writer<'a, 'b: 'a, Term: 'b + Terminal> {
    write: WriterImpl<'a, 'b, Term>,
}

enum WriterImpl<'a, 'b: 'a, Term: 'b + Terminal> {
    Mutex(WriteLock<'b, Term>),
    MutRef(&'a mut WriteLock<'b, Term>),
}

pub(crate) struct Write {
    /// Input buffer
    pub buffer: String,
    /// Original buffer entered before searching through history
    pub backup_buffer: String,
    /// Position of the cursor
    pub cursor: usize,
    /// Position of the cursor if currently performing a blink
    blink: Option<Blink>,

    /// Stored history entries
    pub history: VecDeque<String>,
    /// History entry currently being edited;
    /// `None` if the new buffer is being edited
    pub history_index: Option<usize>,
    /// Maximum size of history
    history_size: usize,
    /// Number of history entries added since last loading history
    history_new_entries: usize,

    /// Whether the prompt is drawn; i.e. a `read_line` operation is in progress
    pub is_prompt_drawn: bool,

    /// Portion of prompt up to and including the final newline
    pub prompt_prefix: String,
    prompt_prefix_len: usize,
    /// Portion of prompt after the final newline
    pub prompt_suffix: String,
    prompt_suffix_len: usize,

    /// Current type of prompt
    pub prompt_type: PromptType,

    /// Whether a search in progress is a reverse search
    pub reverse_search: bool,
    /// Whether a search in progress has failed to find a match
    pub search_failed: bool,
    /// Current search string
    pub search_buffer: String,
    /// Last search string
    pub last_search: String,
    /// Selected history entry prior to a history search
    pub prev_history: Option<usize>,
    /// Position of the cursor prior to a history search
    pub prev_cursor: usize,

    /// Numerical argument
    pub input_arg: Digit,
    /// Whether a numerical argument was supplied
    pub explicit_arg: bool,

    /// Terminal size as of last draw operation
    pub screen_size: Size,
}

pub(crate) struct WriteLock<'a, Term: 'a + Terminal> {
    term: Box<dyn TerminalWriter<Term> + 'a>,
    data: MutexGuard<'a, Write>,
    highlighter: Option<Arc<dyn Highlighter + Send + Sync>>,
}

impl<'a, Term: Terminal> WriteLock<'a, Term> {
    pub fn new(term: Box<dyn TerminalWriter<Term> + 'a>, data: MutexGuard<'a, Write>,
            highlighter: Option<Arc<dyn Highlighter + Send + Sync>>
            ) -> WriteLock<'a, Term> {
        WriteLock{
            term,
            data,
            highlighter,
        }
    }

    pub fn size(&self) -> io::Result<Size> {
        self.term.size()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.term.flush()
    }

    pub fn update_size(&mut self) -> io::Result<()> {
        let size = self.size()?;
        self.screen_size = size;
        Ok(())
    }

    pub fn blink(&mut self, pos: usize) -> io::Result<()> {
        self.expire_blink()?;

        let orig = self.cursor;
        self.move_to(pos)?;
        self.cursor = orig;

        let expiry = Instant::now() + BLINK_DURATION;

        self.blink = Some(Blink{
            pos,
            expiry,
        });

        Ok(())
    }

    pub fn check_expire_blink(&mut self, now: Instant) -> io::Result<bool> {
        if let Some(blink) = self.data.blink {
            if now >= blink.expiry {
                self.expire_blink()?;
            }
        }

        Ok(self.blink.is_none())
    }

    pub fn expire_blink(&mut self) -> io::Result<()> {
        if let Some(blink) = self.data.blink.take() {
            self.move_from(blink.pos)?;
        }

        Ok(())
    }

    pub fn set_prompt(&mut self, prompt: &str) -> io::Result<()> {
        self.expire_blink()?;

        let redraw = self.is_prompt_drawn && self.prompt_type.is_normal();

        if redraw {
            self.clear_full_prompt()?;
        }

        self.data.set_prompt(prompt);

        if redraw {
            self.draw_prompt()?;
        }

        Ok(())
    }

    /// Draws the prompt and current input, assuming the cursor is at column 0
    pub fn draw_prompt(&mut self) -> io::Result<()> {
        self.draw_prompt_prefix()?;
        self.draw_prompt_suffix()
    }

    pub fn draw_prompt_prefix(&mut self) -> io::Result<()> {
        match self.prompt_type {
            // Prefix is not drawn when completions are shown
            PromptType::CompleteMore => Ok(()),
            _ => {
                let pfx = self.prompt_prefix.clone();
                let disp = Display{
                    allow_tab: true,
                    allow_newline: true,
                    allow_escape: true,
                };
                let handle_invisible = true;

                let styles = self.highlighter.as_ref().map(|h|
                    h.highlight(&pfx)
                ).unwrap_or_default();

                self.draw_text_slice(0, &pfx, styles, disp, handle_invisible)?;
                Ok(())
            }
        }
    }

    pub fn draw_prompt_suffix(&mut self) -> io::Result<()> {
        match self.prompt_type {
            PromptType::Normal => {
                let sfx = self.prompt_suffix.clone();
                let disp = Display{
                    allow_tab: true,
                    allow_newline: true,
                    allow_escape: true,
                };
                let handle_invisible = true;

                let styles = self.highlighter.as_ref().map(|h|
                    h.highlight(&sfx)
                ).unwrap_or_default();

                self.draw_text_slice(0, &sfx, styles, disp, handle_invisible)?;
            }
            PromptType::Number => {
                let n = self.input_arg.to_i32();
                let s = format!("(arg: {}) ", n);
                self.draw_text_slice(0, &s, Vec::new(), Display::default(), false)?;
            }
            PromptType::Search => {
                let pre = match (self.reverse_search, self.search_failed) {
                    (false, false) => "(i-search)",
                    (false, true)  => "(failed i-search)",
                    (true,  false) => "(reverse-i-search)",
                    (true,  true)  => "(failed reverse-i-search)",
                };

                let ent = self.get_history(self.history_index).to_owned();
                let s = format!("{}`{}': {}", pre, self.search_buffer, ent);

                self.draw_text_slice(0, &s, Vec::new(), Display::default(), false)?;
                let pos = self.cursor;

                let (lines, cols) = self.move_delta(ent.len(), pos, &ent);
                return self.move_rel(lines, cols);
            }
            PromptType::CompleteIntro(n) => {
                return self.term.write(&complete_intro(n));
            }
            PromptType::CompleteMore => {
                return self.term.write(COMPLETE_MORE);
            }
        }

        self.draw_buffer(0)?;
        let len = self.buffer.len();
        self.move_from(len)
    }

    pub fn redraw_prompt(&mut self, new_prompt: PromptType) -> io::Result<()> {
        self.clear_prompt()?;
        self.prompt_type = new_prompt;
        self.draw_prompt_suffix()
    }

    /// Draws a portion of the buffer, starting from the given cursor position
    pub fn draw_buffer(&mut self, pos: usize) -> io::Result<()> {
        let (_, col) = self.line_col(pos);
        let buf_to_draw = self.buffer[pos..].to_owned(); // Materialize the slice to draw
        let disp = Display{ allow_tab: true, allow_newline: true, .. Display::default() };
        let handle_invisible = false;

        // If highlighter exists, calculate styles for the slice and draw using the function
        let styles = if let Some(highlighter) = self.highlighter.as_ref() {
            let full_styles = highlighter.highlight(&self.buffer); // Highlight full buffer

            // Filter and adjust styles for the slice `pos..`
            full_styles.into_iter()
                .filter(|(range, _)| range.end > pos)
                .filter_map(|(range, style)| {
                    let slice_start = pos;
                    let slice_end = self.buffer.len(); // Use full buffer length
                    let overlap_start = range.start.max(slice_start);
                    let overlap_end = range.end.min(slice_end);
                    if overlap_start < overlap_end {
                        let relative_start = overlap_start - slice_start;
                        let relative_end = overlap_end - slice_start;
                        Some((relative_start..relative_end, style))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>() // Collect adjusted styles
        } else {
            Vec::new() // No highlighter, empty styles
        };

        self.draw_text_slice(col, &buf_to_draw, styles, disp, handle_invisible)?;

        Ok(())
    }

    fn draw_text_slice(
        &mut self,
        start_col: usize,
        text: &str,
        styles: Vec<(Range<usize>, Style)>,
        disp: Display,
        handle_invisible: bool,
    ) -> io::Result<()> {
        let mut current_style = Style::Default;
        let mut style_iter = styles.into_iter().peekable();

        let mut cur_col = start_col;
        let mut text_pos = 0; // Position within text
        let width = self.screen_size.columns;

        for chr in text.chars() {
            let char_start_text_pos = text_pos;
            text_pos += chr.len_utf8();

            // Style lookup logic
            let mut next_style = Style::Default;
            while let Some((range, _)) = style_iter.peek() {
                if char_start_text_pos >= range.end {
                    style_iter.next();
                } else if char_start_text_pos >= range.start {
                    next_style = style_iter.peek().unwrap().1.clone();
                    break;
                } else {
                    break;
                }
            }
            // Apply style changes
            if next_style != current_style {
                match &next_style {
                    Style::AnsiColor(code) => self.term.write(code)?,
                    Style::Default => self.term.write("\x1b[0m")?,
                }
                current_style = next_style;
            }

            // Character drawing logic
            let mut char_buf = [0u8; 4];
            if handle_invisible && chr == START_INVISIBLE {
                 self.term.write(chr.encode_utf8(&mut char_buf))?;
            } else if handle_invisible && chr == END_INVISIBLE {
                 self.term.write(chr.encode_utf8(&mut char_buf))?;
            } else if chr == '\t' && disp.allow_tab {
                let n = TAB_STOP - (cur_col % TAB_STOP);
                let spaces = vec![b' '; n];
                if cur_col + n > width {
                    let pre = width - cur_col;
                    self.term.write(str::from_utf8(&vec![b' '; pre]).unwrap_or(""))?;
                    self.term.write(" \r")?;
                    self.term.write(str::from_utf8(&vec![b' '; n - pre]).unwrap_or(""))?;
                    cur_col = n - pre;
                } else {
                    self.term.write(str::from_utf8(&spaces).unwrap_or(""))?;
                    cur_col += n;
                    if cur_col == width {
                         self.term.write(" \r")?;
                         cur_col = 0;
                    }
                }
            } else if chr == '\n' && disp.allow_newline {
                self.term.write("\n")?;
                self.term.move_to_first_column()?;
                self.term.clear_to_screen_end()?;
                cur_col = 0;
            } else if is_combining_mark(chr) {
                 self.term.write(chr.encode_utf8(&mut char_buf))?;
            } else if is_wide(chr) {
                 if cur_col >= width -1 {
                     self.term.write("  \r")?;
                     self.term.write(chr.encode_utf8(&mut char_buf))?;
                     cur_col = 2;
                 } else {
                     self.term.write(chr.encode_utf8(&mut char_buf))?;
                     cur_col += 2;
                 }
            } else if chr == ESCAPE && disp.allow_escape {
                 self.term.write(chr.encode_utf8(&mut char_buf))?;
                 cur_col += 1;
            } else if is_ctrl(chr) {
                 self.term.write("^")?;
                 self.term.write(unctrl(chr).encode_utf8(&mut char_buf))?;
                 cur_col += 2;
            } else { // Regular printable char
                 self.term.write(chr.encode_utf8(&mut char_buf))?;
                 cur_col += 1;
            }

            if !is_wide(chr) && chr != '\t' && chr != '\n' && cur_col == width {
                 self.term.write(" \r")?;
                 cur_col = 0;
            }
        }

        // Reset style at the end
        if current_style != Style::Default {
            self.term.write("\x1b[0m")?;
        }

        self.term.flush()
    }

    pub fn set_buffer(&mut self, buf: &str) -> io::Result<()> {
        self.expire_blink()?;

        self.move_to(0)?;
        self.buffer.clear();
        self.buffer.push_str(buf);
        self.new_buffer()
    }

    pub fn set_cursor(&mut self, pos: usize) -> io::Result<()> {
        self.expire_blink()?;

        if !self.buffer.is_char_boundary(pos) {
            panic!("invalid cursor position {} in buffer {:?}",
                pos, self.buffer);
        }

        self.move_to(pos)
    }

    pub fn set_cursor_mode(&mut self, mode: CursorMode) -> io::Result<()> {
        self.term.set_cursor_mode(mode)
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn history_size(&self) -> usize {
        self.history_size
    }

    pub fn set_history_size(&mut self, n: usize) {
        self.history_size = n;
        self.truncate_history(n);
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.term.write(s)
    }

    pub fn start_history_search(&mut self, reverse: bool) -> io::Result<()> {
        self.search_buffer = self.buffer[..self.cursor].to_owned();

        self.continue_history_search(reverse)
    }

    pub fn continue_history_search(&mut self, reverse: bool) -> io::Result<()> {
        if let Some(idx) = self.find_history_search(reverse) {
            // Use the same logic as select_history_entry for redraw robustness.
            self.move_to(0)?;
            self.set_history_entry(Some(idx));
            self.new_buffer()?;

            // After redrawing, move cursor to position after the original search prefix.
            let prefix_len = self.search_buffer.len();
            self.move_to(prefix_len)?;
        }

        Ok(())
    }

    fn find_history_search(&self, reverse: bool) -> Option<usize> {
        let len = self.history.len();
        let idx = self.history_index.unwrap_or(len);

        if reverse {
            self.history.iter().rev().skip(len - idx)
                .position(|ent| ent.starts_with(&self.search_buffer))
                .map(|pos| idx - (pos + 1))
        } else {
            self.history.iter().skip(idx + 1)
                .position(|ent| ent.starts_with(&self.search_buffer))
                .map(|pos| idx + (pos + 1))
        }
    }

    pub fn start_search_history(&mut self, reverse: bool) -> io::Result<()> {
        self.reverse_search = reverse;
        self.search_failed = false;
        self.search_buffer.clear();
        self.prev_history = self.history_index;
        self.prev_cursor = self.cursor;

        self.redraw_prompt(PromptType::Search)
    }

    pub fn continue_search_history(&mut self, reverse: bool) -> io::Result<()> {
        self.reverse_search = reverse;
        self.search_failed = false;

        {
            let data = &mut *self.data;
            data.search_buffer.clone_from(&data.last_search);
        }

        self.search_history_step()
    }

    pub fn end_search_history(&mut self) -> io::Result<()> {
        self.redraw_prompt(PromptType::Normal)
    }

    pub fn abort_search_history(&mut self) -> io::Result<()> {
        self.clear_prompt()?;

        let ent = self.prev_history;
        self.set_history_entry(ent);
        self.cursor = self.prev_cursor;

        self.prompt_type = PromptType::Normal;
        self.draw_prompt_suffix()
    }

    fn show_search_match(&mut self, next_match: Option<(Option<usize>, usize)>)
            -> io::Result<()> {
        self.clear_prompt()?;

        if let Some((idx, pos)) = next_match {
            self.search_failed = false;
            self.set_history_entry(idx);
            self.cursor = pos;
        } else {
            self.search_failed = true;
        }

        self.prompt_type = PromptType::Search;
        self.draw_prompt_suffix()
    }

    pub fn search_history_update(&mut self) -> io::Result<()> {
        // Search for the next match, perhaps including the current position
        let next_match = if self.reverse_search {
            self.search_history_backward(&self.search_buffer, true)
        } else {
            self.search_history_forward(&self.search_buffer, true)
        };

        self.show_search_match(next_match)
    }

    fn search_history_step(&mut self) -> io::Result<()> {
        if self.search_buffer.is_empty() {
            return self.redraw_prompt(PromptType::Search);
        }

        // Search for the next match
        let next_match = if self.reverse_search {
            self.search_history_backward(&self.search_buffer, false)
        } else {
            self.search_history_forward(&self.search_buffer, false)
        };

        self.show_search_match(next_match)
    }

    fn search_history_backward(&self, s: &str, include_cur: bool)
            -> Option<(Option<usize>, usize)> {
        let mut idx = self.history_index;
        let mut pos = Some(self.cursor);

        if include_cur && !self.search_failed {
            if let Some(p) = pos {
                if self.get_history(idx).is_char_boundary(p + s.len()) {
                    pos = Some(p + s.len());
                }
            }
        }

        loop {
            let line = self.get_history(idx);

            match line[..pos.unwrap_or(line.len())].rfind(s) {
                Some(found) => {
                    pos = Some(found);
                    break;
                }
                None => {
                    match idx {
                        Some(0) => return None,
                        Some(n) => {
                            idx = Some(n - 1);
                            pos = None;
                        }
                        None => {
                            if self.history.is_empty() {
                                return None;
                            } else {
                                idx = Some(self.history.len() - 1);
                                pos = None;
                            }
                        }
                    }
                }
            }
        }

        pos.map(|pos| (idx, pos))
    }

    fn search_history_forward(&self, s: &str, include_cur: bool)
            -> Option<(Option<usize>, usize)> {
        let mut idx = self.history_index;
        let mut pos = Some(self.cursor);

        if !include_cur {
            if let Some(p) = pos {
                pos = Some(forward_char(1, self.get_history(idx), p));
            }
        }

        loop {
            let line = self.get_history(idx);

            match line[pos.unwrap_or(0)..].find(s) {
                Some(found) => {
                    pos = pos.map(|n| n + found).or(Some(found));
                    break;
                }
                None => {
                    if let Some(n) = idx {
                        if n + 1 == self.history.len() {
                            idx = None;
                        } else {
                            idx = Some(n + 1);
                        }
                        pos = None;
                    } else {
                        return None;
                    }
                }
            }
        }

        pos.map(|pos| (idx, pos))
    }

    pub fn add_history(&mut self, line: String) {
        if self.history.len() == self.history_size {
            self.history.pop_front();
        }

        self.history.push_back(line);
        self.history_new_entries = self.history.len()
            .min(self.history_new_entries + 1);
    }

    pub fn add_history_unique(&mut self, line: String) {
        let is_duplicate = self.history.back().map_or(false, |ent| *ent == line);

        if !is_duplicate {
            self.add_history(line);
        }
    }

    pub fn clear_history(&mut self) {
        self.truncate_history(0);
        self.history_new_entries = 0;
    }

    pub fn remove_history(&mut self, n: usize) {
        if n < self.history.len() {
            let first_new = self.history.len() - self.history_new_entries;

            if n >= first_new {
                self.history_new_entries -= 1;
            }

            self.history.remove(n);
        }
    }

    pub fn truncate_history(&mut self, n: usize) {
        let len = self.history.len();

        if n < len {
            let _ = self.history.drain(..len - n);
            self.history_new_entries = self.history_new_entries.max(n);
        }
    }

    pub fn next_history(&mut self, n: usize) -> io::Result<()> {
        if let Some(old) = self.history_index {
            let new = old.saturating_add(n);

            if new >= self.history.len() {
                self.select_history_entry(None)?;
            } else {
                self.select_history_entry(Some(new))?;
            }
        }

        Ok(())
    }

    pub fn prev_history(&mut self, n: usize) -> io::Result<()> {
        if !self.history.is_empty() && self.history_index != Some(0) {
            let new = if let Some(old) = self.history_index {
                old.saturating_sub(n)
            } else {
                self.history.len().saturating_sub(n)
            };

            self.select_history_entry(Some(new))?;
        }

        Ok(())
    }

    pub fn select_history_entry(&mut self, new: Option<usize>) -> io::Result<()> {
        if new != self.history_index {
            self.move_to(0)?;
            self.set_history_entry(new);
            self.new_buffer()?;
        }

        Ok(())
    }

    pub fn set_history_entry(&mut self, new: Option<usize>) {
        let old = self.history_index;

        if old != new {
            let data = &mut *self.data;
            data.history_index = new;

            if let Some(old) = old {
                data.history[old].clone_from(&data.buffer);
            } else {
                swap(&mut data.buffer, &mut data.backup_buffer);
            }

            if let Some(new) = new {
                data.buffer.clone_from(&data.history[new]);
            } else {
                data.buffer.clear();
                swap(&mut data.buffer, &mut data.backup_buffer);
            }
        }
    }

    fn get_history(&self, n: Option<usize>) -> &str {
        if self.history_index == n {
            &self.buffer
        } else if let Some(n) = n {
            &self.history[n]
        } else {
            &self.backup_buffer
        }
    }

    pub fn backward_char(&mut self, n: usize) -> io::Result<()> {
        let pos = backward_char(n, &self.buffer, self.cursor);
        self.move_to(pos)
    }

    pub fn forward_char(&mut self, n: usize) -> io::Result<()> {
        let pos = forward_char(n, &self.buffer, self.cursor);
        self.move_to(pos)
    }

    pub fn backward_search_char(&mut self, n: usize, ch: char) -> io::Result<()> {
        if let Some(pos) = backward_search_char(n, &self.buffer, self.cursor, ch) {
            self.move_to(pos)?;
        }

        Ok(())
    }

    pub fn forward_search_char(&mut self, n: usize, ch: char) -> io::Result<()> {
        if let Some(pos) = forward_search_char(n, &self.buffer, self.cursor, ch) {
            self.move_to(pos)?;
        }

        Ok(())
    }

    /// Deletes a range from the buffer; the cursor is moved to the end
    /// of the given range.
    pub fn delete_range<R: RangeArgument<usize>>(&mut self, range: R) -> io::Result<()> {
        let start = range.start().cloned().unwrap_or(0);
        let end = range.end().cloned().unwrap_or_else(|| self.buffer.len());
        let old_cursor_pos = self.cursor; // Store cursor position *before* moving/deleting

        // Validate range is within buffer bounds and logical
        if start > end || end > self.buffer.len() {
            // Handle invalid range gracefully, maybe log or return error? For now, do nothing.
            return Ok(());
        }

        // --- Calculate old screen position BEFORE modifying buffer ---
        // Clone buffer *before* drain to calculate the old position accurately.
        // Using the buffer state that corresponds to old_cursor_pos.
        let old_buffer_for_calc = self.buffer.clone();
        let prompt_len = self.prompt_suffix_length(); // Calculate once
        let (old_line, old_col) = self.line_col_with(old_cursor_pos, &old_buffer_for_calc, prompt_len);

        // --- Modify buffer ---
        // Drain the specified range from the buffer
        let _ = self.buffer.drain(start..end);
        let new_cursor_pos = start; // Cursor logically moves to the start of the deleted range

        // Determine the position from where we need to start redrawing.
        // Find the start of the word containing the deletion point (`start`).
        let mut redraw_start_pos = 0;
        if new_cursor_pos > 0 {
            // Search backwards from the new cursor position in the modified buffer
            if let Some(ws_pos) = self.buffer[..new_cursor_pos].rfind(char::is_whitespace) {
                 // Found whitespace, redraw starts after it.
                 redraw_start_pos = ws_pos + self.buffer[ws_pos..].chars().next().map_or(0, |c| c.len_utf8());
            } // else redraw_start_pos remains 0 (start of the line)
        }

        // --- Calculate new screen position AFTER modifying buffer ---
        // Use the current buffer state corresponding to redraw_start_pos.
        let (redraw_line, redraw_col) = self.line_col_with(redraw_start_pos, &self.buffer, prompt_len);

        // --- Redrawing Logic ---
        // 1. Move the physical terminal cursor using relative movement
        //    Calculate delta between the old screen pos and the new redraw start screen pos.
        let lines_delta = redraw_line as isize - old_line as isize;
        let cols_delta = redraw_col as isize - old_col as isize;
        self.move_rel(lines_delta, cols_delta)?;

        // 2. Redraw the buffer content starting from redraw_start_pos
        self.draw_buffer(redraw_start_pos)?;

        // 3. Clear the rest of the screen from the end of the newly drawn buffer content
        self.clear_to_screen_end()?;

        // 4. Update the logical cursor position *after* drawing
        self.cursor = new_cursor_pos;

        // 5. Move the physical terminal cursor from the *end* of the drawn buffer content
        //    to the new logical cursor position, using the existing helper.
        self.move_from(self.buffer.len())?; // Moves cursor from end of line to self.cursor

        Ok(())
    }

    pub fn insert_str(&mut self, s: &str) -> io::Result<()> {
        let insertion_cursor = self.cursor;
        self.buffer.insert_str(insertion_cursor, s);
        let new_cursor_pos = insertion_cursor + s.len();

        // Determine the position from where we need to start redrawing
        // We need to be smarter about finding the start of the current word
        let redraw_start_pos = if insertion_cursor == 0 {
            // If we're at the start of the buffer, redraw everything
            0
        } else {
            // Start with the assumption that we redraw from the insertion point
            let mut pos = insertion_cursor;

            // Find the first non-whitespace character before the insertion point
            // This handles leading whitespace better
            let before_insertion = &self.buffer[..insertion_cursor];

            // Check if there's any non-whitespace before the cursor
            if before_insertion.trim_start().is_empty() {
                // Only whitespace before cursor, start redraw from beginning
                0
            } else {
                // There's at least one non-whitespace char before cursor
                // Find the start of the current word by going back until whitespace
                let mut has_found_nonws = false;

                for (i, c) in before_insertion.char_indices().rev() {
                    if !has_found_nonws && !c.is_whitespace() {
                        // Found a non-whitespace character
                        has_found_nonws = true;
                    } else if has_found_nonws && c.is_whitespace() {
                        // Found the boundary between whitespace and word
                        // Start redrawing after this whitespace
                        pos = i + c.len_utf8();
                        break;
                    }
                }

                // If we examined all chars without finding a word boundary,
                // we're in the first word, so start from the beginning or after leading whitespace
                if !has_found_nonws || pos == insertion_cursor {
                    // Find the start of the first word (after any leading whitespace)
                    let mut start = 0;
                    for (i, c) in self.buffer.char_indices() {
                        if !c.is_whitespace() {
                            break;
                        }
                        start = i + c.len_utf8();
                    }
                    pos = start;
                }

                pos
            }
        };

        // Log for debugging
        // println!("Redraw from pos {} in buffer {:?}", redraw_start_pos, self.buffer);

        // --- Redrawing Logic ---
        // 1. Move the physical terminal cursor to the screen position of redraw_start_pos
        let (lines, cols) = self.move_delta(insertion_cursor, redraw_start_pos, &self.buffer);
        self.move_rel(lines, cols)?;

        // 2. Redraw the buffer content starting from redraw_start_pos
        self.draw_buffer(redraw_start_pos)?;

        // 3. Update the logical cursor position *after* drawing
        self.cursor = new_cursor_pos;

        // 4. Move the physical terminal cursor from the *end* of the drawn buffer
        //    to the new logical cursor position.
        let len = self.buffer.len();
        self.move_from(len)?; // This calculates delta from len to self.cursor (new_cursor_pos)

        Ok(())
    }

    pub fn transpose_range(&mut self, src: Range<usize>, dest: Range<usize>)
            -> io::Result<()> {
        // Ranges must not overlap
        assert!(src.end <= dest.start || src.start >= dest.end);

        // Final cursor position
        let final_cur = if src.start < dest.start {
            dest.end
        } else {
            dest.start + (src.end - src.start)
        };

        let (left, right) = if src.start < dest.start {
            (src, dest)
        } else {
            (dest, src)
        };

        self.move_to(left.start)?;

        let a = self.buffer[left.clone()].to_owned();
        let b = self.buffer[right.clone()].to_owned();

        let _ = self.buffer.drain(right.clone());
        self.buffer.insert_str(right.start, &a);

        let _ = self.buffer.drain(left.clone());
        self.buffer.insert_str(left.start, &b);

        let cursor = self.cursor;
        self.draw_buffer(cursor)?;
        self.term.clear_to_screen_end()?;

        self.cursor = final_cur;
        let len = self.buffer.len();
        self.move_from(len)
    }

    fn prompt_suffix_length(&self) -> usize {
        match self.prompt_type {
            PromptType::Normal => self.prompt_suffix_len,
            PromptType::Number => {
                let n = number_len(self.input_arg.to_i32());
                PROMPT_NUM_PREFIX + PROMPT_NUM_SUFFIX + n
            }
            PromptType::Search => {
                let mut prefix = PROMPT_SEARCH_PREFIX;

                if self.reverse_search {
                    prefix += PROMPT_SEARCH_REVERSE_PREFIX;
                }
                if self.search_failed {
                    prefix += PROMPT_SEARCH_FAILED_PREFIX;
                }

                let n = self.display_size(&self.search_buffer, prefix);
                prefix + n + PROMPT_SEARCH_SUFFIX
            }
            PromptType::CompleteIntro(n) => complete_intro(n).len(),
            PromptType::CompleteMore => COMPLETE_MORE.len(),
        }
    }

    fn line_col(&self, pos: usize) -> (usize, usize) {
        let prompt_len = self.prompt_suffix_length();

        match self.prompt_type {
            PromptType::CompleteIntro(_) |
            PromptType::CompleteMore => {
                let width = self.screen_size.columns;
                (prompt_len / width, prompt_len % width)
            }
            _ => self.line_col_with(pos, &self.buffer, prompt_len)
        }
    }

    fn line_col_with(&self, pos: usize, buf: &str, start_col: usize) -> (usize, usize) {
        let width = self.screen_size.columns;
        if width == 0 {
            return (0, 0);
        }

        let n = start_col + self.display_size(&buf[..pos], start_col);

        (n / width, n % width)
    }

    pub fn clear_screen(&mut self) -> io::Result<()> {
        self.term.clear_screen()?;
        self.draw_prompt()?;

        Ok(())
    }

    pub fn clear_to_screen_end(&mut self) -> io::Result<()> {
        self.term.clear_to_screen_end()
    }

    /// Draws a new buffer on the screen. Cursor position is assumed to be `0`.
    pub fn new_buffer(&mut self) -> io::Result<()> {
        self.draw_buffer(0)?;
        self.cursor = self.buffer.len();

        self.term.clear_to_screen_end()?;

        Ok(())
    }

    pub fn clear_full_prompt(&mut self) -> io::Result<()> {
        let prefix_lines = self.prompt_prefix_len / self.screen_size.columns;
        let (line, _) = self.line_col(self.cursor);
        self.term.move_up(prefix_lines + line)?;
        self.term.move_to_first_column()?;
        self.term.clear_to_screen_end()
    }

    pub(crate) fn clear_prompt(&mut self) -> io::Result<()> {
        let (line, _) = self.line_col(self.cursor);

        self.term.move_up(line)?;
        self.term.move_to_first_column()?;
        self.term.clear_to_screen_end()
    }

    /// Move back to true cursor position from some other position
    pub fn move_from(&mut self, pos: usize) -> io::Result<()> {
        let (lines, cols) = self.move_delta(pos, self.cursor, &self.buffer);
        self.move_rel(lines, cols)
    }

    pub fn move_to(&mut self, pos: usize) -> io::Result<()> {
        if pos != self.cursor {
            let (lines, cols) = self.move_delta(self.cursor, pos, &self.buffer);
            self.move_rel(lines, cols)?;
            self.cursor = pos;
        }

        Ok(())
    }

    pub fn move_to_end(&mut self) -> io::Result<()> {
        let pos = self.buffer.len();
        self.move_to(pos)
    }

    pub fn move_right(&mut self, n: usize) -> io::Result<()> {
        self.term.move_right(n)
    }

    /// Moves from `old` to `new` cursor position, using the given buffer
    /// as current input.
    fn move_delta(&self, old: usize, new: usize, buf: &str) -> (isize, isize) {
        let prompt_len = self.prompt_suffix_length();
        let (old_line, old_col) = self.line_col_with(old, buf, prompt_len);
        let (new_line, new_col) = self.line_col_with(new, buf, prompt_len);

        (new_line as isize - old_line as isize,
         new_col as isize - old_col as isize)
    }

    fn move_rel(&mut self, lines: isize, cols: isize) -> io::Result<()> {
        if lines > 0 {
            self.term.move_down(lines as usize)?;
        } else if lines < 0 {
            self.term.move_up((-lines) as usize)?;
        }

        if cols > 0 {
            self.term.move_right(cols as usize)?;
        } else if cols < 0 {
            self.term.move_left((-cols) as usize)?;
        }

        Ok(())
    }

    pub fn reset_data(&mut self) {
        self.data.reset_data();
    }

    pub fn set_digit_from_char(&mut self, ch: char) {
        let digit = match ch {
            '-' => Digit::NegNone,
            '0' ..= '9' => Digit::from(ch),
            _ => Digit::None
        };

        self.input_arg = digit;
        self.explicit_arg = true;
    }
}

#[derive(Copy, Clone)]
struct Blink {
    pos: usize,
    expiry: Instant,
}

impl<'a, 'b: 'a, Term: 'b + Terminal> Writer<'a, 'b, Term> {
    fn new(mut write: WriterImpl<'a, 'b, Term>, clear: bool) -> io::Result<Self> {
        write.expire_blink()?;

        if write.is_prompt_drawn {
            if clear {
                write.clear_full_prompt()?;
            } else {
                write.move_to_end()?;
                write.write_str("\n")?;
            }
        }

        Ok(Writer{write})
    }

    pub(crate) fn with_lock(write: WriteLock<'b, Term>, clear: bool) -> io::Result<Self> {
        Writer::new(WriterImpl::Mutex(write), clear)
    }

    pub(crate) fn with_ref(write: &'a mut WriteLock<'b, Term>, clear: bool) -> io::Result<Self> {
        Writer::new(WriterImpl::MutRef(write), clear)
    }

    /// Returns an iterator over history entries.
    pub fn history(&self) -> HistoryIter {
        self.write.history()
    }

    /// Writes some text to the terminal device.
    ///
    /// Before the `Writer` is dropped, any output written should be followed
    /// by a newline. A newline is automatically written if the `writeln!`
    /// macro is used.
    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.write.write_str(s)
    }

    /// Writes formatted text to the terminal display.
    ///
    /// This method enables `Interface` to be used as the receiver to
    /// the [`writeln!`] macro.
    ///
    /// If the text contains any unprintable characters (e.g. escape sequences),
    /// those characters will be escaped before printing.
    ///
    /// [`read_line`]: ../interface/struct.Interface.html#method.read_line
    /// [`writeln!`]: https://doc.rust-lang.org/std/macro.writeln.html
    pub fn write_fmt(&mut self, args: fmt::Arguments) -> io::Result<()> {
        let s = args.to_string();
        self.write_str(&s)
    }
}

impl<'a, 'b: 'a, Term: 'b + Terminal> Drop for Writer<'a, 'b, Term> {
    fn drop(&mut self) {
        if self.write.is_prompt_drawn {
            // There's not really anything useful to be done with this error.
            let _ = self.write.draw_prompt();
        }
    }
}

impl<'a, Term: 'a + Terminal> Deref for WriteLock<'a, Term> {
    type Target = Write;

    fn deref(&self) -> &Write {
        &self.data
    }
}

impl<'a, Term: 'a + Terminal> DerefMut for WriteLock<'a, Term> {
    fn deref_mut(&mut self) -> &mut Write {
        &mut self.data
    }
}

impl Write {
    pub fn new(screen_size: Size) -> Write {
        Write{
            buffer: String::new(),
            backup_buffer: String::new(),
            cursor: 0,
            blink: None,

            history: VecDeque::new(),
            history_index: None,
            history_size: MAX_HISTORY,
            history_new_entries: 0,

            is_prompt_drawn: false,

            prompt_prefix: String::new(),
            prompt_prefix_len: 0,
            prompt_suffix: String::new(),
            prompt_suffix_len: 0,

            prompt_type: PromptType::Normal,

            reverse_search: false,
            search_failed: false,
            search_buffer: String::new(),
            last_search: String::new(),
            prev_history: None,
            prev_cursor: !0,

            input_arg: Digit::None,
            explicit_arg: false,

            screen_size,
        }
    }

    pub fn history(&self) -> HistoryIter {
        HistoryIter(self.history.iter())
    }

    pub fn new_history(&self) -> Skip<HistoryIter> {
        let first_new = self.history.len() - self.history_new_entries;
        self.history().skip(first_new)
    }

    pub fn new_history_entries(&self) -> usize {
        self.history_new_entries
    }

    pub fn reset_data(&mut self) {
        self.buffer.clear();
        self.backup_buffer.clear();
        self.cursor = 0;
        self.history_index = None;

        self.prompt_type = PromptType::Normal;

        self.input_arg = Digit::None;
        self.explicit_arg = false;
    }

    pub fn reset_new_history(&mut self) {
        self.history_new_entries = 0;
    }

    pub fn set_buffer(&mut self, buf: &str) {
        self.buffer.clear();
        self.buffer.push_str(buf);
        self.cursor = buf.len();
    }

    pub fn set_cursor(&mut self, pos: usize) {
        if !self.buffer.is_char_boundary(pos) {
            panic!("invalid cursor position {} in buffer {:?}",
                pos, self.buffer);
        }

        self.cursor = pos;
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        let (pre, suf) = match prompt.rfind('\n') {
            Some(pos) => (&prompt[..pos + 1], &prompt[pos + 1..]),
            None => (&prompt[..0], prompt)
        };

        self.prompt_prefix = pre.to_owned();
        self.prompt_suffix = suf.to_owned();

        let pre_virt = filter_visible(pre);
        self.prompt_prefix_len = self.display_size(&pre_virt, 0);

        let suf_virt = filter_visible(suf);
        self.prompt_suffix_len = self.display_size(&suf_virt, 0);
    }

    pub fn display_size(&self, s: &str, start_col: usize) -> usize {
        let width = self.screen_size.columns;
        let mut col = start_col;

        let disp = Display{
            allow_tab: true,
            allow_newline: true,
            .. Display::default()
        };

        for ch in s.chars().flat_map(|ch| display(ch, disp)) {
            let n = match ch {
                '\n' => width - (col % width),
                '\t' => TAB_STOP - (col % TAB_STOP),
                ch if is_combining_mark(ch) => 0,
                ch if is_wide(ch) => {
                    if col % width == width - 1 {
                        // Can't render a fullwidth character into last column
                        3
                    } else {
                        2
                    }
                }
                _ => 1
            };

            col += n;
        }

        col - start_col
    }
}

/// Maximum value of digit input
const NUMBER_MAX: i32 = 1_000_000;

#[derive(Copy, Clone, Debug)]
pub(crate) enum Digit {
    None,
    NegNone,
    Num(i32),
    NegNum(i32),
}

impl Digit {
    pub fn input(&mut self, n: i32) {
        match *self {
            Digit::None => *self = Digit::Num(n),
            Digit::NegNone => *self = Digit::NegNum(n),
            Digit::Num(ref mut m) | Digit::NegNum(ref mut m) => {
                *m *= 10;
                *m += n;
            }
        }
    }

    pub fn is_out_of_bounds(&self) -> bool {
        match *self {
            Digit::Num(n) | Digit::NegNum(n) if n > NUMBER_MAX => true,
            _ => false
        }
    }

    pub fn to_i32(&self) -> i32 {
        match *self {
            Digit::None => 1,
            Digit::NegNone => -1,
            Digit::Num(n) => n,
            Digit::NegNum(n) => -n,
        }
    }
}

impl From<char> for Digit {
    /// Convert a decimal digit character to a `Digit` value.
    ///
    /// The input must be in the range `'0' ..= '9'`.
    fn from(ch: char) -> Digit {
        let n = (ch as u8) - b'0';
        Digit::Num(n as i32)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum PromptType {
    Normal,
    Number,
    Search,
    CompleteIntro(usize),
    CompleteMore,
}

impl PromptType {
    pub(crate) fn is_normal(&self) -> bool {
        *self == PromptType::Normal
    }
}

impl<'a, 'b, Term: 'b + Terminal> Deref for WriterImpl<'a, 'b, Term> {
    type Target = WriteLock<'b, Term>;

    fn deref(&self) -> &WriteLock<'b, Term> {
        match *self {
            WriterImpl::Mutex(ref m) => m,
            WriterImpl::MutRef(ref m) => m,
        }
    }
}

impl<'a, 'b: 'a, Term: 'b + Terminal> DerefMut for WriterImpl<'a, 'b, Term> {
    fn deref_mut(&mut self) -> &mut WriteLock<'b, Term> {
        match *self {
            WriterImpl::Mutex(ref mut m) => m,
            WriterImpl::MutRef(ref mut m) => m,
        }
    }
}

/// Iterator over `Interface` history entries
pub struct HistoryIter<'a>(vec_deque::Iter<'a, String>);

impl<'a> ExactSizeIterator for HistoryIter<'a> {}

impl<'a> Iterator for HistoryIter<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.0.next().map(|s| &s[..])
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<&'a str> {
        self.0.nth(n).map(|s| &s[..])
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a> DoubleEndedIterator for HistoryIter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.0.next_back().map(|s| &s[..])
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DisplaySequence {
    Char(char),
    Escape(char),
    End,
}

impl Iterator for DisplaySequence {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        use self::DisplaySequence::*;

        let (res, next) = match *self {
            Char(ch) => (ch, End),
            Escape(ch) => ('^', Char(ch)),
            End => return None
        };

        *self = next;
        Some(res)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        use self::DisplaySequence::*;

        let n = match *self {
            Char(_) => 1,
            Escape(_) => 2,
            End => 0,
        };

        (n, Some(n))
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Display {
    allow_tab: bool,
    allow_newline: bool,
    allow_escape: bool,
}

pub(crate) fn display(ch: char, style: Display) -> DisplaySequence {
    match ch {
        '\t' if style.allow_tab => DisplaySequence::Char(ch),
        '\n' if style.allow_newline => DisplaySequence::Char(ch),
        ESCAPE if style.allow_escape => DisplaySequence::Char(ch),
        '\0' => DisplaySequence::Escape('@'),
        RUBOUT => DisplaySequence::Escape('?'),
        ch if is_ctrl(ch) => DisplaySequence::Escape(unctrl(ch)),
        ch => DisplaySequence::Char(ch)
    }
}

pub(crate) fn display_str<'a>(s: &'a str, style: Display) -> Cow<'a, str> {
    if s.chars().all(|ch| display(ch, style) == DisplaySequence::Char(ch)) {
        Borrowed(s)
    } else {
        Owned(s.chars().flat_map(|ch| display(ch, style)).collect())
    }
}

fn complete_intro(n: usize) -> String {
    format!("Display all {} possibilities? (y/n)", n)
}

fn number_len(n: i32) -> usize {
    match n {
        -1_000_000              => 8,
        -  999_999 ..= -100_000 => 7,
        -   99_999 ..= - 10_000 => 6,
        -    9_999 ..= -  1_000 => 5,
        -      999 ..= -    100 => 4,
        -       99 ..= -     10 => 3,
        -        9 ..= -      1 => 2,
                 0 ..=        9 => 1,
                10 ..=       99 => 2,
               100 ..=      999 => 3,
             1_000 ..=    9_999 => 4,
            10_000 ..=   99_999 => 5,
           100_000 ..=  999_999 => 6,
         1_000_000              => 7,
        _ => unreachable!()
    }
}
