use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::io::RawFd;
use std::os::unix::io::FromRawFd;
use std::process;

use regex::Regex;
use libc;

use nix::unistd::execve;
use nix::unistd::pipe;
use nix::unistd::{fork, ForkResult};
use nom::IResult;

use builtins;
use libs;
use parsers;
use shell;
use tools::{self, clog};
use types;
use jobc;

use types::CommandOptions;
use types::CommandResult;
use types::Tokens;

/// Entry point for non-ttys (e.g. Cmd-N on MacVim)
pub fn handle_non_tty(sh: &mut shell::Shell) {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    match handle.read_to_string(&mut buffer) {
        Ok(_) => {
            log!("run non tty command: {}", &buffer);
            run_procs(sh, &buffer, false);
        }
        Err(e) => {
            println!("cicada: io stdin read_to_string failed: {:?}", e);
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

pub fn run_procs(sh: &mut shell::Shell, line: &str, tty: bool) -> i32 {
    if tools::is_arithmetic(line) {
        if line.contains('.') {
            match run_calc_float(line) {
                Ok(x) => {
                    println!("{}", x);
                    return 0;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    return 1;
                }
            }
        } else {
            match run_calc_int(line) {
                Ok(x) => {
                    println!("{}", x);
                    return 0;
                }
                Err(e) => {
                    println!("Error: {}", e);
                    return 1;
                }
            }
        }
    }

    let mut cmd_line: String = line.to_string();
    cmd_line = tools::extend_alias(&sh, &cmd_line);
    let mut status = 0;
    let mut sep = String::new();
    for token in parsers::parser_line::line_to_cmds(&cmd_line) {
        if token == ";" || token == "&&" || token == "||" {
            sep = token.clone();
            continue;
        }
        if sep == "&&" && status != 0 {
            break;
        }
        if sep == "||" && status == 0 {
            break;
        }
        let mut cmd = token.clone();
        status = run_proc(sh, &cmd, tty);
    }
    status
}

fn drain_env_tokens(tokens: &mut Tokens) -> HashMap<String, String> {
    let mut envs: HashMap<String, String> = HashMap::new();
    let mut n = 0;
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !tools::re_contains(text, r"^([a-zA-Z0-9_]+)=(.*)$") {
            break;
        }

        let re;
        match Regex::new(r"^([a-zA-Z0-9_]+)=(.*)$") {
            Ok(x) => {
                re = x;
            }
            Err(e) => {
                println_stderr!("Regex new: {:?}", e);
                return envs;
            }
        }
        for cap in re.captures_iter(text) {
            let name = cap[1].to_string();
            let value = parsers::parser_line::unquote(&cap[2]);
            envs.insert(name, value);
        }

        n += 1;
    }
    if n > 0 {
        tokens.drain(0..n);
    }
    envs
}

fn line_to_tokens(sh: &mut shell::Shell, line: &str) -> (Tokens, HashMap<String, String>) {
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
    shell::do_expansion(sh, &mut tokens);
    let envs = drain_env_tokens(&mut tokens);

    if tokens.is_empty() {
        for (name, value) in envs.iter() {
            sh.set_env(name, value);
        }
        return (Vec::new(), HashMap::new());
    }
    return (tokens, envs);
}

pub fn run_proc(sh: &mut shell::Shell, line: &str, tty: bool) -> i32 {
    let (mut tokens, envs) = line_to_tokens(sh, line);
    if tokens.is_empty() {
        return 0;
    }

    let cmd = tokens[0].1.clone();
    // for built-ins
    if cmd == "cd" {
        return builtins::cd::run(sh, &tokens);
    }
    if cmd == "export" {
        return builtins::export::run(sh, &tokens);
    }
    if cmd == "exec" {
        return builtins::exec::run(&tokens);
    }
    if cmd == "exit" {
        return builtins::exit::run(&tokens);
    }
    if cmd == "fg" {
        return builtins::fg::run(sh, &tokens);
    }
    if cmd == "vox" && tokens.len() > 1 && (tokens[1].1 == "enter" || tokens[1].1 == "exit") {
        return builtins::vox::run(sh, &tokens);
    }

    // for any other situations
    let mut background = false;
    let mut len = tokens.len();
    if len > 1 && tokens[len - 1].1 == "&" {
        background = true;
        tokens.pop();
        len -= 1;
    }
    let mut redirect_from = String::new();
    let has_redirect_from = tokens.iter().any(|x| x.1 == "<");
    if has_redirect_from {
        if let Some(idx) = tokens.iter().position(|x| x.1 == "<") {
            tokens.remove(idx);
            len -= 1;
            if len > idx {
                redirect_from = tokens.remove(idx).1;
                len -= 1;
            } else {
                println!("cicada: invalid command: cannot get redirect from");
                return 1;
            }
        }
    }
    if len == 0 {
        return 0;
    }

    let log_cmd = !sh.cmd.starts_with(' ');
    let (term_given, cr) = run_pipeline(
        sh,
        &tokens,
        redirect_from.as_str(),
        background,
        tty,
        false,
        log_cmd,
        Some(envs),
    );

    if term_given {
        unsafe {
            let gid = libc::getpgid(0);
            shell::give_terminal_to(gid);
        }
    }
    cr.status
}

fn run_calc_float(line: &str) -> Result<f64, String> {
    match parsers::parser_float::expr_float(line.as_bytes()) {
        IResult::Done(_, x) => Ok(x),
        IResult::Error(e) => Err(e.description().to_owned()),
        IResult::Incomplete(_) => Err(String::from("Incomplete arithmetic")),
    }
}

fn run_calc_int(line: &str) -> Result<i64, String> {
    match parsers::parser_int::expr_int(line.as_bytes()) {
        IResult::Done(_, x) => Ok(x),
        IResult::Error(e) => Err(e.description().to_owned()),
        IResult::Incomplete(_) => Err(String::from("Incomplete arithmetic")),
    }
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
                                unsafe { libc::dup2(fd, 1); }
                                stdout_redirected = true;
                            } else {
                                unsafe { libc::dup2(fd, 2); }
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
            }

            // We are certain that our string doesn't have 0 bytes in the
            // middle, so we can use CString::new().expect()
            let mut c_envs: Vec<_> = env::vars().map(|(k, v)| CString::new(format!("{}={}", k, v).as_str()).expect("CString error")).collect();
            for (key, value) in options.envs.iter() {
                c_envs.push(
                    CString::new(
                        format!("{}={}", key, value).as_str()
                    ).expect("CString error")
                );
            }

            let path = libs::path::find_first_exec(&program);
            let c_program = CString::new(path.as_str()).expect("CString::new failed");
            let c_args: Vec<_> = cmd.tokens.iter().map(|x| CString::new(x.1.as_str()).expect("CString error")).collect();

            match execve(&c_program, &c_args, &c_envs) {
                Ok(_) => {}
                Err(e) => {
                    println_stderr!("cicada: {}: {:?}", program, e);
                }
            }

            process::exit(1);
        }
        Ok(ForkResult::Parent { child, .. }) => {
            let pid: i32 = child.into();
            if options.isatty && !options.capture_output && idx_cmd == 0 {
                *pgid = pid;
                unsafe {
                    // we need to wait pgid of child set to itself,
                    // before give terminal to it.
                    loop {
                        let _pgid = libc::getpgid(pid);
                        if _pgid == pid {
                            break;
                        }
                    }
                    if !options.background {
                        *term_given = shell::give_terminal_to(pid);
                    }
                    sh.jobs.insert(pid, vec![pid]);
                }
            }

            if idx_cmd > 0 {
                if let Some(x) = sh.jobs.get_mut(&*pgid) {
                    x.push(pid);
                }
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
                *cmd_result = CommandResult{
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

    // the defaults to return
    let mut term_given = false;
    let mut cmd_result = CommandResult::ok();

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

    for pid in &children {
        log!("wait_process: {}", *pid);
        let status = jobc::wait_process(sh, pgid, *pid, true);
        log!("after wait_process: {} status: {}", *pid, status);
        cmd_result = CommandResult::from_status(status);
    }

    (term_given, cmd_result)
}

fn run_with_shell<'a, 'b>(
    sh: &'a mut shell::Shell,
    line: &'b str,
) -> CommandResult {
    let mut line2 = String::from(line);
    line2 = tools::extend_alias(&sh, &line2);
    let (mut tokens, envs) = line_to_tokens(sh, &line2);
    if tokens.is_empty() {
        return CommandResult::ok();
    }

    let mut len = tokens.len();
    if len > 1 && tokens[len - 1].1 == "&" {
        tokens.pop();
        len -= 1;
    }
    let mut redirect_from = String::new();
    let has_redirect_from = tokens.iter().any(|x| x.1 == "<");
    if has_redirect_from {
        if let Some(idx) = tokens.iter().position(|x| x.1 == "<") {
            tokens.remove(idx);
            len -= 1;
            if len > idx {
                redirect_from = tokens.remove(idx).1;
                len -= 1;
            } else {
                println_stderr!("cicada: invalid command: cannot get redirect from");
                return CommandResult::error();
            }
        }
    }
    if len == 0 {
        return CommandResult::ok();
    }

    let (_, cmd_result) = run_pipeline(
        sh,
        &tokens,
        redirect_from.as_str(),
        false,
        false,
        true,
        false,
        Some(envs),
    );
    cmd_result
}

pub fn run(line: &str) -> CommandResult {
    let mut sh = shell::Shell::new();
    return run_with_shell(&mut sh, line);
}

#[cfg(test)]
mod tests {
    use super::run_calc_float;
    use super::run_calc_int;
    use super::run_with_shell;
    use super::shell;
    use super::tools;

    #[test]
    fn test_run_calc_float() {
        assert_eq!(
            run_calc_float("(1 + 2 * 3.0 - 1.54) / 0.2"),
            Ok(27.299999999999997)
        );
    }

    #[test]
    fn test_run_calc_int() {
        assert_eq!(run_calc_int("(5 + 2 * 3 - 4) / 3"), Ok(2));
    }

    #[test]
    fn test_run_itself() {
        use std::fs::File;
        use std::io::BufRead;
        use std::io::BufReader;

        let f = File::open("./tests/run_procs.txt").expect("file not found");
        let file = BufReader::new(&f);
        let mut input = String::new();
        let mut expected_stdout = String::new();
        let mut sh = shell::Shell::new();
        for (num, l) in file.lines().enumerate() {
            let line = l.unwrap();
            match num % 3 {
                0 => {
                    input = line.clone();
                }
                1 => {
                    expected_stdout = line.clone();
                }
                2 => match run_with_shell(&mut sh, &input) {
                    cr => {
                        let ptn = if expected_stdout.is_empty() {
                            r"^$"
                        } else {
                            expected_stdout.as_str()
                        };
                        let matched = tools::re_contains(&cr.stdout.trim(), &ptn);
                        if !matched {
                            println!("\nSTDOUT Check Failed:");
                            println!("input: {}", &input);
                            println!("stdout: {:?}", &cr.stdout.trim());
                            println!("expected: {:?}", &expected_stdout);
                            println!("line number: {}\n", num);
                        }
                        assert!(matched);

                        let ptn = if line.is_empty() {
                            r"^$"
                        } else {
                            line.as_str()
                        };
                        let matched = tools::re_contains(&cr.stderr.trim(), &ptn);
                        if !matched {
                            println!("\nSTDERR Check Failed:");
                            println!("input: {}", &input);
                            println!("stderr: {:?}", &cr.stderr);
                            println!("expected: {}", &ptn);
                            println!("line number: {}\n", num + 1);
                        }
                        assert!(matched);
                    }
                },
                _ => {
                    assert!(false);
                }
            }
        }
    }
}
