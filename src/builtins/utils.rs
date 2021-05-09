use std::fs::File;
use std::io::Write;
use std::os::unix::io::{FromRawFd, RawFd};

use crate::tools;
use crate::types::{Command, CommandLine};

fn _get_dupped_stderr_fd(cmd: &Command, cl: &CommandLine) -> RawFd {
    if cl.with_pipeline() {
        return 2;
    }

    if cmd.redirects_to.is_empty() {
        let _fd = unsafe { libc::dup(2) };
        return _fd;
    }

    let mut handled = false;

    let mut fd = 2;
    for item in &cmd.redirects_to {
        if item.0 != "2" {
            continue;
        }

        let append = item.1 == ">>";
        if &item.2 == "&1" {
            let _fd = _get_dupped_stdout_fd(cmd, cl);
            fd = _fd;
            handled = true;
        } else {
            if let Ok(_fd) = tools::create_raw_fd_from_file(&item.2, append) {
                fd = _fd;
                handled = true;
            }
        }
    }

    if handled {
        return fd;
    } else {
        let _fd = unsafe { libc::dup(2) };
        return _fd;
    }
}

fn _get_dupped_stdout_fd(cmd: &Command, cl: &CommandLine) -> RawFd {
    if cl.with_pipeline() {
        return 1;
    }

    if cmd.redirects_to.is_empty() {
        let _fd = unsafe { libc::dup(1) };
        return _fd;
    }

    let mut handled = false;

    let mut fd = 1;
    for item in &cmd.redirects_to {
        if item.0 != "1" {
            continue;
        }

        let append = item.1 == ">>";
        if &item.2 == "&2" {
            let _fd = _get_dupped_stderr_fd(cmd, cl);
            fd = _fd;
            handled = true;
        } else {
            if let Ok(_fd) = tools::create_raw_fd_from_file(&item.2, append) {
                fd = _fd;
                handled = true;
            }
        }
    }

    if handled {
        return fd;
    } else {
        let _fd = unsafe { libc::dup(1) };
        return _fd;
    }
}

pub fn print_stdout(info: &str, cmd: &Command, cl: &CommandLine) {
    let fd = _get_dupped_stdout_fd(cmd, cl);
    // log!("created stdout fd: {}", fd);

    unsafe {
        let mut f = File::from_raw_fd(fd);
        f.write_all(info.as_bytes()).unwrap();
        f.write_all(b"\n").unwrap();
    }
}

pub fn print_stderr(info: &str, cmd: &Command, cl: &CommandLine) {
    let fd = _get_dupped_stderr_fd(cmd, cl);
    // log!("created stderr fd: {}", fd);

    unsafe {
        let mut f = File::from_raw_fd(fd);
        f.write_all(info.as_bytes()).unwrap();
        f.write_all(b"\n").unwrap();
    }
}
