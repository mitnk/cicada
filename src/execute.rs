use std::collections::HashMap;
use std::error::Error as STDError;
use std::fs::File;
use std::io::{self, Error, Read, Write};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio, Output};

use nix::unistd::pipe;
use nix::sys::signal;
use nom::IResult;
use libc;

use types;
use tools::{self, clog, CommandResult};
use builtins;
use parsers;
use shell;

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
    tools::pre_handle_cmd_line(&sh, &mut cmd_line);
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
        status = run_proc(sh, token.as_str(), tty);
    }
    status
}

pub fn run_proc(sh: &mut shell::Shell, line: &str, tty: bool) -> i32 {
    let mut envs = HashMap::new();
    let cmd_line = tools::remove_envs_from_line(line, &mut envs);

    let mut tokens = parsers::parser_line::cmd_to_tokens(&cmd_line);
    if tokens.is_empty() {
        return 0;
    }
    let cmd = tokens[0].1.clone();

    // for built-ins
    if cmd == "cd" {
        return builtins::cd::run(sh, &tokens);
    }
    if cmd == "export" {
        return builtins::export::run(sh, &cmd_line);
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
            if len >= idx + 1 {
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

    let (result, term_given, _) = run_pipeline(
        tokens.clone(), redirect_from.as_str(), false, background,
        tty, false, Some(envs));

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
        IResult::Done(_, x) => {
            return Ok(x);
        }
        IResult::Error(e) => {
            return Err(e.description().to_owned());
        }
        IResult::Incomplete(_) => {
            return Err(String::from("Incomplete arithmetic"));
        }
    }
}

fn run_calc_int(line: &str) -> Result<i64, String> {
    match parsers::parser_int::expr_int(line.as_bytes()) {
        IResult::Done(_, x) => {
            return Ok(x);
        }
        IResult::Error(e) => {
            return Err(e.description().to_owned());
        }
        IResult::Incomplete(_) => {
            return Err(String::from("Incomplete arithmetic"));
        }
    }
}

pub fn run_pipeline(
    tokens: types::Tokens,
    redirect_from: &str,
    append: bool,
    background: bool,
    tty: bool,
    capture_stdout: bool,
    envs: Option<HashMap<String, String>>,
) -> (i32, bool, Option<Output>) {

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

    let mut term_given = false;
    let cmds = tokens_to_cmd_tokens(&tokens);
    log!("run: {:?}", cmds);

    let length = cmds.len();
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
    let mut status = 0;
    let mut output = None;

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

        if cmd_.len() == 0 {
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
        } else if capture_stdout && i == length - 1 {
            p.stdout(Stdio::piped());
        }

        for item in &cmd_new.redirects {
            let from_ = &item.0;
            let to_ = &item.2;
            if to_ == "&1" {
                unsafe {
                    if from_ == "1" {
                        let fd_std = libc::dup(1);
                        p.stdout(Stdio::from_raw_fd(fd_std));
                    } else {
                        if i < length - 1 {
                            let fds = pipes[i];
                            let pipe_out = Stdio::from_raw_fd(fds.1);
                            p.stderr(pipe_out);
                        } else {
                            // ???
                        }
                    }
                }
            } else if to_ == "&2" {
                unsafe {
                    let fd = libc::dup(2);
                    if from_ == "1" {
                        p.stdout(Stdio::from_raw_fd(fd));
                    } else {
                        p.stderr(Stdio::from_raw_fd(fd));
                    }
                }
            } else {
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

        if i == 0 && redirect_from != "" {
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
            if capture_stdout {
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

pub fn run(line: &str) -> Result<CommandResult, &str> {
    let mut envs = HashMap::new();
    let mut cmd_line = tools::remove_envs_from_line(line, &mut envs);
    let sh = shell::Shell::new();
    tools::pre_handle_cmd_line(&sh, &mut cmd_line);

    let mut tokens = parsers::parser_line::cmd_to_tokens(&cmd_line);
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
            if len >= idx + 1 {
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
        tokens.clone(), redirect_from.as_str(), false, false, false, true,
        Some(envs));

    match output {
        Some(x) => {
            return Ok(CommandResult {
                status: status,
                stdout: String::from_utf8_lossy(&x.stdout).into_owned(),
                stderr: String::new(),
            })
        }
        None => {
            return Err("no output");
        }
    }
}


#[cfg(test)]
mod tests {
    use super::run_pipeline;
    use super::run_calc_float;
    use super::run_calc_int;

    #[test]
    fn test_run_pipeline() {
        let str_empty = String::new();
        let cmd = vec![(str_empty.clone(), String::from("ls"))];
        let (result, term_given, output) =
            run_pipeline(cmd, "", false, false, false, true, None);
        assert_eq!(result, 0);
        assert_eq!(term_given, false);
        if let Some(x) = output {
            let stdout = String::from_utf8_lossy(&x.stdout);
            assert!(stdout.contains("README.md"));
            assert!(stdout.contains("Cargo.toml"));
            assert!(stdout.contains("src"));
            assert!(stdout.contains("LICENSE"));
        } else {
            assert_eq!(1, 2);
        }

        let cmd = vec![
            (str_empty.clone(), String::from("ls")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("cat")),
        ];
        let (result, term_given, output) =
            run_pipeline(cmd, "", false, false, false, true, None);
        assert_eq!(result, 0);
        assert_eq!(term_given, false);
        if let Some(x) = output {
            let stdout = String::from_utf8_lossy(&x.stdout);
            assert!(stdout.contains("README.md"));
            assert!(stdout.contains("Cargo.toml"));
            assert!(stdout.contains("src"));
            assert!(stdout.contains("LICENSE"));
        } else {
            assert_eq!(1, 2);
        }

        let cmd = vec![
            (str_empty.clone(), String::from("ls")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("cat")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("cat")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("more")),
        ];
        let (result, term_given, output) =
            run_pipeline(cmd, "", false, false, false, true, None);
        assert_eq!(result, 0);
        assert_eq!(term_given, false);
        if let Some(x) = output {
            let stdout = String::from_utf8_lossy(&x.stdout);
            assert!(stdout.contains("README.md"));
            assert!(stdout.contains("Cargo.toml"));
            assert!(stdout.contains("src"));
            assert!(stdout.contains("LICENSE"));
        } else {
            assert_eq!(1, 2);
        }

        let cmd = vec![
            (str_empty.clone(), String::from("echo")),
            (str_empty.clone(), String::from("foo")),
            (str_empty.clone(), String::from("bar")),
            (str_empty.clone(), String::from("\"baz")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("awk")),
            (str_empty.clone(), String::from("-F")),
            (str_empty.clone(), String::from("[ \"]+")),
            (str_empty.clone(), String::from("{print $3, $2, $1}")),
        ];
        let (result, term_given, output) =
            run_pipeline(cmd, "", false, false, false, true, None);
        assert_eq!(result, 0);
        assert_eq!(term_given, false);
        if let Some(x) = output {
            let stdout = String::from_utf8_lossy(&x.stdout);
            assert_eq!(stdout, "baz bar foo\n");
        } else {
            assert_eq!(1, 2);
        }
    }

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
}
