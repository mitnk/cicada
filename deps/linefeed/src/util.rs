//! Provides miscellaneous utilities

use std::borrow::Cow;
use std::io;
use std::ops::{Range, RangeFrom, RangeFull, RangeTo};
use std::str::{from_utf8, from_utf8_unchecked};

pub fn filter_visible(s: &str) -> Cow<str> {
    use crate::reader::{START_INVISIBLE, END_INVISIBLE};

    if !s.contains(START_INVISIBLE) {
        return Cow::Borrowed(s);
    }

    let mut virt = String::new();
    let mut ignore = false;

    for ch in s.chars() {
        if ch == START_INVISIBLE {
            ignore = true;
        } else if ch == END_INVISIBLE {
            ignore = false;
        } else if !ignore {
            virt.push(ch);
        }
    }

    Cow::Owned(virt)
}

/// Returns the longest common prefix of a set of strings.
///
/// If no common prefix exists, `None` is returned.
pub fn longest_common_prefix<'a, I, S>(iter: I) -> Option<&'a str> where
        I: IntoIterator<Item=&'a S>,
        S: 'a + ?Sized + AsRef<str>,
        {
    let mut iter = iter.into_iter();

    let mut pfx = iter.next()?.as_ref();

    for s in iter {
        let s = s.as_ref();

        let n = pfx.chars().zip(s.chars())
            .take_while(|&(a, b)| a == b)
            .map(|(ch, _)| ch.len_utf8()).sum();

        if n == 0 {
            return None;
        } else {
            pfx = &pfx[..n];
        }
    }

    Some(pfx)
}

/// Returns a string consisting of a `char`, repeated `n` times.
pub fn repeat_char(ch: char, n: usize) -> String {
    let mut buf = [0; 4];
    let s = ch.encode_utf8(&mut buf);

    s.repeat(n)
}

/// Implemented for built-in range types
// Waiting for stabilization of `std` trait of the same name
pub trait RangeArgument<T> {
    /// Returns the start of range, if present.
    fn start(&self) -> Option<&T> { None }

    /// Returns the end of range, if present.
    fn end(&self) -> Option<&T> { None }
}

impl<T> RangeArgument<T> for Range<T> {
    fn start(&self) -> Option<&T> { Some(&self.start) }

    fn end(&self) -> Option<&T> { Some(&self.end) }
}

impl<T> RangeArgument<T> for RangeFrom<T> {
    fn start(&self) -> Option<&T> { Some(&self.start) }
}

impl<T> RangeArgument<T> for RangeTo<T> {
    fn end(&self) -> Option<&T> { Some(&self.end) }
}

impl<T> RangeArgument<T> for RangeFull {}

pub fn backward_char(n: usize, s: &str, cur: usize) -> usize {
    let mut chars = s[..cur].char_indices()
        .filter(|&(_, ch)| !is_combining_mark(ch));
    let mut res = cur;

    for _ in 0..n {
        match chars.next_back() {
            Some((idx, _)) => res = idx,
            None => return 0
        }
    }

    res
}

pub fn forward_char(n: usize, s: &str, cur: usize) -> usize {
    let mut chars = s[cur..].char_indices()
        .filter(|&(_, ch)| !is_combining_mark(ch));

    for _ in 0..n {
        match chars.next() {
            Some(_) => (),
            None => return s.len()
        }
    }

    match chars.next() {
        Some((idx, _)) => cur + idx,
        None => s.len()
    }
}

pub fn backward_search_char(n: usize, buf: &str, mut cur: usize, ch: char) -> Option<usize> {
    let mut pos = None;

    for _ in 0..n {
        match buf[..cur].rfind(ch) {
            Some(p) => {
                cur = p;
                pos = Some(cur);
            }
            None => break
        }
    }

    pos
}

pub fn forward_search_char(n: usize, buf: &str, mut cur: usize, ch: char) -> Option<usize> {
    let mut pos = None;

    for _ in 0..n {
        // Skip past the character under the cursor
        let off = match buf[cur..].chars().next() {
            Some(ch) => ch.len_utf8(),
            None => break
        };

        match buf[cur + off..].find(ch) {
            Some(p) => {
                cur += off + p;
                pos = Some(cur);
            }
            None => break
        }
    }

    pos
}

pub fn backward_word(n: usize, buf: &str, cur: usize, word_break: &str) -> usize {
    let mut chars = buf[..cur].char_indices().rev();

    for _ in 0..n {
        drop_while(&mut chars, |(_, ch)| word_break.contains(ch));
        if chars.clone().next().is_none() { break; }
        drop_while(&mut chars, |(_, ch)| !word_break.contains(ch));
        if chars.clone().next().is_none() { break; }
    }

    match chars.next() {
        Some((ind, ch)) => ind + ch.len_utf8(),
        None => 0
    }
}

pub fn forward_word(n: usize, buf: &str, cur: usize, word_break: &str) -> usize {
    let mut chars = buf[cur..].char_indices();

    for _ in 0..n {
        drop_while(&mut chars, |(_, ch)| word_break.contains(ch));
        if chars.clone().next().is_none() { break; }
        drop_while(&mut chars, |(_, ch)| !word_break.contains(ch));
        if chars.clone().next().is_none() { break; }
    }

    match chars.next() {
        Some((ind, _)) => cur + ind,
        None => buf.len()
    }
}

pub fn back_n_words(n: usize, buf: &str, cur: usize, word_break: &str) -> Range<usize> {
    let prev = backward_word(1, buf, cur, word_break);
    let end = word_end(&buf, prev, word_break);

    if n > 1 {
        let start = backward_word(n - 1, buf, prev, word_break);
        start..end
    } else {
        prev..end
    }
}

