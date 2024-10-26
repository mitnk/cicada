use std::env;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::FromRawFd;
use std::os::fd::RawFd;
use std::process;

use nix::unistd::{execve, ForkResult};
use libs::pipes::pipe;

use crate::builtins;
use crate::calculator;
use crate::jobc;
use crate::libs;
use crate::parsers;
use crate::scripting;
use crate::shell::{self, Shell};
use crate::tools;
use crate::types::{CommandLine, CommandOptions, CommandResult};

fn try_run_builtin_in_subprocess(
    sh: &mut Shell,
    cl: &CommandLine,
    idx_cmd: usize,
    capture: bool,
) -> Option<i32> {
    if let Some(cr) = try_run_builtin(sh, cl, idx_cmd, capture) {
        return Some(cr.status);
    }
    None
}

fn try_run_builtin(
    sh: &mut Shell,
    cl: &CommandLine,
    idx_cmd: usize,
    capture: bool,
) -> Option<CommandResult> {
    // for builtin, only capture its outputs when it locates at the end
    let capture = capture && idx_cmd + 1 == cl.commands.len();

    if idx_cmd >= cl.commands.len() {
        println_stderr!("unexpected error in try_run_builtin");
        return None;
    }

    let cmd = &cl.commands[idx_cmd];
    let tokens = cmd.tokens.clone();
    let cname = tokens[0].1.clone();
    if cname == "alias" {
        let cr = builtins::alias::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "bg" {
        let cr = builtins::bg::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "cd" {
        let cr = builtins::cd::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "cinfo" {
        let cr = builtins::cinfo::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "exec" {
        let cr = builtins::exec::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "exit" {
        let cr = builtins::exit::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "export" {
        let cr = builtins::export::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "fg" {
        let cr = builtins::fg::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "history" {
        let cr = builtins::history::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "jobs" {
        let cr = builtins::jobs::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "minfd" {
        let cr = builtins::minfd::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "read" {
        let cr = builtins::read::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "set" {
        let cr = builtins::set::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "source" {
        let cr = builtins::source::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "ulimit" {
        let cr = builtins::ulimit::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "unalias" {
        let cr = builtins::unalias::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "unset" {
        let cr = builtins::unset::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "unpath" {
        let cr = builtins::unpath::run(sh, cl, cmd, capture);
        return Some(cr);
    } else if cname == "vox" {
        let cr = builtins::vox::run(sh, cl, cmd, capture);
        return Some(cr);
    }
    None
}

/// Run a pipeline (e.g. `echo hi | wc -l`)
/// returns: (is-terminal-given, command-result)
pub fn run_pipeline(
    sh: &mut shell::Shell,
    cl: &CommandLine,
    tty: bool,
    capture: bool,
    log_cmd: bool,
) -> (bool, CommandResult) {
    let mut term_given = false;
    if cl.background && capture {
        println_stderr!("cicada: cannot capture output of background cmd");
        return (term_given, CommandResult::error());
    }

    if let Some(cr) = try_run_calculator(&cl.line, capture) {
        return (term_given, cr);
    }

    // FIXME: move func-run into run single command
    if let Some(cr) = try_run_func(sh, cl, capture, log_cmd) {
        return (term_given, cr);
    }

    if log_cmd {
        log!("run: {}", cl.line);
    }

    let length = cl.commands.len();
    if length == 0 {
        println!("cicada: invalid command: cmds with empty length");
        return (false, CommandResult::error());
    }

    let mut pipes = Vec::new();
    let mut errored_pipes = false;
    for _ in 0..length - 1 {
        match pipe() {
            Ok(fds) => pipes.push(fds),
            Err(e) => {
                errored_pipes = true;
                println_stderr!("cicada: pipeline1: {}", e);
                break;
            }
        }
    }

    if errored_pipes {
        // release fds that already created when errors occurred
        for fds in pipes {
            libs::close(fds.0);
            libs::close(fds.1);
        }
        return (false, CommandResult::error());
    }

    if pipes.len() + 1 != length {
        println!("cicada: invalid command: unmatched pipes count");
        return (false, CommandResult::error());
    }

    let mut pgid: i32 = 0;
    let mut fg_pids: Vec<i32> = Vec::new();

    let isatty = if tty {
        unsafe { libc::isatty(1) == 1 }
    } else {
        false
    };
    let options = CommandOptions {
        isatty,
        capture_output: capture,
        background: cl.background,
        envs: cl.envs.clone(),
    };

    let mut fds_capture_stdout = None;
    let mut fds_capture_stderr = None;
    if capture {
        match pipe() {
            Ok(fds) => fds_capture_stdout = Some(fds),
            Err(e) => {
                println_stderr!("cicada: pipeline2: {}", e);
                return (false, CommandResult::error());
            }
        }
        match pipe() {
            Ok(fds) => fds_capture_stderr = Some(fds),
            Err(e) => {
                if let Some(fds) = fds_capture_stdout {
                    libs::close(fds.0);
                    libs::close(fds.1);
                }
                println_stderr!("cicada: pipeline3: {}", e);
                return (false, CommandResult::error());
            }
        }
    }

    let mut cmd_result = CommandResult::new();
    for i in 0..length {
        let child_id: i32 = run_single_program(
            sh,
            cl,
            i,
            &options,
            &mut pgid,
            &mut term_given,
            &mut cmd_result,
            &pipes,
            &fds_capture_stdout,
            &fds_capture_stderr,
        );

        if child_id > 0 && !cl.background {
            fg_pids.push(child_id);
        }
    }

    if cl.is_single_and_builtin() {
        return (false, cmd_result);
    }

    if cl.background {
        if let Some(job) = sh.get_job_by_gid(pgid) {
            println_stderr!("[{}] {}", job.id, job.gid);
        }
    }

    if !fg_pids.is_empty() {
        let _cr = jobc::wait_fg_job(sh, pgid, &fg_pids);
        // for capture commands, e.g. `echo foo` in `echo "hello $(echo foo)"
        // the cmd_result is already built in loop calling run_single_program()
        // above.
        if !capture {
            cmd_result = _cr;
        }
    }
    (term_given, cmd_result)
}

/// Run a single command.
/// e.g. the `sort -k2` part of `ps ax | sort -k2 | head`
#[allow(clippy::needless_range_loop)]
#[allow(clippy::too_many_arguments)]
fn run_single_program(
    sh: &mut shell::Shell,
    cl: &CommandLine,
    idx_cmd: usize,
    options: &CommandOptions,
    pgid: &mut i32,
    term_given: &mut bool,
    cmd_result: &mut CommandResult,
    pipes: &[(RawFd, RawFd)],
    fds_capture_stdout: &Option<(RawFd, RawFd)>,
    fds_capture_stderr: &Option<(RawFd, RawFd)>,
) -> i32 {
    let capture = options.capture_output;
    if cl.is_single_and_builtin() {
        if let Some(cr) = try_run_builtin(sh, cl, idx_cmd, capture) {
            *cmd_result = cr;
            return unsafe { libc::getpid() };
        }

        println_stderr!("cicada: error when run singler builtin");
        log!("error when run singler builtin: {:?}", cl);
        return 1;
    }

    let pipes_count = pipes.len();
    let mut fds_stdin = None;
    let cmd = cl.commands.get(idx_cmd).unwrap();

    if cmd.has_here_string() {
        match pipe() {
            Ok(fds) => fds_stdin = Some(fds),
            Err(e) => {
                println_stderr!("cicada: pipeline4: {}", e);
                return 1;
            }
        }
    }

    match libs::fork::fork() {
        Ok(ForkResult::Child) => {
            unsafe {
                // child processes need to handle ctrl-Z
                libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
            }

            // close pipes unrelated to current child (left side)
            if idx_cmd > 0 {
                for i in 0..idx_cmd - 1 {
                    let fds = pipes[i];
                    libs::close(fds.0);
                    libs::close(fds.1);
                }
            }
            // close pipes unrelated to current child (right side)
            for i in idx_cmd + 1..pipes_count {
                let fds = pipes[i];
                libs::close(fds.0);
                libs::close(fds.1);
            }
            // close pipe fds for capturing stdout/stderr
            // (they're only used in last child)
            if idx_cmd < pipes_count {
                if let Some(fds) = fds_capture_stdout {
                    libs::close(fds.0);
                    libs::close(fds.1);
                }
                if let Some(fds) = fds_capture_stderr {
                    libs::close(fds.0);
                    libs::close(fds.1);
                }
            }

            if idx_cmd == 0 {
                unsafe {
                    let pid = libc::getpid();
                    libc::setpgid(0, pid);
                }
            } else {
                unsafe {
                    libc::setpgid(0, *pgid);
                }
            }

            // (in child) replace stdin/stdout with read/write ends of pipe
            if idx_cmd > 0 {
                let fds_prev = pipes[idx_cmd - 1];
                libs::dup2(fds_prev.0, 0);
                libs::close(fds_prev.0);
                libs::close(fds_prev.1);
            }
            if idx_cmd < pipes_count {
                let fds = pipes[idx_cmd];
                libs::dup2(fds.1, 1);
                libs::close(fds.1);
                libs::close(fds.0);
            }

            if cmd.has_redirect_from() {
                if let Some(redirect_from) = &cmd.redirect_from {
                    let fd = tools::get_fd_from_file(&redirect_from.clone().1);
                    if fd == -1 {
                        process::exit(1);
                    }

                    libs::dup2(fd, 0);
                    libs::close(fd);
                }
            }

            if cmd.has_here_string() {
                if let Some(fds) = fds_stdin {
                    libs::close(fds.1);
                    libs::dup2(fds.0, 0);
                    libs::close(fds.0);
                }
            }

            let mut stdout_redirected = false;
            let mut stderr_redirected = false;
            for item in &cmd.redirects_to {
                let from_ = &item.0;
                let op_ = &item.1;
                let to_ = &item.2;
                if to_ == "&1" && from_ == "2" {
                    if idx_cmd < pipes_count {
                        libs::dup2(1, 2);
                    } else if !options.capture_output {
                        let fd = libs::dup(1);
                        if fd == -1 {
                            println_stderr!("cicada: dup error");
                            process::exit(1);
                        }
                        libs::dup2(fd, 2);
                    } else {
                        // note: capture output with redirections does not
                        // make much sense
                    }
                } else if to_ == "&2" && from_ == "1" {
                    if idx_cmd < pipes_count || !options.capture_output {
                        let fd = libs::dup(2);
                        if fd == -1 {
                            println_stderr!("cicada: dup error");
                            process::exit(1);
                        }
                        libs::dup2(fd, 1);
                    } else {
                        // note: capture output with redirections does not
                        // make much sense
                    }
                } else {
                    let append = op_ == ">>";
                    match tools::create_raw_fd_from_file(to_, append) {
                        Ok(fd) => {
                            if fd == -1 {
                                println_stderr!("cicada: fork: fd error");
                                process::exit(1);
                            }

                            if from_ == "1" {
                                libs::dup2(fd, 1);
                                stdout_redirected = true;
                            } else {
                                libs::dup2(fd, 2);
                                stderr_redirected = true;
                            }
                        }
                        Err(e) => {
                            println_stderr!("cicada: fork: {}", e);
                            process::exit(1);
                        }
                    }
                }
            }

            // capture output of last process if needed.
            if idx_cmd == pipes_count && options.capture_output {
                if !stdout_redirected {
                    if let Some(fds) = fds_capture_stdout {
                        libs::close(fds.0);
                        libs::dup2(fds.1, 1);
                        libs::close(fds.1);
                    }
                }
                if !stderr_redirected {
                    if let Some(fds) = fds_capture_stderr {
                        libs::close(fds.0);
                        libs::dup2(fds.1, 2);
                        libs::close(fds.1);
                    }
                }
            }

            if cmd.is_builtin() {
                if let Some(status) = try_run_builtin_in_subprocess(sh, cl, idx_cmd, capture) {
                    process::exit(status);
                }
            }

            // our strings do not have '\x00' bytes in them,
            // we can use CString::new().expect() safely.
            let mut c_envs: Vec<_> = env::vars()
                .map(|(k, v)| {
                    CString::new(format!("{}={}", k, v).as_str()).expect("CString error")
                })
                .collect();
            for (key, value) in cl.envs.iter() {
                c_envs.push(
                    CString::new(format!("{}={}", key, value).as_str()).expect("CString error"),
                );
            }

            let program = &cmd.tokens[0].1;
            let path = if program.contains('/') {
                program.clone()
            } else {
                libs::path::find_file_in_path(program, true)
            };
            if path.is_empty() {
                println_stderr!("cicada: {}: command not found", program);
                process::exit(127);
            }

            let c_program = CString::new(path.as_str()).expect("CString::new failed");
            let c_args: Vec<_> = cmd
                .tokens
                .iter()
                .map(|x| CString::new(x.1.as_str()).expect("CString error"))
                .collect();

            let c_args: Vec<&CStr> = c_args.iter().map(|x| x.as_c_str()).collect();
            let c_envs: Vec<&CStr> = c_envs.iter().map(|x| x.as_c_str()).collect();
            match execve(&c_program, &c_args, &c_envs) {
                Ok(_) => {}
                Err(e) => match e {
                    nix::Error::ENOEXEC => {
                        println_stderr!("cicada: {}: exec format error (ENOEXEC)", program);
                    }
                    nix::Error::ENOENT => {
                        println_stderr!("cicada: {}: file does not exist", program);
                    }
                    nix::Error::EACCES => {
                        println_stderr!("cicada: {}: Permission denied", program);
                    }
                    _ => {
                        println_stderr!("cicada: {}: {:?}", program, e);
                    }
                },
            }

            process::exit(1);
        }
        Ok(ForkResult::Parent { child, .. }) => {
            let pid: i32 = child.into();
            if idx_cmd == 0 {
                *pgid = pid;
                unsafe {
                    // we need to wait pgid of child set to itself,
                    // before give terminal to it (for macos).
                    // 1. this loop causes `bash`, `htop` etc to go `T` status
                    //    immediate after start on linux (ubuntu).
                    // 2. but on mac, we need this loop, otherwise commands
                    //    like `vim` will go to `T` status after start.
                    if cfg!(target_os = "macos") {
                        loop {
                            let _pgid = libc::getpgid(pid);
                            if _pgid == pid {
                                break;
                            }
                        }
                    }

                    if sh.has_terminal
                        && options.isatty
                        && !cl.background
                    {
                        *term_given = shell::give_terminal_to(pid);
                    }
                }
            }

            if options.isatty && !options.capture_output {
                let _cmd = parsers::parser_line::tokens_to_line(&cmd.tokens);
                sh.insert_job(*pgid, pid, &_cmd, "Running", cl.background);
            }

            if let Some(redirect_from) = &cmd.redirect_from {
                if redirect_from.0 == "<<<" {
                    if let Some(fds) = fds_stdin {
                        unsafe {
                            libs::close(fds.0);

                            let mut f = File::from_raw_fd(fds.1);
                            match f.write_all(redirect_from.1.clone().as_bytes()) {
                                Ok(_) => {}
                                Err(e) => println_stderr!("cicada: write_all: {}", e),
                            }
                            match f.write_all(b"\n") {
                                Ok(_) => {}
                                Err(e) => println_stderr!("cicada: write_all: {}", e),
                            }
                        }
                    }
                }
            }

            // (in parent) close unused pipe ends
            if idx_cmd < pipes_count {
                let fds = pipes[idx_cmd];
                libs::close(fds.1);
            }
            if idx_cmd > 0 {
                // close pipe end only after dupped in the child
                let fds = pipes[idx_cmd - 1];
                libs::close(fds.0);
            }

            if idx_cmd == pipes_count && options.capture_output {
                let mut s_out = String::new();
                let mut s_err = String::new();

                unsafe {
                    if let Some(fds) = fds_capture_stdout {
                        libs::close(fds.1);

                        let mut f = File::from_raw_fd(fds.0);
                        match f.read_to_string(&mut s_out) {
                            Ok(_) => {}
                            Err(e) => println_stderr!("cicada: readstr: {}", e),
                        }
                    }
                    if let Some(fds) = fds_capture_stderr {
                        libs::close(fds.1);
                        let mut f_err = File::from_raw_fd(fds.0);
                        match f_err.read_to_string(&mut s_err) {
                            Ok(_) => {}
                            Err(e) => println_stderr!("cicada: readstr: {}", e),
                        }
                    }
                }

                *cmd_result = CommandResult {
                    gid: *pgid,
                    status: 0,
                    stdout: s_out.clone(),
                    stderr: s_err.clone(),
                };
            }

            pid
        }

        Err(_) => {
            println_stderr!("Fork failed");
            *cmd_result = CommandResult::error();
            0
        }
    }
}

fn try_run_func(
    sh: &mut Shell,
    cl: &CommandLine,
    capture: bool,
    log_cmd: bool,
) -> Option<CommandResult> {
    if cl.is_empty() {
        return None;
    }

    let command = &cl.commands[0];
    if let Some(func_body) = sh.get_func(&command.tokens[0].1) {
        let mut args = vec!["cicada".to_string()];
        for token in &command.tokens {
            args.push(token.1.to_string());
        }
        if log_cmd {
            log!("run func: {:?}", &args);
        }
        let cr_list = scripting::run_lines(sh, &func_body, &args, capture);
        let mut stdout = String::new();
        let mut stderr = String::new();
        for cr in cr_list {
            stdout.push_str(cr.stdout.trim());
            stdout.push(' ');
            stderr.push_str(cr.stderr.trim());
            stderr.push(' ');
        }
        let mut cr = CommandResult::new();
        cr.stdout = stdout;
        cr.stderr = stderr;
        return Some(cr);
    }
    None
}

fn try_run_calculator(line: &str, capture: bool) -> Option<CommandResult> {
    if tools::is_arithmetic(line) {
        match run_calculator(line) {
            Ok(result) => {
                let mut cr = CommandResult::new();
                if capture {
                    cr.stdout = result.clone();
                } else {
                    println!("{}", result);
                }
                return Some(cr);
            }
            Err(e) => {
                let mut cr = CommandResult::from_status(0, 1);
                if capture {
                    cr.stderr = e.to_string();
                } else {
                    println_stderr!("cicada: calculator: {}", e);
                }
                return Some(cr);
            }
        }
    }
    None
}

pub fn run_calculator(line: &str) -> Result<String, &str> {
    let parse_result = calculator::calculate(line);
    match parse_result {
        Ok(mut calc) => {
            let expr = calc.next().unwrap().into_inner();

            if line.contains('.') {
                Ok(format!("{}", calculator::eval_float(expr)))
            } else {
                Ok(format!("{}", calculator::eval_int(expr)))
            }
        }
        Err(_) => {
            Err("syntax error")
        }
    }
}
