use clap::{Arg, App, ErrorKind};
use libc;

use std::io::{Error, Write};

use crate::builtins::utils::print_stderr_with_capture;
use crate::builtins::utils::print_stdout_with_capture;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(_sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);
    // NOTE: these default_missing_value -1 is for reporting only
    // we cannot change the limit less then zero.
    let mut app = App::new("ulimit")
        .about("Show / Modify shell resource limits")
        .arg(Arg::new("report_all")
             .short('a')
             .help("Report all limits"))
        .arg(Arg::new("for_hard")
             .short('H')
             .help("specify the hard limit"))
        .arg(Arg::new("for_soft")
             .short('S')
             .help("specify the soft limit (default)"))
        .arg(Arg::new("open_files")
             .short('n')
             .default_missing_value("-1")
             .takes_value(true))
        .arg(Arg::new("core_file_size")
             .short('c')
             .default_missing_value("-1")
             .takes_value(true));

    if tokens.len() == 2 && (tokens[1].1 == "-h" || tokens[1].1 == "--help") {
        use std::io;
        let mut out = io::stdout();
        match app.write_help(&mut out) {
            Ok(_) => {},
            Err(e) => {
                println_stderr!("cicada: clap: {}", e);
            }
        }
        print!("\n");
        return CommandResult::new();
    }

    let matches;
    match app.try_get_matches_from(&args) {
        Ok(x) => {
            matches = x;
        }
        Err(e) => {
            let info = format!("ulimit error: {}", e);
            print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
            return cr;
        }
    }

    let open_files;
    match matches.value_of_t("open_files") {
        Ok(x) => open_files = x,
        Err(e) => {
            match e.kind {
                ErrorKind::ArgumentNotFound => open_files = -1,
                _ => {
                    let info = format!("cicada: ulimit: invalid params: {}", e);
                    print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                    return cr;
                }
            }
        }
    }

    let core_file_size;
    match matches.value_of_t("core_file_size") {
        Ok(x) => core_file_size = x,
        Err(e) => {
            match e.kind {
                ErrorKind::ArgumentNotFound => core_file_size = -1,
                _ => {
                    let info = format!("cicada: ulimit: invalid params: {}", e);
                    print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                    return cr;
                }
            }
        }
    }

    let mut options = Vec::new();
    for (_, o) in tokens {
        if o.starts_with('-') && (o != "-H" && o != "-S" && o != "-a") {
            options.push(o);
        }
    }

    let for_hard = matches.is_present("for_hard");
    if matches.is_present("report_all") || options.len() == 0 {
        let (_out, _err) = report_all(for_hard);
        if !_out.is_empty() {
            print_stdout_with_capture(&_out, &mut cr, cl, cmd, capture);
        }
        if !_err.is_empty() {
            print_stderr_with_capture(&_err, &mut cr, cl, cmd, capture);
        }
        return cr;
    }

    if open_files > -1 {
        let _err = set_limit("open_files", open_files as u64, for_hard);
        if !_err.is_empty() {
            print_stderr_with_capture(&_err, &mut cr, cl, cmd, capture);
            return cr;
        }
    }
    if core_file_size > -1 {
        let _err = set_limit("core_file_size", core_file_size as u64, for_hard);
        if !_err.is_empty() {
            print_stderr_with_capture(&_err, &mut cr, cl, cmd, capture);
            return cr;
        }
    }

    let (_out, _err) = report_needed(&options, for_hard, open_files, core_file_size);
    if !_out.is_empty() {
        print_stdout_with_capture(&_out, &mut cr, cl, cmd, capture);
    }
    if !_err.is_empty() {
        print_stderr_with_capture(&_err, &mut cr, cl, cmd, capture);
    }
    cr
}

