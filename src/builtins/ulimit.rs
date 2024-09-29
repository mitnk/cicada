use clap::{Parser, CommandFactory};
use std::io::Error;
use crate::builtins::utils::print_stderr_with_capture;
use crate::builtins::utils::print_stdout_with_capture;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

#[derive(Parser)]
#[command(name = "ulimit", about = "show / modify shell resource limits")]
#[allow(non_snake_case)]
struct App {
    #[arg(short, help = "All current limits are reported.")]
    a: bool,
    #[arg(short, value_name = "NEW VALUE", help = "The maximum number of open file descriptors.")]
    n: Option<Option<u64>>,
    #[arg(short, value_name = "NEW VALUE", help = "The maximum size of core files created.")]
    c: Option<Option<u64>>,
    #[arg(short = 'S', help = "Set a soft limit for the given resource. (default)")]
    S: bool,
    #[arg(short = 'H', help = "Set a hard limit for the given resource.")]
    H: bool,
}

pub fn run(_sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        App::command().print_help().unwrap();
        println!();
        return cr;
    }

    let app = App::parse_from(args);

    if app.H && app.S {
        println!("cicada: ulimit: Cannot both hard and soft.");
        cr.status = 1;
        return cr;
    }

    let mut all_stdout = String::new();
    let mut all_stderr = String::new();

    if app.a {
        report_all(&app, &mut all_stdout, &mut all_stderr);
    } else if handle_limit(app.n, "open_files", app.H, &mut all_stdout, &mut all_stderr)
        || handle_limit(app.c, "core_file_size", app.H, &mut all_stdout, &mut all_stderr) {
    } else {
        report_all(&app, &mut all_stdout, &mut all_stderr);
    }

    if !all_stdout.is_empty() {
        print_stdout_with_capture(&all_stdout, &mut cr, cl, cmd, capture);
    }
    if !all_stderr.is_empty() {
        print_stderr_with_capture(&all_stderr, &mut cr, cl, cmd, capture);
    }

    cr
}

fn set_limit(limit_name: &str, value: u64, for_hard: bool) -> String {
    let limit_id = match limit_name {
        "open_files" => libc::RLIMIT_NOFILE,
        "core_file_size" => libc::RLIMIT_CORE,
        _ => return String::from("invalid limit name"),
    };

    let mut rlp = libc::rlimit { rlim_cur: 0, rlim_max: 0 };

    unsafe {
        if libc::getrlimit(limit_id, &mut rlp) != 0 {
            return format!("cicada: ulimit: error getting limit: {}", Error::last_os_error());
        }
    }

    // to support armv7-linux-gnueabihf & 32-bit musl systems
    if for_hard {
        #[cfg(all(target_pointer_width = "32", not(target_env = "musl")))]
        { rlp.rlim_max = value as u32; }

        #[cfg(all(target_pointer_width = "32", target_env = "musl"))]
        { rlp.rlim_max = value as u64; }

        #[cfg(target_pointer_width = "64")]
        { rlp.rlim_max = value; }
    } else {
        #[cfg(all(target_pointer_width = "32", not(target_env = "musl")))]
        { rlp.rlim_cur = value as u32; }

        #[cfg(all(target_pointer_width = "32", target_env = "musl"))]
        { rlp.rlim_cur = value as u64; }

        #[cfg(target_pointer_width = "64")]
        { rlp.rlim_cur = value; }
    }

    unsafe {
        if libc::setrlimit(limit_id, &rlp) != 0 {
            return format!("cicada: ulimit: error setting limit: {}", Error::last_os_error());
        }
    }

    String::new()
}

fn get_limit(limit_name: &str, single_print: bool, for_hard: bool) -> (String, String) {
    let (desc, limit_id) = match limit_name {
        "open_files" => ("open files", libc::RLIMIT_NOFILE),
        "core_file_size" => ("core file size", libc::RLIMIT_CORE),
        _ => return (String::new(), String::from("ulimit: error: invalid limit name")),
    };

    let mut rlp = libc::rlimit { rlim_cur: 0, rlim_max: 0 };

    let mut result_stdout = String::new();
    let mut result_stderr = String::new();

    unsafe {
        if libc::getrlimit(limit_id, &mut rlp) != 0 {
            result_stderr.push_str(&format!("error getting limit: {}", Error::last_os_error()));
            return (result_stdout, result_stderr);
        }

        let to_print = if for_hard { rlp.rlim_max } else { rlp.rlim_cur };

        let info = if to_print == libc::RLIM_INFINITY {
            if single_print { "unlimited\n".to_string() } else { format!("{}\t\tunlimited\n", desc) }
        } else if single_print {
            format!("{}\n", to_print)
        } else {
            format!("{}\t\t{}\n", desc, to_print)
        };

        result_stdout.push_str(&info);
    }

    (result_stdout, result_stderr)
}

fn report_all(app: &App, all_stdout: &mut String, all_stderr: &mut String) {
    for limit_name in &["open_files", "core_file_size"] {
        let (out, err) = get_limit(limit_name, false, app.H);
        all_stdout.push_str(&out);
        all_stderr.push_str(&err);
    }
}

fn handle_limit(
    limit_option: Option<Option<u64>>,
    limit_name: &str,
    for_hard: bool,
    all_stdout: &mut String,
    all_stderr: &mut String) -> bool {
    match limit_option {
        None => false,
        Some(None) => {
            let (out, err) = get_limit(limit_name, true, for_hard);
            all_stdout.push_str(&out);
            all_stderr.push_str(&err);
            true
        }
        Some(Some(value)) => {
            let err = set_limit(limit_name, value, for_hard);
            if !err.is_empty() {
                all_stderr.push_str(&err);
            }
            true
        }
    }
}
