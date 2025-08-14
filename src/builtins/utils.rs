use std::fs::File;
use std::io::Write;
use std::os::unix::io::{FromRawFd, RawFd};

use errno::errno;

use crate::tools;
use crate::types::{Command, CommandLine, CommandResult, Redirection};

/// Helper function to get (stdout, stderr) pairs for redirections,
/// e.g. `alias foo 1>/dev/null 2>&1 > foo.txt`
/// (i.e. [
///      ("1", ">", "/dev/null"),
///      ("2", ">", "&1"),
///      ("1", ">", "foo.txt"),
///  ])
fn _get_std_fds(redirects: &[Redirection]) -> (Option<RawFd>, Option<RawFd>) {
    if redirects.is_empty() {
        return (None, None);
    }

    let mut fd_out = None;
    let mut fd_err = None;

    for i in 0..redirects.len() {
        let item = &redirects[i];
        if item.0 == "1" {
            // 1>&2
            let mut _fd_candidate = None;

            if item.2 == "&2" {
                let (_fd_out, _fd_err) = _get_std_fds(&redirects[i + 1..]);
                if let Some(fd) = _fd_err {
                    _fd_candidate = Some(fd);
                } else {
                    _fd_candidate = unsafe { Some(libc::dup(2)) };
                }
            } else {
                // 1> foo.log
                let append = item.1 == ">>";
                if let Ok(fd) = tools::create_raw_fd_from_file(&item.2, append) {
                    _fd_candidate = Some(fd);
                }
            }

            // for command like this: `alias > a.txt > b.txt > c.txt`,
            // we need to return the last one, but close the previous two.
            if let Some(fd) = fd_out {
                unsafe {
                    libc::close(fd);
                }
            }

            fd_out = _fd_candidate;
        }

        if item.0 == "2" {
            // 2>&1
            let mut _fd_candidate = None;

            if item.2 == "&1" {
                if let Some(fd) = fd_out {
                    _fd_candidate = unsafe { Some(libc::dup(fd)) };
                }
            } else {
                // 2>foo.log
                let append = item.1 == ">>";
                if let Ok(fd) = tools::create_raw_fd_from_file(&item.2, append) {
                    _fd_candidate = Some(fd);
                }
            }

            if let Some(fd) = fd_err {
                unsafe {
                    libc::close(fd);
                }
            }

            fd_err = _fd_candidate;
        }
    }

    (fd_out, fd_err)
}

fn _get_dupped_stdout_fd(cmd: &Command, cl: &CommandLine) -> RawFd {
    // if with pipeline, e.g. `history | grep foo`, then we don't need to
    // dup stdout since it is running in a sperated process, whose fd can
    // be dropped after use.
    if cl.with_pipeline() {
        return 1;
    }

    let (_fd_out, _fd_err) = _get_std_fds(&cmd.redirects_to);
    if let Some(fd) = _fd_err {
        unsafe {
            libc::close(fd);
        }
    }
    if let Some(fd) = _fd_out {
        fd
    } else {
        let fd = unsafe { libc::dup(1) };
        if fd == -1 {
            let eno = errno();
            println_stderr!("cicada: dup: {}", eno);
        }
        fd
    }
}

fn _get_dupped_stderr_fd(cmd: &Command, cl: &CommandLine) -> RawFd {
    if cl.with_pipeline() {
        return 2;
    }

    let (_fd_out, _fd_err) = _get_std_fds(&cmd.redirects_to);
    if let Some(fd) = _fd_out {
        unsafe {
            libc::close(fd);
        }
    }

    if let Some(fd) = _fd_err {
        fd
    } else {
        let fd = unsafe { libc::dup(2) };
        if fd == -1 {
            let eno = errno();
            println_stderr!("cicada: dup: {}", eno);
        }
        fd
    }
}

pub fn print_stdout(info: &str, cmd: &Command, cl: &CommandLine) {
    let fd = _get_dupped_stdout_fd(cmd, cl);
    if fd == -1 {
        return;
    }

    unsafe {
        let mut f = File::from_raw_fd(fd);
        let info = info.trim_end_matches('\n');
        match f.write_all(info.as_bytes()) {
            Ok(_) => {}
            Err(e) => {
                println_stderr!("write_all: error: {}", e);
            }
        }
        if !info.is_empty() {
            match f.write_all(b"\n") {
                Ok(_) => {}
                Err(e) => {
                    println_stderr!("write_all: error: {}", e);
                }
            }
        }
    }
}

pub fn print_stderr(info: &str, cmd: &Command, cl: &CommandLine) {
    let fd = _get_dupped_stderr_fd(cmd, cl);
    if fd == -1 {
        return;
    }

    unsafe {
        let mut f = File::from_raw_fd(fd);
        let info = info.trim_end_matches('\n');
        match f.write_all(info.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                println_stderr!("write_all: error: {}", e);
            }
        }

        if !info.is_empty() {
            match f.write_all(b"\n") {
                Ok(_) => (),
                Err(e) => {
                    println_stderr!("write_all: error: {}", e);
                }
            }
        }
    }
}

pub fn print_stderr_with_capture(
    info: &str,
    cr: &mut CommandResult,
    cl: &CommandLine,
    cmd: &Command,
    capture: bool,
) {
    cr.status = 1;
    if capture {
        cr.stderr = info.to_string();
    } else {
        print_stderr(info, cmd, cl);
    }
}

pub fn print_stdout_with_capture(
    info: &str,
    cr: &mut CommandResult,
    cl: &CommandLine,
    cmd: &Command,
    capture: bool,
) {
    cr.status = 0;
    if capture {
        cr.stdout = info.to_string();
    } else {
        print_stdout(info, cmd, cl);
    }
}
