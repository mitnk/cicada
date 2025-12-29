use std::mem;

/// Returns `(columns, rows)` for the current terminal if stdout is a TTY.
pub fn dimensions() -> Option<(usize, usize)> {
    unsafe {
        let fd = libc::STDOUT_FILENO;
        let is_tty = libc::isatty(fd) == 1;
        if !is_tty {
            log!("Error: stdout is not a TTY.");
            return None;
        }

        let mut ws: libc::winsize = mem::zeroed();

        if libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) != 0 {
            return None;
        }

        let cols = ws.ws_col as usize;
        let rows = ws.ws_row as usize;
        if cols > 0 && rows > 0 {
            Some((cols, rows))
        } else {
            None
        }
    }
}
