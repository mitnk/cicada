use std::collections::HashMap;
use std::error::Error as STDError;
use std::fs::File;
use std::io::{self, Error, Read, Write};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use regex::Regex;

use libc;
use nix::sys::signal;
use nix::unistd::pipe;
use nom::IResult;

use builtins;
use parsers;
use shell;
use tools::{self, clog, CommandResult};
use types;

extern "C" fn handle_sigchld(_: i32) {
    // When handle waitpid here & for commands like `ls | cmd-not-exist`
    // will panic: "wait() should either return Ok or panic"
    // which I currently don't know how to fix.

    /*
    unsafe {
        let mut stat: i32 = 0;
        let ptr: *mut i32 = &mut stat;
        let pid = libc::waitpid(-1, ptr, libc::WNOHANG);
    }
    */
}

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
fn tokens_to_cmd_tokens(tokens: &types::Tokens) -> Vec<types::Tokens> {
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

fn drain_env_tokens(tokens: &mut Vec<(String, String)>) -> HashMap<String, String> {
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

fn line_to_tokens(
    sh: &mut shell::Shell,
    line: &str,
) -> (Vec<(String, String)>, HashMap<String, String>) {
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
    if cmd == "vox" {
        return builtins::vox::run(sh, &tokens);
    }
    if cmd == "history" {
        return builtins::history::run(&tokens);
    }
    if cmd == "exec" {
        return builtins::exec::run(&tokens);
    }
    if cmd == "cinfo" {
        return builtins::cinfo::run(&tokens);
    }
    if cmd == "exit" {
        return builtins::exit::run(&tokens);
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

    let log_cmd = !sh.cmd.starts_with(" ");
    let (result, term_given, _) = run_pipeline(
        tokens.clone(),
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
    result
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

#[allow(cyclomatic_complexity)]
pub fn run_pipeline(
    tokens: types::Tokens,
    redirect_from: &str,
    background: bool,
    tty: bool,
    capture_output: bool,
    log_cmd: bool,
    envs: Option<HashMap<String, String>>,
) -> (i32, bool, Option<Output>) {
    if background && capture_output {
        println_stderr!("cicada: cannot capture output of background cmd");
        return (1, false, None);
    }

    // the defaults to return
    let mut status = 0;
    let mut term_given = false;
    let mut output = None;

    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(handle_sigchld),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );
    unsafe {
        match signal::sigaction(signal::SIGCHLD, &sig_action) {
            Ok(_) => {}
            Err(e) => println!("sigaction error: {:?}", e),
        }
    }

    let cmds = tokens_to_cmd_tokens(&tokens);
    let length = cmds.len();

    // info is built here only for printing log
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

    if log_cmd {
        log!("run: {}", info);
    }

    if length == 0 {
        println!("cicada: invalid command: cmds with empty length");
        return (1, false, None);
    }
    let mut pipes = Vec::new();
    for _ in 0..length - 1 {
        let fds;
        match pipe() {
            Ok(x) => fds = x,
            Err(e) => {
                println!("pipe error: {:?}", e);
                return (1, false, None);
            }
        }
        pipes.push(fds);
    }
    if pipes.len() + 1 != length {
        println!("cicada: invalid command: unmatched pipes count");
        return (1, false, None);
    }

    let isatty = if tty {
        unsafe { libc::isatty(0) == 1 }
    } else {
        false
    };
    let mut i = 0;
    let mut pgid: u32 = 0;
    let mut children: Vec<u32> = Vec::new();

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
                return (1, false, None);
            }
        }

        let cmd_ = parsers::parser_line::tokens_to_args(&cmd_new.tokens);

        if cmd_.is_empty() {
            println!("cicada: cmd_ is empty");
            return (1, false, None);
        }
        let program = &cmd_[0];
        // treat `(ls)` as `ls`
        let mut p = Command::new(program.trim_matches(|c| c == '(' || c == ')'));
        p.args(&cmd_[1..]);
        p.envs(&_envs);

        if isatty {
            p.before_exec(move || {
                unsafe {
                    if i == 0 {
                        // set the first process as progress group leader
                        let pid = libc::getpid();
                        libc::setpgid(0, pid);
                    } else {
                        libc::setpgid(0, pgid as i32);
                    }
                }
                Ok(())
            });
        }

        if i > 0 {
            let fds_prev = pipes[i - 1];
            let pipe_in = unsafe { Stdio::from_raw_fd(fds_prev.0) };
            p.stdin(pipe_in);
        }

        // all processes except the last one need to get stdout piped
        if i < length - 1 {
            let fds = pipes[i];
            let pipe_out = unsafe { Stdio::from_raw_fd(fds.1) };
            p.stdout(pipe_out);
        }

        // capture output of last process if needed.
        if i == length - 1 && capture_output {
            p.stdout(Stdio::piped());
            p.stderr(Stdio::piped());
        }

        for item in &cmd_new.redirects {
            let from_ = &item.0;
            let op_ = &item.1;
            let to_ = &item.2;
            if to_ == "&1" && from_ == "2" {
                unsafe {
                    if i < length - 1 {
                        let fds = pipes[i];
                        let pipe_out = Stdio::from_raw_fd(fds.1);
                        p.stderr(pipe_out);
                    } else if !capture_output {
                        let fd = libc::dup(1);
                        p.stderr(Stdio::from_raw_fd(fd));
                    } else {
                        // note: capture output with redirections does not
                        // make much sense
                    }
                }
            } else if to_ == "&2" && from_ == "1" {
                unsafe {
                    if i < length - 1 || !capture_output {
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
                        return (1, false, None);
                    }
                }
            }
        }

        if i == 0 && !redirect_from.is_empty() {
            let path = Path::new(redirect_from);
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
                if i != length - 1 {
                    children.push(child.id());
                }
            }
            Err(e) => {
                println!("{}: {}", program, e.description());
                status = 1;
                continue;
            }
        }

        if isatty && !background && i == 0 {
            pgid = child.id();
            unsafe {
                term_given = shell::give_terminal_to(pgid as i32);
            }
        }

        if !background && i == length - 1 {
            if capture_output {
                match child.wait_with_output() {
                    Ok(x) => {
                        output = Some(x);
                    }
                    Err(e) => {
                        println_stderr!("cicada: {:?}", e);
                        output = None;
                    }
                }
            } else {
                match child.wait() {
                    Ok(ecode) => {
                        if ecode.success() {
                            status = 0;
                        } else {
                            match ecode.code() {
                                Some(x) => status = x,
                                None => status = 1,
                            }
                        }
                    }
                    Err(_) => {
                        match Error::last_os_error().raw_os_error() {
                            Some(10) => {
                                // no such process; it's already done
                                status = 0;
                            }
                            Some(e) => {
                                status = e;
                            }
                            None => {
                                status = 1;
                            }
                        }
                    }
                }
            }

            // ack of the zombies
            // FIXME: better wait children in signal handler, but ..
            // .. see comments in `handle_sigchld()` above.
            for pid in &children {
                unsafe {
                    let mut stat: i32 = 0;
                    let ptr: *mut i32 = &mut stat;
                    libc::waitpid(*pid as i32, ptr, 0);
                }
            }
        }
        i += 1;
    }
    (status, term_given, output)
}

