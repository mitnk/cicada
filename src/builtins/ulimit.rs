use crate::builtins::utils::print_stderr_with_capture;
use crate::builtins::utils::print_stdout_with_capture;
use crate::parsers;
use crate::shell::Shell;
use crate::types::{Command, CommandLine, CommandResult};
use clap::Parser;
use std::io::Error;

struct LimitInfo {
    name: &'static str,
    desc: &'static str,
    id: libc::c_int,
    scale: u64, // multiplier for set, divisor for get (e.g., 1024 for kbytes)
}

const LIMITS: &[LimitInfo] = &[
    LimitInfo {
        name: "open_files",
        desc: "open files",
        id: libc::RLIMIT_NOFILE,
        scale: 1,
    },
    LimitInfo {
        name: "core_file_size",
        desc: "core file size",
        id: libc::RLIMIT_CORE,
        scale: 1,
    },
    LimitInfo {
        name: "max_user_processes",
        desc: "max user processes",
        id: libc::RLIMIT_NPROC,
        scale: 1,
    },
    LimitInfo {
        name: "stack_size",
        desc: "stack size (kbytes)",
        id: libc::RLIMIT_STACK,
        scale: 1024,
    },
];

fn get_limit_info(name: &str) -> Option<&'static LimitInfo> {
    LIMITS.iter().find(|l| l.name == name)
}

#[derive(Parser)]
#[command(name = "ulimit", about = "show / modify shell resource limits")]
#[allow(non_snake_case)]
struct App {
    #[arg(short, help = "All current limits are reported.")]
    a: bool,
    #[arg(
        short,
        value_name = "NEW VALUE",
        help = "The maximum number of open file descriptors."
    )]
    n: Option<Option<u64>>,
    #[arg(
        short,
        value_name = "NEW VALUE",
        help = "The maximum size of core files created."
    )]
    c: Option<Option<u64>>,
    #[arg(
        short,
        value_name = "NEW VALUE",
        help = "The maximum number of processes available to a single user."
    )]
    u: Option<Option<u64>>,
    #[arg(
        short,
        value_name = "NEW VALUE",
        help = "The maximum stack size (kbytes)."
    )]
    s: Option<Option<u64>>,
    #[arg(
        short = 'S',
        help = "Set a soft limit for the given resource. (default)"
    )]
    S: bool,
    #[arg(short = 'H', help = "Set a hard limit for the given resource.")]
    H: bool,
}

pub fn run(_sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    let tokens = &cmd.tokens;
    let args = parsers::parser_line::tokens_to_args(tokens);

    let show_help = args.len() > 1 && (args[1] == "-h" || args[1] == "--help");
    let app = match App::try_parse_from(&args) {
        Ok(app) => app,
        Err(e) => {
            let info = format!("{}", e);
            if show_help {
                print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
            } else {
                print_stderr_with_capture(&info, &mut cr, cl, cmd, capture);
                cr.status = 1;
            }
            return cr;
        }
    };

    let mut all_stdout = String::new();
    let mut all_stderr = String::new();

    if app.H && app.S {
        all_stderr.push_str("cicada: ulimit: cannot specify both -H and -S\n");
    } else if app.a {
        report_all(app.H, &mut all_stdout, &mut all_stderr);
    } else {
        let limit_opts = [
            (app.n, "open_files"),
            (app.c, "core_file_size"),
            (app.u, "max_user_processes"),
            (app.s, "stack_size"),
        ];
        let handled = limit_opts
            .iter()
            .any(|(opt, name)| handle_limit(*opt, name, app.H, &mut all_stdout, &mut all_stderr));
        if !handled {
            report_all(app.H, &mut all_stdout, &mut all_stderr);
        }
    }

    if !all_stdout.is_empty() {
        print_stdout_with_capture(&all_stdout, &mut cr, cl, cmd, capture);
    }
    if !all_stderr.is_empty() {
        print_stderr_with_capture(&all_stderr, &mut cr, cl, cmd, capture);
        cr.status = 1;
    }

    cr
}

fn set_limit(limit_name: &str, value: u64, for_hard: bool) -> String {
    let info = match get_limit_info(limit_name) {
        Some(info) => info,
        None => return String::from("cicada: ulimit: invalid limit name\n"),
    };

    let actual_value = value.saturating_mul(info.scale);

    let mut rlp = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    if unsafe { libc::getrlimit(info.id, &mut rlp) } != 0 {
        return format!(
            "cicada: ulimit: error getting limit: {}\n",
            Error::last_os_error()
        );
    }

    // to support armv7-linux-gnueabihf & 32-bit musl systems
    #[cfg(all(target_pointer_width = "32", target_env = "gnu"))]
    {
        if actual_value > u32::MAX as u64 {
            return String::from("cicada: ulimit: value too large for 32-bit system\n");
        }
        if for_hard {
            rlp.rlim_max = actual_value as u32;
        } else {
            rlp.rlim_cur = actual_value as u32;
        }
    }
    #[cfg(not(all(target_pointer_width = "32", target_env = "gnu")))]
    {
        if for_hard {
            rlp.rlim_max = actual_value;
        } else {
            rlp.rlim_cur = actual_value;
        }
    }

    if unsafe { libc::setrlimit(info.id, &rlp) } != 0 {
        return format!(
            "cicada: ulimit: {}: cannot modify limit: {}\n",
            info.desc,
            Error::last_os_error()
        );
    }

    String::new()
}

fn get_limit(limit_name: &str, single_print: bool, for_hard: bool) -> (String, String) {
    let info = match get_limit_info(limit_name) {
        Some(info) => info,
        None => {
            return (
                String::new(),
                String::from("cicada: ulimit: invalid limit name\n"),
            )
        }
    };

    let mut rlp = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };

    if unsafe { libc::getrlimit(info.id, &mut rlp) } != 0 {
        return (
            String::new(),
            format!(
                "cicada: ulimit: error getting limit: {}\n",
                Error::last_os_error()
            ),
        );
    }

    let to_print = if for_hard { rlp.rlim_max } else { rlp.rlim_cur };

    let output = if to_print == libc::RLIM_INFINITY {
        if single_print {
            "unlimited\n".to_string()
        } else {
            format!("{}\t\tunlimited\n", info.desc)
        }
    } else {
        let display_value = to_print as u64 / info.scale;
        if single_print {
            format!("{}\n", display_value)
        } else {
            format!("{}\t\t{}\n", info.desc, display_value)
        }
    };

    (output, String::new())
}

fn report_all(for_hard: bool, all_stdout: &mut String, all_stderr: &mut String) {
    for info in LIMITS {
        let (out, err) = get_limit(info.name, false, for_hard);
        all_stdout.push_str(&out);
        all_stderr.push_str(&err);
    }
}

fn handle_limit(
    limit_option: Option<Option<u64>>,
    limit_name: &str,
    for_hard: bool,
    all_stdout: &mut String,
    all_stderr: &mut String,
) -> bool {
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
