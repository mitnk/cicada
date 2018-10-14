use std::collections::HashMap;
use std::error::Error as STDError;
use std::fs::File;
use std::io::{self, Error, Read, Write};
use std::os::unix::io::RawFd;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{self, Command, Stdio};

use regex::Regex;
use libc;

use nix::sys::wait::waitpid;
use nix::sys::wait::WaitStatus;
use nix::unistd::pipe;
use nix::unistd::Pid;
use nix::unistd::{fork, ForkResult};
use nom::IResult;

use builtins;
use parsers;
use shell;
use tools::{self, clog};
use types;

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

fn run_std_command(
    cmd: &types::Command,
    idx_cmd: usize,
    options: &CommandOptions,
    pgid: &mut i32,
    term_given: &mut bool,
    cmd_result: &mut CommandResult,
    pipes: &Vec<(RawFd, RawFd)>,
) -> i32 {
    let pipes_count = pipes.len();

    let _program = &cmd.tokens[0].1;
    // treat `(ls)` as `ls`
    let program = _program.trim_matches(|c| c == '(' || c == ')');

    let mut p = Command::new(program);
    let args: Vec<String> = cmd.tokens[1..].iter().map(|x| x.1.clone()).collect();
    p.args(args);
    p.envs(&options.envs);

    let the_pgid = *pgid;
    if options.isatty {
        p.before_exec(move || {
            unsafe {
                if idx_cmd == 0 {
                    // set the first process as progress group leader
                    let pid = libc::getpid();
                    libc::setpgid(0, pid);
                } else {
                    libc::setpgid(0, the_pgid as i32);
                }
            }
            Ok(())
        });
    }

    if idx_cmd > 0 {
        let fds_prev = pipes[idx_cmd - 1];
        let pipe_in = unsafe { Stdio::from_raw_fd(fds_prev.0) };
        p.stdin(pipe_in);
    }

    // all processes except the last one need to get stdout piped
    if idx_cmd < pipes_count {
        let fds = pipes[idx_cmd];
        let pipe_out = unsafe { Stdio::from_raw_fd(fds.1) };
        p.stdout(pipe_out);
    }

    // capture output of last process if needed.
    if idx_cmd == pipes_count && options.capture_output {
        p.stdout(Stdio::piped());
        p.stderr(Stdio::piped());
    }

    for item in &cmd.redirects {
        let from_ = &item.0;
        let op_ = &item.1;
        let to_ = &item.2;
        if to_ == "&1" && from_ == "2" {
            unsafe {
                if idx_cmd < pipes_count {
                    let fds = pipes[idx_cmd];
                    let pipe_out = Stdio::from_raw_fd(fds.1);
                    p.stderr(pipe_out);
                } else if !options.capture_output {
                    let fd = libc::dup(1);
                    p.stderr(Stdio::from_raw_fd(fd));
                } else {
                    // note: capture output with redirections does not
                    // make much sense
                }
            }
        } else if to_ == "&2" && from_ == "1" {
            unsafe {
                if idx_cmd < pipes_count || !options.capture_output {
                    let fd = libc::dup(2);
                    p.stdout(Stdio::from_raw_fd(fd));
                } else {
                    // note: capture output with redirections does not
                    // make much sense
                }
            }
        } else {
            let append = op_ == ">>";
            match tools::create_fd_from_file(to_, append) {
                Ok(fd) => {
                    if from_ == "1" {
                        p.stdout(fd);
                    } else {
                        p.stderr(fd);
                    }
                }
                Err(e) => {
                    println_stderr!("cicada: {}", e);
                    *term_given = false;
                    *cmd_result = CommandResult::error();
                    return 0;
                }
            }
        }
    }

    if idx_cmd == 0 && !options.redirect_from.is_empty() {
        let path = Path::new(&options.redirect_from);
        let display = path.display();
        let file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", display, why.description()),
            Ok(file) => file,
        };
        let fd = file.into_raw_fd();
        let file_in = unsafe { Stdio::from_raw_fd(fd) };
        p.stdin(file_in);
    }

    let mut child;
    match p.spawn() {
        Ok(x) => {
            child = x;
        }
        Err(e) => {
            println!("{}: {}", program, e.description());
            return 0;
        }
    }

    let child_id = child.id();
    if options.isatty && !options.background && idx_cmd == 0 {
        *pgid = child_id as i32;
        unsafe {
            *term_given = shell::give_terminal_to(child_id as i32);
        }
    }

    if !options.background && idx_cmd == pipes_count {
        if options.capture_output {
            match child.wait_with_output() {
                Ok(x) => {
                    let _status = if let Some(x) = x.status.code() {
                        x
                    } else {
                        1
                    };
                    *cmd_result = CommandResult {
                        status: _status,
                        stdout: String::from_utf8_lossy(&x.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&x.stderr).to_string(),
                    };
                }
                Err(e) => {
                    println_stderr!("cicada: {:?}", e);
                    *cmd_result = CommandResult::error();
                }
            }
        } else {
            match child.wait() {
                Ok(ecode) => {
                    if ecode.success() {
                        *cmd_result = CommandResult::from_status(0);
                    } else {
                        match ecode.code() {
                            Some(x) => {
                                *cmd_result = CommandResult::from_status(x);
                            }
                            None => {
                                *cmd_result = CommandResult::error();
                            }
                        }
                    }
                }
                Err(_) => {
                    match Error::last_os_error().raw_os_error() {
                        Some(10) => {
                            // no such process; it's already done
                            *cmd_result = CommandResult::from_status(0);
                        }
                        Some(e) => {
                            *cmd_result = CommandResult::from_status(e);
                        }
                        None => {
                            *cmd_result = CommandResult::from_status(1);
                        }
                    }
                }
            }
        }
    }
    child_id as i32
}