pub fn forward_n_words(n: usize, buf: &str, cur: usize, word_break: &str) -> Range<usize> {
    let start = next_word(1, buf, cur, word_break);

    if n > 1 {
        let last = next_word(n - 1, buf, start, word_break);
        let end = word_end(buf, last, word_break);
        start..end
    } else {
        let end = word_end(buf, start, word_break);
        start..end
    }
}

/// Returns the first character in the buffer, if it contains any valid characters.
pub fn first_char(buf: &[u8]) -> io::Result<Option<char>> {
    match from_utf8(buf) {
        Ok(s) => Ok(s.chars().next()),
        Err(e) => {
            if e.error_len().is_some() {
                return Err(io::Error::new(io::ErrorKind::InvalidData,
                    "invalid utf-8 input received"));
            }

            let valid = e.valid_up_to();

            let s = unsafe { from_utf8_unchecked(&buf[..valid]) };
            Ok(s.chars().next())
        }
    }
}

pub fn first_word(buf: &str, word_break: &str) -> Option<usize> {
    let mut chars = buf.char_indices();

    drop_while(&mut chars, |(_, ch)| word_break.contains(ch));

    chars.next().map(|(idx, _)| idx)
}

pub fn word_start(buf: &str, cur: usize, word_break: &str) -> usize {
    let fwd = match buf[cur..].chars().next() {
        Some(ch) => word_break.contains(ch),
        None => return buf.len()
    };

    if fwd {
        next_word(1, buf, cur, word_break)
    } else {
        let mut chars = buf[..cur].char_indices().rev();

        drop_while(&mut chars, |(_, ch)| !word_break.contains(ch));

        match chars.next() {
            Some((idx, ch)) => idx + ch.len_utf8(),
            None => 0
        }
    }
}

pub fn next_word(n: usize, buf: &str, cur: usize, word_break: &str) -> usize {
    let mut chars = buf[cur..].char_indices();

    for _ in 0..n {
        drop_while(&mut chars, |(_, ch)| !word_break.contains(ch));
        if chars.clone().next().is_none() { break; }
        drop_while(&mut chars, |(_, ch)| word_break.contains(ch));
        if chars.clone().next().is_none() { break; }
    }

    match chars.next() {
        Some((idx, _)) => cur + idx,
        None => buf.len()
    }
}

pub fn word_end(buf: &str, cur: usize, word_break: &str) -> usize {
    let mut chars = buf[cur..].char_indices();

    drop_while(&mut chars, |(_, ch)| !word_break.contains(ch));

    match chars.next() {
        Some((idx, _)) => cur + idx,
        None => buf.len()
    }
}

pub fn drop_while<I, T, F>(iter: &mut I, mut f: F)
        where I: Iterator<Item=T> + Clone, F: FnMut(T) -> bool {
    loop {
        let mut clone = iter.clone();

        match clone.next() {
            None => break,
            Some(t) => {
                if f(t) {
                    *iter = clone;
                } else {
                    break;
                }
            }
        }
    }
}

pub fn get_open_paren(ch: char) -> Option<char> {
    match ch {
        ')' => Some('('),
        ']' => Some('['),
        '}' => Some('{'),
        _ => None
    }
}

pub fn find_matching_paren(s: &str, quotes: &str, open: char, close: char) -> Option<usize> {
    let mut chars = s.char_indices().rev();
    let mut level = 0;
    let mut string_delim = None;

    while let Some((ind, ch)) = chars.next() {
        if string_delim == Some(ch) {
            string_delim = None;
        } else if quotes.contains(ch) {
            string_delim = Some(ch);
        } else if string_delim.is_none() && ch == close {
            level += 1;
        } else if string_delim.is_none() && ch == open {
            level -= 1;

            if level == 0 {
                return Some(ind);
            }
        }
    }

    None
}

pub fn is_combining_mark(ch: char) -> bool {
    use mortal::util::is_combining_mark;

    is_combining_mark(ch)
}

pub fn is_wide(ch: char) -> bool {
    use mortal::util::char_width;

    char_width(ch) == Some(2)
}

pub fn match_name(name: &str, value: &str) -> bool {
    // A value of "foo" matches both "foo" and "foo-bar"
    name == value ||
        (name.starts_with(value) && name.as_bytes()[value.len()] == b'-')
}

#[cfg(test)]
mod test {
    use super::{
        longest_common_prefix,
        match_name,
    };

    #[test]
    fn test_longest_common_prefix() {
        let empty: &[&str] = &[];

        assert_eq!(longest_common_prefix(empty.iter()),
            None);
        assert_eq!(longest_common_prefix(["foo", "bar"].iter()),
            None);
        assert_eq!(longest_common_prefix(["foo"].iter()),
            Some("foo"));
        assert_eq!(longest_common_prefix(["foo", "foobar"].iter()),
            Some("foo"));
        assert_eq!(longest_common_prefix(["foobar", "foo"].iter()),
            Some("foo"));
        assert_eq!(longest_common_prefix(["alpha", "alpaca", "alto"].iter()),
            Some("al"));

        assert_eq!(longest_common_prefix(["äöüx", "äöüy"].iter()),
            Some("äöü"));
    }

    #[test]
    fn test_match_name() {
        assert!(match_name("foo", "foo"));
        assert!(match_name("foo-", "foo"));
        assert!(match_name("foo-bar", "foo"));
        assert!(match_name("foo-bar-baz", "foo-bar"));

        assert!(!match_name("foo", "bar"));
        assert!(!match_name("foo", "foo-"));
        assert!(!match_name("foo", "foo-bar"));
    }
}
