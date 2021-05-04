use std::env;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::process;

use libc;
use nix::unistd::{execve, pipe, ForkResult};
use nix::errno::Errno;
use nix::Error;

use crate::builtins;
use crate::calculator;
use crate::jobc;
use crate::libs;
use crate::parsers;
use crate::scripting;
use crate::shell::{self, Shell};
use crate::tools::{self, clog};
use crate::types;

use crate::types::CommandLine;
use crate::types::CommandOptions;
use crate::types::CommandResult;

/// Run a pipeline (e.g. `echo hi | wc -l`)
/// returns: (is-terminal-given, command-result)
pub fn run_pipeline(sh: &mut shell::Shell, cl: &CommandLine, tty: bool,
                    capture: bool, log_cmd: bool) -> (bool, CommandResult) {
    let mut term_given = false;

    if cl.background && capture {
        println_stderr!("cicada: cannot capture output of background cmd");
        return (term_given, CommandResult::error());
    }

    if let Some(cr) = try_run_calculator(&cl.line, capture) {
        return (term_given, cr);
    }

    // FIXME: move func-run into _run_single_command()
    if let Some(cr) = try_run_func(sh, &cl, capture, log_cmd) {
        return (term_given, cr);
    }

    let mut cmd_result = CommandResult::new();
    if log_cmd {
        log!("run: {}", cl.line);
    }

    let length = cl.commands.len();
    if length == 0 {
        println!("cicada: invalid command: cmds with empty length");
        return (false, CommandResult::error());
    }
    let mut pipes = Vec::new();
    for _ in 0..length - 1 {
        let fds;
        match pipe() {
            Ok(x) => fds = x,
            Err(e) => {
                println!("pipe error: {:?}", e);
                return (false, CommandResult::error());
            }
        }
        pipes.push(fds);
    }
    if pipes.len() + 1 != length {
        println!("cicada: invalid command: unmatched pipes count");
        return (false, CommandResult::error());
    }

    let isatty = if tty {
        unsafe { libc::isatty(1) == 1 }
    } else {
        false
    };

    let mut pgid: i32 = 0;
    let mut children: Vec<i32> = Vec::new();

    let options = CommandOptions {
        isatty: isatty,
        capture_output: capture,
        background: cl.background,
        envs: cl.envs.clone(),
    };

    for i in 0..length {
        let child_id: i32 = _run_single_command(
            sh,
            cl,
            i,
            &options,
            &mut pgid,
            &mut term_given,
            &mut cmd_result,
            &pipes,
        );

        if child_id > 0 && !cl.background {
            children.push(child_id);
        }
    }

    if cl.background {
        if let Some(job) = sh.get_job_by_gid(pgid) {
            println_stderr!("[{}] {}", job.id, job.gid);
        }
    }

    for pid in &children {
        let status = jobc::wait_process(sh, pgid, *pid, true);
        if capture {
            cmd_result.status = status;
        } else {
            cmd_result = CommandResult::from_status(pgid, status);
        }
    }

    if cmd_result.status == types::STOPPED {
        jobc::mark_job_as_stopped(sh, pgid);
    }

    (term_given, cmd_result)
}

