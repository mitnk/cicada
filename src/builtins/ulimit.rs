use clap::{value_t, Arg, App};
use libc;

use std::io::Write;

use crate::shell;
use crate::parsers;
use crate::types::Tokens;


pub fn run(_sh: &shell::Shell, tokens: &Tokens) -> i32 {
    let args = parsers::parser_line::tokens_to_args(tokens);
    // NOTE: these default_value -1 is for reporting only
    // we cannot change the limit less then zero.
    let app = App::new("ulimit")
        .about("Show / Modify shell resource limits")
        .arg(Arg::with_name("report_all")
             .short("a")
             .help("Report all limits"))
        .arg(Arg::with_name("open_files")
             .short("n")
             .default_value("-1")
             .takes_value(true))
        .arg(Arg::with_name("core_file_size")
             .short("c")
             .default_value("-1")
             .takes_value(true));

    if tokens.len() == 2 && (tokens[1].1 == "-h" || tokens[1].1 == "--help") {
        use std::io;
        let mut out = io::stdout();
        app.write_help(&mut out).expect("failed to write to stdout");
        println!("");
        return 0;
    }

    let _matches = app.get_matches_from_safe(&args);
    let matches;
    match _matches {
        Ok(x) => {
            matches = x;
        }
        Err(e) => {
            println_stderr!("ulimit error: {}", e.message);
            return 1;
        }
    }

    let open_files = value_t!(matches, "open_files", i64).unwrap_or(-1);
    let core_file_size = value_t!(matches, "core_file_size", i64).unwrap_or(-1);

    let mut options = Vec::new();
    for (_, o) in tokens {
        if o.starts_with('-') {
            options.push(o);
        }
    }

    if matches.is_present("report_all") || options.len() == 0 {
        report_all();
        return 0;
    }

    if open_files > -1 {
        let ok = set_limit("open_files", open_files as u64);
        if !ok {
            return 1;
        }
    }
    if core_file_size > -1 {
        let ok = set_limit("core_file_size", core_file_size as u64);
        if !ok {
            return 1;
        }
    }

    report_needed(&options, open_files, core_file_size);
    0
}

fn set_limit(limit_name: &str, value: u64) -> bool {
    // Since libc::RLIMIT_NOFILE etc has different types on different OS
    // so we cannot pass them via params, see issue:
    // https://github.com/rust-lang/libc/issues/2029
    let limit_id;
    if limit_name == "open_files" {
        limit_id = libc::RLIMIT_NOFILE
    } else if limit_name == "core_file_size" {
        limit_id = libc::RLIMIT_CORE
    } else {
        println_stderr!("invalid limit name");
        return false;
    }

    let rlp = libc::rlimit {rlim_cur: value, rlim_max: libc::RLIM_INFINITY};
    let rlpb: *const libc::rlimit = &rlp;
    unsafe {
        let res = libc::setrlimit(limit_id, rlpb);
        if res != 0 {
            println_stderr!("error when setting limit");
            return false;
        }
    }
    return true;
}

fn print_limit(limit_name: &str, single_print: bool) {
    let desc;
    let limit_id;
    if limit_name == "open_files" {
        desc = "open files";
        limit_id = libc::RLIMIT_NOFILE;
    } else if limit_name == "core_file_size" {
        desc = "core file size";
        limit_id = libc::RLIMIT_CORE;
    } else {
        println_stderr!("ulimit: error: invalid limit name");
        return;
    }

    let mut rlp = libc::rlimit {rlim_cur: 0, rlim_max: 0};
    let r: *mut libc::rlimit = &mut rlp;
    unsafe {
        let res = libc::getrlimit(limit_id, r);
        if res != 0 {
            println_stderr!("error when getrlimit");
        }
        if single_print {
            println!("{}", rlp.rlim_cur);
        } else {
            println!("{}\t\t{}", desc, rlp.rlim_cur);
        }
    }
}

fn report_all() {
    print_limit("open_files", false);
    print_limit("core_file_size", false);
}

fn report_needed(options: &Vec<&String>, open_files: i64, core_file_size: i64) {
    let single_print = options.len() == 1;
    for o in options {
        if *o == "-n" && open_files == -1 {
            print_limit("open_files", single_print);
        }
        if *o == "-c" && core_file_size == -1 {
            print_limit("core_file_size", single_print);
        }
    }
}