fn run_with_shell<'a, 'b>(
    sh: &'a mut shell::Shell,
    line: &'b str,
) -> Result<CommandResult, &'b str> {
    let mut line2 = String::from(line);
    line2 = tools::extend_alias(&sh, &line2);
    let (mut tokens, envs) = line_to_tokens(sh, &line2);
    if tokens.is_empty() {
        return Ok(CommandResult::new());
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
                return Err("cicada: invalid command: cannot get redirect from");
            }
        }
    }
    if len == 0 {
        return Ok(CommandResult::new());
    }

    let (status, _, output) = run_pipeline(
        tokens.clone(),
        redirect_from.as_str(),
        false,
        false,
        true,
        false,
        Some(envs),
    );

    match output {
        Some(x) => Ok(CommandResult {
            status,
            stdout: String::from_utf8_lossy(&x.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&x.stderr).into_owned(),
        }),
        None => Err("no output"),
    }
}

pub fn run(line: &str) -> Result<CommandResult, &str> {
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
                    Ok(c) => {
                        let ptn = if expected_stdout.is_empty() {
                            r"^$"
                        } else {
                            expected_stdout.as_str()
                        };
                        let matched = tools::re_contains(&c.stdout.trim(), &ptn);
                        if !matched {
                            println!("\nSTDOUT Check Failed:");
                            println!("input: {}", &input);
                            println!("stdout: {:?}", &c.stdout.trim());
                            println!("expected: {:?}", &expected_stdout);
                            println!("line number: {}\n", num);
                        }
                        assert!(matched);

                        let ptn = if line.is_empty() {
                            r"^$"
                        } else {
                            line.as_str()
                        };
                        let matched = tools::re_contains(&c.stderr.trim(), &ptn);
                        if !matched {
                            println!("\nSTDERR Check Failed:");
                            println!("input: {}", &input);
                            println!("stderr: {:?}", &c.stderr);
                            println!("expected: {}", &ptn);
                            println!("line number: {}\n", num + 1);
                        }
                        assert!(matched);
                    }
                    Err(e) => {
                        println!("run error on {} - {:?}", &input, e);
                        assert!(false);
                    }
                },
                _ => {
                    assert!(false);
                }
            }
        }
    }
}