fn _run_single_command(sh: &mut shell::Shell, cl: &CommandLine, idx_cmd: usize,
                       options: &CommandOptions, pgid: &mut i32,
                       term_given: &mut bool, cmd_result: &mut CommandResult,
                       pipes: &Vec<(RawFd, RawFd)>) -> i32 {
    let fds_capture_stdout: (RawFd, RawFd);
    let fds_capture_stderr: (RawFd, RawFd);
    match pipe() {
        Ok(x) => {
            fds_capture_stdout = x;
        }
        Err(e) => {
            println_stderr!("cicada: pipe error: {:?}", e);
            *cmd_result = CommandResult::error();
            return 0;
        }
    }
    match pipe() {
        Ok(x) => {
            fds_capture_stderr = x;
        }
        Err(e) => {
            println_stderr!("cicada: pipe error: {:?}", e);
            *cmd_result = CommandResult::error();
            return 0;
        }
    }

    let fds_stdin: (RawFd, RawFd);
    match pipe() {
        Ok(x) => {
            fds_stdin = x;
        }
        Err(e) => {
            println_stderr!("cicada: pipe error: {:?}", e);
            *cmd_result = CommandResult::error();
            return 0;
        }
    }

    let cmd = cl.commands.get(idx_cmd).unwrap();
    let pipes_count = pipes.len();
    match libs::fork::fork() {
        Ok(ForkResult::Child) => {
            unsafe {
                // child processes need to handle ctrl-Z
                libc::signal(libc::SIGTSTP, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
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
                unsafe {
                    libc::dup2(fds_prev.0, 0);
                    libc::close(fds_prev.1);
                }
            }
            if idx_cmd < pipes_count {
                let fds = pipes[idx_cmd];
                unsafe {
                    libc::dup2(fds.1, 1);
                    libc::close(fds.0);
                }
            }

            if idx_cmd == 0 {
                if cmd.has_redirect_from() {
                    let fd = tools::get_fd_from_file(&cmd.redirect_from.clone().unwrap().1);
                    unsafe {
                        libc::dup2(fd, 0);
                        libc::close(fd);
                    }
                }

                if cmd.has_here_string() {
                    unsafe {
                        libc::dup2(fds_stdin.0, 0);
                        libc::close(fds_stdin.1);
                        libc::close(fds_stdin.0);
                    }
                }
            }

            let mut stdout_redirected = false;
            let mut stderr_redirected = false;
            for item in &cmd.redirects_to {
                let from_ = &item.0;
                let op_ = &item.1;
                let to_ = &item.2;
                if to_ == "&1" && from_ == "2" {
                    unsafe {
                        if idx_cmd < pipes_count {
                            let fds = pipes[idx_cmd];
                            libc::dup2(fds.1, 2);
                        } else if !options.capture_output {
                            let fd = libc::dup(1);
                            libc::dup2(fd, 2);
                        } else {
                            // note: capture output with redirections does not
                            // make much sense
                        }
                    }
                } else if to_ == "&2" && from_ == "1" {
                    unsafe {
                        if idx_cmd < pipes_count || !options.capture_output {
                            let fd = libc::dup(2);
                            libc::dup2(fd, 1);
                        } else {
                            // note: capture output with redirections does not
                            // make much sense
                        }
                    }
                } else {
                    let append = op_ == ">>";
                    match tools::create_raw_fd_from_file(to_, append) {
                        Ok(fd) => {
                            if from_ == "1" {
                                unsafe {
                                    libc::dup2(fd, 1);
                                }
                                stdout_redirected = true;
                            } else {
                                unsafe {
                                    libc::dup2(fd, 2);
                                }
                                stderr_redirected = true;
                            }
                        }
                        Err(e) => {
                            println_stderr!("cicada: {}", e);
                            *cmd_result = CommandResult::error();
                            return 0;
                        }
                    }
                }
            }

            // capture output of last process if needed.
            if idx_cmd == pipes_count && options.capture_output {
                unsafe {
                    if !stdout_redirected {
                        libc::close(fds_capture_stdout.0);
                        libc::dup2(fds_capture_stdout.1, 1);
                        libc::close(fds_capture_stdout.1);
                    }

                    if !stderr_redirected {
                        libc::close(fds_capture_stderr.0);
                        libc::dup2(fds_capture_stderr.1, 2);
                        libc::close(fds_capture_stderr.1);
                    }
                }
            }

            let program = &cmd.tokens[0].1;
            if program == "history" {
                let status = builtins::history::run(sh, &cmd);
                process::exit(status);
            } else if program == "vox" {
                let status = builtins::vox::run(sh, &cmd.tokens);
                process::exit(status);
            } else if program == "cinfo" {
                let status = builtins::cinfo::run();
                process::exit(status);
            } else if program == "jobs" {
                let status = builtins::jobs::run(sh);
                process::exit(status);
            } else if program == "source" || program == "." {
                // NOTE: do pipeline on source would make processes forked,
                // which may not get correct results (e.g. `echo $$`),
                // (e.g. cannot make new $PROMPT take effects).
                let status = builtins::source::run(sh, &cmd.tokens);
                process::exit(status);
            } else if program == "alias" {
                let status = builtins::alias::run(sh, &cmd.tokens);
                process::exit(status);
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

            let path = if program.contains('/') {
                program.clone()
            } else {
                libs::path::find_file_in_path(&program, true)
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
                    Error::Sys(Errno::ENOEXEC) => {
                        println_stderr!("cicada: {}: exec format error (ENOEXEC)", program);
                    }
                    Error::Sys(Errno::ENOENT) => {
                        println_stderr!("cicada: {}: file does not exist", program);
                    }
                    Error::Sys(Errno::EACCES) => {
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
            if options.isatty && !options.capture_output && idx_cmd == 0 {
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

                    if !cl.background {
                        *term_given = shell::give_terminal_to(pid);
                    }
                }
            }

            if options.isatty && !options.capture_output {
                let _cmd = parsers::parser_line::tokens_to_line(&cmd.tokens);
                sh.insert_job(*pgid, pid, &_cmd, "Running", cl.background);
            }

            if let Some(redirect_from) = &cmd.redirect_from {
                unsafe {
                    if redirect_from.0 == "<<<" {
                        libc::close(fds_stdin.0);

                        let mut f = File::from_raw_fd(fds_stdin.1);
                        f.write_all(redirect_from.1.clone().as_bytes()).unwrap();
                        f.write_all(b"\n").unwrap();

                        libc::close(fds_stdin.1);
                    }
                }
            }

            // (in parent) close unused pipe ends
            if idx_cmd < pipes_count {
                let fds = pipes[idx_cmd];
                unsafe {
                    libc::close(fds.1);
                }
            }
            if idx_cmd > 0 {
                unsafe {
                    // close pipe end only after dupped in the child
                    let fds = pipes[idx_cmd - 1];
                    libc::close(fds.0);
                }
            }

            if idx_cmd == pipes_count && options.capture_output {
                unsafe {
                    libc::close(fds_capture_stdout.1);
                    libc::close(fds_capture_stderr.1);
                }

                let mut f_out = unsafe { File::from_raw_fd(fds_capture_stdout.0) };
                let mut s_out = String::new();
                f_out.read_to_string(&mut s_out).expect("fds stdout");

                let mut f_err = unsafe { File::from_raw_fd(fds_capture_stderr.0) };
                let mut s_err = String::new();
                f_err.read_to_string(&mut s_err).expect("fds stderr");
                *cmd_result = CommandResult {
                    gid: *pgid,
                    status: 0,
                    stdout: s_out.clone(),
                    stderr: s_err.clone(),
                }
            }

            unsafe {
                libc::close(fds_capture_stdout.0);
                libc::close(fds_capture_stdout.1);
                libc::close(fds_capture_stderr.0);
                libc::close(fds_capture_stderr.1);
                libc::close(fds_stdin.0);
                libc::close(fds_stdin.1);
            }

            return pid;
        }

        Err(_) => {
            println_stderr!("Fork failed");
            *cmd_result = CommandResult::error();
            0
        }
    }
}

fn try_run_func(sh: &mut Shell, cl: &CommandLine, capture: bool,
                log_cmd: bool) -> Option<CommandResult> {
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
            stdout.push_str(&cr.stdout.trim());
            stdout.push(' ');
            stderr.push_str(&cr.stderr.trim());
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
        Ok(calc) => {
            if line.contains('.') {
                Ok(format!("{}", calculator::eval_float(calc)))
            } else {
                Ok(format!("{}", calculator::eval_int(calc)))
            }
        }
        Err(_) => {
            return Err("syntax error");
        }
    }
}