fn run_builtin(
    sh: &shell::Shell,
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
                    libc::close(fds.1);
                }
            }

            for item in &cmd.redirects {
                let from_ = &item.0;
                let op_ = &item.1;
                let to_ = &item.2;
                if to_ == "&1" && from_ == "2" {
                    unsafe {
                        if idx_cmd < pipes_count {
                            let fds = pipes[idx_cmd];
                            libc::dup2(fds.1, 2);
                            libc::close(fds.1);
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
                            } else {
                                unsafe { libc::dup2(fd, 2); }
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
                    libc::close(fds_capture_stdout.0);
                    libc::dup2(fds_capture_stdout.1, 1);
                    libc::close(fds_capture_stdout.1);

                    libc::close(fds_capture_stderr.0);
                    libc::dup2(fds_capture_stderr.1, 2);
                    libc::close(fds_capture_stderr.1);
                }
            }


            let program = &cmd.tokens[0].1;
            let mut status = 0;
            if program == "history" {
                status = builtins::history::run(&cmd);
            } else if program == "vox" {
                status = builtins::vox::run(sh, &cmd.tokens);
            } else if program == "cinfo" {
                status = builtins::cinfo::run();
            }
            process::exit(status);
        }
        Ok(ForkResult::Parent { child, .. }) => {
            let pid: i32 = child.into();
            if options.isatty && !options.capture_output && !options.background && idx_cmd == 0 {
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
                    *term_given = shell::give_terminal_to(pid);
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
    sh: &shell::Shell,
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
    let mut cmd_result = CommandResult::empty();

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

        let mut only_builtins = true;
        let child_id: i32 = if sh.is_builtin(&cmd_[0]) {
            run_builtin(
                sh,
                &cmd_new,
                i,
                &options,
                &mut pgid,
                &mut term_given,
                &mut cmd_result,
                &pipes,
            )
        } else {
            only_builtins = false;
            run_std_command(
                &cmd_new,
                i,
                &options,
                &mut pgid,
                &mut term_given,
                &mut cmd_result,
                &pipes,
            )
        };

        if child_id > 0 && !background && (i != length - 1 || only_builtins) {
            // we didn't need to wait bg children, and the last one
            // already wait() itself.
            children.push(child_id);
        }

        i += 1;
    }

    // ack for zombies
    for pid in &children {
        match waitpid(Pid::from_raw(*pid), None) {
            Ok(info) => {
                match info {
                    WaitStatus::Exited(_pid, status) => {
                        if cmd_result.is_empty() {
                            cmd_result = CommandResult::from_status(status);
                        }
                    }
                    _x => {
                        if cmd_result.is_empty() {
                            cmd_result = CommandResult::from_status(1);
                        }
                    }
                }
            }
            Err(_e) => {
                // log!("waitpid error: {:?}", _e);
            }
        }
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