fn set_limit(limit_name: &str, value: u64, for_hard: bool) -> String {
    // Since libc::RLIMIT_NOFILE etc has different types on different OS
    // so we cannot pass them via params, see issue:
    // https://github.com/rust-lang/libc/issues/2029
    let limit_id;
    if limit_name == "open_files" {
        limit_id = libc::RLIMIT_NOFILE
    } else if limit_name == "core_file_size" {
        limit_id = libc::RLIMIT_CORE
    } else {
        return String::from("invalid limit name");
    }

    let mut rlp = libc::rlimit {rlim_cur: 0, rlim_max: 0};
    let rlim: *mut libc::rlimit = &mut rlp;
    unsafe {
        let res = libc::getrlimit(limit_id, rlim);
        if res != 0 {
            let info = format!("cicada: ulimit: error when getting limit: {}",
                               Error::last_os_error());
            return String::from(&info);
        }
    }

    if for_hard {
        #[cfg(target_pointer_width = "32")]
        { rlp.rlim_max = value as u32; }
        #[cfg(target_pointer_width = "64")]
        { rlp.rlim_max = value; }
    } else {
        #[cfg(target_pointer_width = "32")]
        { rlp.rlim_cur = value as u32; }
        #[cfg(target_pointer_width = "64")]
        { rlp.rlim_cur = value; }
    }

    unsafe {
        let res = libc::setrlimit(limit_id, rlim);
        if res != 0 {
            let info = format!("cicada: ulimit: error when setting limit: {}",
                               Error::last_os_error());
            return String::from(&info);
        }
    }

    String::new()
}

fn get_limit(limit_name: &str, single_print: bool,
             for_hard: bool) -> (String, String) {
    let mut result_stderr = String::new();
    let mut result_stdout = String::new();

    let desc;
    let limit_id;
    if limit_name == "open_files" {
        desc = "open files";
        limit_id = libc::RLIMIT_NOFILE;
    } else if limit_name == "core_file_size" {
        desc = "core file size";
        limit_id = libc::RLIMIT_CORE;
    } else {
        let info = "ulimit: error: invalid limit name";
        result_stderr.push_str(&info);
        return (result_stdout, result_stderr);
    }

    let mut rlp = libc::rlimit {rlim_cur: 0, rlim_max: 0};
    let r: *mut libc::rlimit = &mut rlp;
    unsafe {
        let res = libc::getrlimit(limit_id, r);
        if res != 0 {
            let info = format!("error when getting limit: {}", Error::last_os_error());
            result_stderr.push_str(&info);
            return (result_stdout, result_stderr);
        }

        let to_print;
        if for_hard {
            to_print = rlp.rlim_max;
        } else {
            to_print = rlp.rlim_cur;
        }

        if single_print {
            if to_print == libc::RLIM_INFINITY {
                result_stdout.push_str("unlimited\n");
            } else {
                let info = format!("{}\n", to_print);
                result_stdout.push_str(&info);
            }
        } else {
            if to_print == libc::RLIM_INFINITY {
                let info = format!("{}\t\tunlimited\n", desc);
                result_stdout.push_str(&info);
            } else {
                let info = format!("{}\t\t{}\n", desc, to_print);
                result_stdout.push_str(&info);
            }
        }
    }

    (result_stdout, result_stderr)
}

fn report_all(for_hard: bool) -> (String, String) {
    let mut result_stderr = String::new();
    let mut result_stdout = String::new();

    let (_out, _err) = get_limit("open_files", false, for_hard);
    if !_out.is_empty() {
        result_stdout.push_str(&_out);
    }
    if !_err.is_empty() {
        result_stderr.push_str(&_err);
    }
    let (_out, _err) = get_limit("core_file_size", false, for_hard);
    if !_out.is_empty() {
        result_stdout.push_str(&_out);
    }
    if !_err.is_empty() {
        result_stderr.push_str(&_err);
    }

    (result_stdout, result_stderr)
}

fn report_needed(options: &Vec<&String>, for_hard: bool, open_files: i64,
                 core_file_size: i64) -> (String, String) {
    let mut result_stderr = String::new();
    let mut result_stdout = String::new();

    let single_print = options.len() == 1;
    for o in options {
        if *o == "-n" && open_files == -1 {
            let (_out, _err) = get_limit("open_files", single_print, for_hard);
            if !_out.is_empty() {
                result_stdout.push_str(&_out);
            }
            if !_err.is_empty() {
                result_stderr.push_str(&_err);
            }
        }
        if *o == "-c" && core_file_size == -1 {
            let (_out, _err) = get_limit("core_file_size", single_print, for_hard);
            if !_out.is_empty() {
                result_stdout.push_str(&_out);
            }
            if !_err.is_empty() {
                result_stderr.push_str(&_err);
            }
        }
    }

    (result_stdout, result_stderr)
}
