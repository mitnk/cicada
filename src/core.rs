use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::process;

use libc;
use nix::unistd::{execve, fork, pipe, ForkResult};
use nix::errno::Errno;
use nix::Error;

use crate::builtins;
use crate::calculator;
use crate::jobc;
use crate::libs;
use crate::parsers;
use crate::shell;
use crate::scripting;
use crate::tools::{self, clog};
use crate::types;

use crate::types::CommandOptions;
use crate::types::CommandResult;
use crate::types::Tokens;

// #[allow(clippy::cyclomatic_complexity)]
pub fn run_pipeline(
    sh: &mut shell::Shell,
    tokens: &Tokens,
    redirect_from: &str,
    background: bool,
    tty: bool,
    capture_output: bool,
    log_cmd: bool,
    envs: Option<HashMap<String, String>>,
) -> (bool, CommandResult) {
    if background && capture_output {
        println_stderr!("cicada: cannot capture output of background cmd");
        return (false, CommandResult::error());
    }

    let line = parsers::parser_line::tokens_to_line(tokens);
    if tools::is_arithmetic(&line) {
        let mut cr = CommandResult::new();
        match run_calculator(&line) {
            Ok(x) => {
                cr.stdout = x;
            }
            Err(e) => {
                cr.stderr = e.to_string();
            }
        }
        return (true, cr);
    }

    // TODO: func arg1 arg2
    log!("try run: {:?}", &line);
    log!("funcs: {:?}", sh.funcs);
    if let Some(func_body) = sh.get_func(&line) {
        let args = vec!["cicada".to_string()];
        scripting::run_lines(sh, &func_body, &args, capture_output);
        // TODO: xxx
        return (false, CommandResult::new());
    }

    // the defaults to return
    let mut term_given = false;
    let mut cmd_result = CommandResult::new();

    let cmds = tokens_to_cmd_tokens(&tokens);
    if log_cmd {
        log_cmd_info(&cmds);
    }

    let length = cmds.len();
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
        unsafe { libc::isatty(0) == 1 }
    } else {
        false
    };
    let mut i = 0;
    let mut pgid: i32 = 0;
    let mut children: Vec<i32> = Vec::new();

    let mut _envs: HashMap<String, String> = HashMap::new();
    if let Some(x) = envs {
        _envs = x;
    }

    for cmd in &cmds {
        let cmd_new;
        match parsers::parser_line::cmd_to_with_redirects(&cmd) {
            Ok(x) => {
                cmd_new = x;
            }
            Err(e) => {
                println!("cicada: cmd_to_with_redirects failed: {:?}", e);
                return (false, CommandResult::error());
            }
        }

        let cmd_ = parsers::parser_line::tokens_to_args(&cmd_new.tokens);
        if cmd_.is_empty() {
            println!("cicada: cmd_ is empty");
            return (false, CommandResult::error());
        }

        let options = CommandOptions {
            redirect_from: redirect_from.to_string(),
            isatty: isatty,
            capture_output: capture_output,
            background: background,
            envs: _envs.clone(),
        };

        let child_id: i32 = run_command(
            sh,
            &cmd_new,
            i,
            &options,
            &mut pgid,
            &mut term_given,
            &mut cmd_result,
            &pipes,
        );

        if child_id > 0 && !background {
            children.push(child_id);
        }

        i += 1;
    }

    if background {
        if let Some(job) = sh.get_job_by_gid(pgid) {
            println_stderr!("[{}] {}", job.id, job.gid);
        }
    }

    for pid in &children {
        let status = jobc::wait_process(sh, pgid, *pid, true);
        if capture_output {
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

fn run_command(
    sh: &mut shell::Shell,
    cmd: &types::Command,
    idx_cmd: usize,
    options: &CommandOptions,
    pgid: &mut i32,
    term_given: &mut bool,
    cmd_result: &mut CommandResult,
    pipes: &Vec<(RawFd, RawFd)>,
) -> i32 {
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

    let pipes_count = pipes.len();
    match fork() {
        Ok(ForkResult::Child) => {
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

            // read from pipe instead of stdin
            if idx_cmd > 0 {
                let fds_prev = pipes[idx_cmd - 1];
                unsafe {
                    libc::dup2(fds_prev.0, 0);
                    libc::close(fds_prev.0);
                }
            }

            // all processes except the last one need to get stdout piped
            if idx_cmd < pipes_count {
                let fds = pipes[idx_cmd];
                unsafe {
                    libc::dup2(fds.1, 1);
                    // libc::close(fds.1);
                }
            }

            if idx_cmd == 0 && !options.redirect_from.is_empty() {
                let fd = tools::get_fd_from_file(&options.redirect_from);
                unsafe {
                    libc::dup2(fd, 0);
                    libc::close(fd);
                }
            }

            let mut stdout_redirected = false;
            let mut stderr_redirected = false;
            for item in &cmd.redirects {
                let from_ = &item.0;
                let op_ = &item.1;
                let to_ = &item.2;
                if to_ == "&1" && from_ == "2" {
                    unsafe {
                        if idx_cmd < pipes_count {
                            let fds = pipes[idx_cmd];
                            libc::dup2(fds.1, 2);
                        // libc::close(fds.1);
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
                let status = builtins::history::run(&cmd);
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

            // We are certain that our string doesn't have 0 bytes in the
            // middle, so we can use CString::new().expect()
            let mut c_envs: Vec<_> = env::vars()
                .map(|(k, v)| {
                    CString::new(format!("{}={}", k, v).as_str()).expect("CString error")
                })
                .collect();
            for (key, value) in options.envs.iter() {
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

                    if !options.background {
                        *term_given = shell::give_terminal_to(pid);
                    }
                }
            }

            if options.isatty && !options.capture_output {
                let _cmd = parsers::parser_line::tokens_to_line(&cmd.tokens);
                sh.insert_job(*pgid, pid, &_cmd, "Running", options.background);
            }

            if idx_cmd < pipes_count {
                let fds = pipes[idx_cmd];
                unsafe {
                    libc::close(fds.1);
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

            return pid;
        }
        Err(_) => {
            println_stderr!("Fork failed");
            *cmd_result = CommandResult::error();
        }
    }
    0
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

// TODO: write tests
fn tokens_to_cmd_tokens(tokens: &Tokens) -> Vec<Tokens> {
    let mut cmd = Vec::new();
    let mut cmds = Vec::new();
    for token in tokens {
        let sep = &token.0;
        let value = &token.1;
        if sep.is_empty() && value == "|" {
            if cmd.is_empty() {
                return Vec::new();
            }
            cmds.push(cmd.clone());
            cmd = Vec::new();
        } else {
            cmd.push(token.clone());
        }
    }
    if cmd.is_empty() {
        return Vec::new();
    }
    cmds.push(cmd.clone());
    cmds
}

fn log_cmd_info(cmds: &Vec<Tokens>) {
    let length = cmds.len();
    let mut info = String::new();
    for (i, cmd) in cmds.iter().enumerate() {
        for item in cmd {
            let sep = &item.0;
            let token = &item.1;
            info.push_str(sep);
            info.push_str(token);
            info.push_str(sep);
            info.push(' ');
        }
        if length > 1 && i < length - 1 {
            info.push_str("| ")
        }
    }
    log!("run: {}", info.trim());
}
