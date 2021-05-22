use std::fs::File;
use std::io::Write;
use std::os::unix::io::{FromRawFd, RawFd};

use crate::tools;
use crate::tools::clog;
use crate::types::{Command, CommandLine, Redirection};

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
                let (_fd_out, _fd_err) = _get_std_fds(&redirects[i+1..]);
                if let Some(fd) = _fd_err {
                    _fd_candidate = Some(fd);
                } else {
                    _fd_candidate = unsafe { Some(libc::dup(2)) };
                }
            } else {  // 1> foo.log
                let append = item.1 == ">>";
                if let Ok(fd) = tools::create_raw_fd_from_file(&item.2, append) {
                    _fd_candidate = Some(fd);
                }
            }

            // for command like this: `alias > a.txt > b.txt > c.txt`,
            // we need to return the last one, but close the previous two.
            if let Some(fd) = fd_out {
                unsafe { libc::close(fd); }
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
            } else {  // 2>foo.log
                let append = item.1 == ">>";
                if let Ok(fd) = tools::create_raw_fd_from_file(&item.2, append) {
                    _fd_candidate = Some(fd);
                }
            }

            if let Some(fd) = fd_err {
                unsafe { libc::close(fd); }
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
        unsafe { libc::close(fd); }
    }
    if let Some(fd) = _fd_out {
        fd
    } else {
        unsafe { libc::dup(1) }
    }
}

fn _get_dupped_stderr_fd(cmd: &Command, cl: &CommandLine) -> RawFd {
    if cl.with_pipeline() {
        return 2;
    }

    let (_fd_out, _fd_err) = _get_std_fds(&cmd.redirects_to);
    if let Some(fd) = _fd_out {
        unsafe { libc::close(fd); }
    }

    if let Some(fd) = _fd_err {
        fd
    } else {
        unsafe { libc::dup(2) }
    }
}

pub fn print_stdout(info: &str, cmd: &Command, cl: &CommandLine) {
    let fd = _get_dupped_stdout_fd(cmd, cl);
    log!("created stdout fd: {:?}", fd);

    unsafe {
        let mut f = File::from_raw_fd(fd);
        f.write_all(info.as_bytes()).unwrap();
        if !info.is_empty() {
            f.write_all(b"\n").unwrap();
        }
    }
}

pub fn print_stderr(info: &str, cmd: &Command, cl: &CommandLine) {
    let fd = _get_dupped_stderr_fd(cmd, cl);
    log!("created stderr fd: {}", fd);

    unsafe {
        let mut f = File::from_raw_fd(fd);
        f.write_all(info.as_bytes()).unwrap();
        if !info.is_empty() {
            f.write_all(b"\n").unwrap();
        }
    }
}
