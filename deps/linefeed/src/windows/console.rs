use std::char;
use std::io;
use std::mem::zeroed;
use std::os::raw::c_int;
use std::time::Duration;

use mortal::{Event, TerminalReadGuard};
use mortal::windows::TerminalExt;

use winapi::shared::minwindef::{
    DWORD,
    TRUE,
};
use winapi::um::wincon::{
    self,
    INPUT_RECORD,
    KEY_EVENT,
    KEY_EVENT_RECORD,
};
use winapi::um::winuser;

use crate::chars::DELETE;
use crate::terminal::RawRead;

// Generate some sequences for special characters.
// The basic ones align with common Unix terminals, so that they match up with
// default bindings. Ctrl/Shift/Alt combinations for arrow keys are somewhat
// arbitrary, as Unix terminals can't seem to agree on those.
const HOME_SEQ: &str = "\x1b[H";
const END_SEQ: &str = "\x1b[F";
const INSERT_SEQ: &str = "\x1b[2~";
const DELETE_SEQ: &str = "\x1b[3~";
const PAGE_UP_SEQ: &str = "\x1b[5~";
const PAGE_DOWN_SEQ: &str = "\x1b[6~";

struct SeqGroup {
    norm: &'static str,
    ctrl: &'static str,
    shift: &'static str,
    alt: &'static str,
    ctrl_shift: &'static str,
    ctrl_alt: &'static str,
    shift_alt: &'static str,
    ctrl_shift_alt: &'static str,
}

impl SeqGroup {
    fn select(&self, state: DWORD) -> &'static str {
        match (has_ctrl(state), has_shift(state), has_alt(state)) {
            (false, false, false) => self.norm,
            (true,  false, false) => self.ctrl,
            (false, true,  false) => self.shift,
            (true,  true,  false) => self.ctrl_shift,
            (false, false, true)  => self.alt,
            (true,  false, true)  => self.ctrl_alt,
            (false, true,  true)  => self.shift_alt,
            (true,  true,  true)  => self.ctrl_shift_alt,
        }
    }
}

macro_rules! seq_group {
    ( $name:ident , $ch:expr ) => {
        const $name: SeqGroup = SeqGroup {
            norm:           concat!("\x1b[",  $ch),
            ctrl:           concat!("\x1b[1", $ch),
            shift:          concat!("\x1b[2", $ch),
            alt:            concat!("\x1b[4", $ch),
            ctrl_shift:     concat!("\x1b[3", $ch),
            ctrl_alt:       concat!("\x1b[5", $ch),
            shift_alt:      concat!("\x1b[6", $ch),
            ctrl_shift_alt: concat!("\x1b[7", $ch),
        };
    }
}

seq_group!{ UP_SEQ, "A" }
seq_group!{ DOWN_SEQ, "B" }
seq_group!{ RIGHT_SEQ, "C" }
seq_group!{ LEFT_SEQ, "D" }

pub fn terminal_read(term: &mut TerminalReadGuard, buf: &mut Vec<u8>) -> io::Result<RawRead> {
    let mut events: [INPUT_RECORD; 1] = unsafe { zeroed() };

    let n = match term.read_raw_event(&mut events, Some(Duration::new(0, 0)))? {
        Some(Event::Raw(n)) => n,
        None => return Ok(RawRead::Bytes(0)),
        Some(Event::Resize(size)) => return Ok(RawRead::Resize(size)),
        Some(Event::Signal(sig)) => return Ok(RawRead::Signal(sig)),
        _ => unreachable!()
    };

    if n == 1 {
        let old_len = buf.len();

        translate_event(buf, &events[0]);

        Ok(RawRead::Bytes(buf.len() - old_len))
    } else {
        Ok(RawRead::Bytes(0))
    }
}

fn translate_event(buf: &mut Vec<u8>, event: &INPUT_RECORD) {
    if event.EventType == KEY_EVENT {
        translate_key(buf, unsafe { event.Event.KeyEvent() });
    }
}

fn translate_key(buf: &mut Vec<u8>, event: &KEY_EVENT_RECORD) {
    if event.bKeyDown == TRUE {
        let start = buf.len();

        match event.wVirtualKeyCode as c_int {
            winuser::VK_BACK    => buf.push(DELETE as u8),
            winuser::VK_TAB     => buf.push(b'\t'),
            winuser::VK_RETURN  => {
                buf.push(b'\r');
            }
            winuser::VK_ESCAPE  => buf.push(b'\x1b'),
            // Page up
            winuser::VK_PRIOR   => buf.extend(PAGE_UP_SEQ.as_bytes()),
            // Page down
            winuser::VK_NEXT    => buf.extend(PAGE_DOWN_SEQ.as_bytes()),
            winuser::VK_END     => buf.extend(END_SEQ.as_bytes()),
            winuser::VK_HOME    => buf.extend(HOME_SEQ.as_bytes()),
            winuser::VK_LEFT    => {
                buf.extend(LEFT_SEQ.select(event.dwControlKeyState).as_bytes());
            }
            winuser::VK_UP      => {
                buf.extend(UP_SEQ.select(event.dwControlKeyState).as_bytes());
            }
            winuser::VK_RIGHT   => {
                buf.extend(RIGHT_SEQ.select(event.dwControlKeyState).as_bytes());
            }
            winuser::VK_DOWN    => {
                buf.extend(DOWN_SEQ.select(event.dwControlKeyState).as_bytes());
            }
            winuser::VK_INSERT  => buf.extend(INSERT_SEQ.as_bytes()),
            winuser::VK_DELETE  => buf.extend(DELETE_SEQ.as_bytes()),
            _ => {
                let u_ch = unsafe { *event.uChar.UnicodeChar() };
                if u_ch != 0 {
                    if let Some(ch) = char::from_u32(u_ch as u32) {
                        let mut bytes = [0; 4];
                        buf.extend(ch.encode_utf8(&mut bytes).as_bytes());
                    }
                }
            }
        }

        if event.wRepeatCount > 1 {
            let seq = buf[start..].to_owned();

            for _ in 1..event.wRepeatCount {
                buf.extend(&seq);
            }
        }
    }
}

fn has_alt(state: DWORD) -> bool {
    state & (wincon::LEFT_ALT_PRESSED | wincon::RIGHT_ALT_PRESSED) != 0
}

fn has_ctrl(state: DWORD) -> bool {
    state & (wincon::LEFT_CTRL_PRESSED | wincon::RIGHT_CTRL_PRESSED) != 0
}

fn has_shift(state: DWORD) -> bool {
    state & wincon::SHIFT_PRESSED != 0
}
