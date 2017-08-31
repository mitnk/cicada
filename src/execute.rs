use std::error::Error as STDError;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, Error, Read, Write};
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio, Output};

use nix::unistd::pipe;
use nix::sys::signal;
use nom::IResult;
use libc;

use tools::{self, clog};
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

pub fn handle_non_tty(sh: &mut shell::Shell) {
    log!("handle non tty");
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    match handle.read_to_string(&mut buffer) {
        Ok(_) => {
            run_procs(sh, &buffer, false);
        }
        Err(e) => {
            println!("cicada: io stdin read_to_string failed: {:?}", e);
        }
    }
}

#[allow(needless_pass_by_value)]
fn tokens_to_cmds(tokens: Vec<(String, String)>) -> Vec<Vec<String>> {
    let mut cmd: Vec<String> = Vec::new();
    let mut cmds: Vec<Vec<String>> = Vec::new();
    for token in &tokens {
        let sep = &token.0;
        let value = &token.1;
        if sep.is_empty() && value == "|" {
            if cmd.is_empty() {
                return Vec::new();
            }
            cmds.push(cmd.clone());
            cmd = Vec::new();
        } else {
            cmd.push(value.clone());
        }
    }
    if cmd.is_empty() {
        return Vec::new();
    }
    cmds.push(cmd.clone());
    cmds
}

#[allow(needless_pass_by_value)]
fn args_to_redirections(tokens: Vec<(String, String)>) -> (Vec<(String, String)>, Vec<i32>) {
    let mut vec_redirected = Vec::new();
    let mut args_new = tokens.clone();
    let mut redirected_to = 0;
    for arg in &args_new {
        let value = &arg.1;
        if value == "2>&1" {
            redirected_to = 1;
        }
        if value == "1>&2" {
            redirected_to = 2;
        }
        if value == "|" {
            vec_redirected.push(redirected_to);
            redirected_to = 0;
        }
    }
    vec_redirected.push(redirected_to);

    while args_new.iter().any(|x| x.1 == "2>&1") {
        if let Some(index) = args_new.iter().position(|x| x.1 == "2>&1") {
            args_new.remove(index);
        }
    }
    while args_new.iter().any(|x| x.1 == "1>&2") {
        if let Some(index) = args_new.iter().position(|x| x.1 == "1>&2") {
            args_new.remove(index);
        }
    }
    (args_new, vec_redirected)
}

pub fn run_procs(sh: &mut shell::Shell, line: &str, tty: bool) -> i32 {
    if tools::is_arithmetic(line) {
        if line.contains('.') {
            match parsers::parser_float::expr_float(line.as_bytes()) {
                IResult::Done(_, x) => {
                    println!("{:?}", x);
                }
                IResult::Error(x) => println!("Error: {:?}", x),
                IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
            }
        } else {
            match parsers::parser_int::expr_int(line.as_bytes()) {
                IResult::Done(_, x) => {
                    println!("{:?}", x);
                }
                IResult::Error(x) => println!("Error: {:?}", x),
                IResult::Incomplete(x) => println!("Incomplete: {:?}", x),
            }
        }
        return 0;
    }
    let mut status = 0;
    let mut sep = String::new();
    for token in parsers::parser_line::line_to_cmds(line) {
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
    let mut tokens = parsers::parser_line::cmd_to_tokens(line);
    if tokens.is_empty() {
        return 0;
    }
    extend_alias(sh, &mut tokens);
    let cmd = tokens[0].1.clone();

    // for built-ins
    if cmd == "cd" {
        return builtins::cd::run(sh, &tokens);
    }
    if cmd == "export" {
        return builtins::export::run(sh, line);
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

    // for any other situations
    let mut background = false;
    let mut len = tokens.len();
    if len > 1 && tokens[len - 1].1 == "&" {
        background = true;
        tokens.pop();
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

    let (result, term_given, _) =
        if len > 2 && (tokens[len - 2].1 == ">" || tokens[len - 2].1 == ">>") {
            let append = tokens[len - 2].1 == ">>";
            let redirect_to;
            match tokens.pop() {
                Some(x) => redirect_to = x.1,
                None => {
                    println!("cicada: redirect_to pop error");
                    return 1;
                }
            }
            tokens.pop(); // pop '>>'
            run_pipeline(
                tokens,
                redirect_from.as_str(),
                redirect_to.as_str(),
                append,
                background,
                tty,
                false,
            )
        } else {
            run_pipeline(
                tokens.clone(),
                redirect_from.as_str(),
                "",
                false,
                background,
                tty,
                false,
            )
        };
    if term_given {
        unsafe {
            let gid = libc::getpgid(0);
            shell::give_terminal_to(gid);
        }
    }
    result
}

fn extend_alias(sh: &mut shell::Shell, tokens: &mut Vec<(String, String)>) {
    let tokens_new = tokens.clone();
    let mut is_cmd = false;
    let mut insert_pos: usize = 0;
    for (i, token) in tokens_new.iter().enumerate() {
        let sep = &token.0;
        let arg = &token.1;

        if i == 0 {
            is_cmd = true;
        } else if arg == "|" {
            is_cmd = true;
            insert_pos += 1;
            continue;
        }
        if !is_cmd {
            insert_pos += 1;
            continue;
        }

        let extended;
        match sh.extend_alias(arg) {
            Some(_extended) => {
                extended = _extended;
            }
            None => {
                extended = arg.clone();
            }
        }
        is_cmd = false;
        if extended != *arg {
            let _args = parsers::parser_line::parse_line(extended.as_str());
            for (i, item) in _args.iter().enumerate() {
                if i == 0 {
                    tokens[insert_pos] = (sep.to_string(), item.clone());
                    insert_pos += 1;
                    continue;
                }
                tokens.insert(insert_pos, (sep.to_string(), item.clone()));
                insert_pos += 1;
            }
            continue;
        }
        insert_pos += 1;
    }
}

pub fn run_pipeline(
    args: Vec<(String, String)>,
    redirect_from: &str,
    redirect_to: &str,
    append: bool,
    background: bool,
    tty: bool,
    capture_stdout: bool,
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
    let (args_new, vec_redirected) = args_to_redirections(args);
    let mut cmds = tokens_to_cmds(args_new);
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
        println!("cicada: invalid command: too many pipes");
        return (1, false, None);
    }

    let mut info = String::from("run: ");
    for cmd in &cmds {
        for x in cmd {
            info.push_str(format!("{} ", x).as_str());
        }
        info.push_str("| ");
    }
    match info.pop() {
        Some(_) => {}
        None => println!("cicada: debug pop error"),
    }
    match info.pop() {
        Some(_) => {}
        None => println!("cicada: debug pop error"),
    }
    log!("{}", info);

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
    for cmd in &mut cmds {
        let program = &cmd[0];
        // treat `(ls)` as `ls`
        let mut p = Command::new(program.trim_matches(|c| c == '(' || c == ')'));
        p.args(&cmd[1..]);

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

        // all processes except the last one need to get stdout piped
        if i < length - 1 {
            let fds = pipes[i];
            let pipe_out = unsafe { Stdio::from_raw_fd(fds.1) };
            p.stdout(pipe_out);
        } else if capture_stdout && i == length - 1 {
            p.stdout(Stdio::piped());
        }

        if vec_redirected[i] > 0 {
            if vec_redirected[i] == 1 {
                if i == length - 1 {
                    unsafe {
                        let fd_std = libc::dup(1);
                        p.stderr(Stdio::from_raw_fd(fd_std));
                    }
                } else {
                    let fds = pipes[i];
                    let pipe_out = unsafe { Stdio::from_raw_fd(fds.1) };
                    p.stderr(pipe_out);
                }
            } else if vec_redirected[i] == 2 {
                unsafe {
                    let fd_std = libc::dup(2);
                    p.stdout(Stdio::from_raw_fd(fd_std));
                }
            }
        }

        if i > 0 {
            if vec_redirected[i - 1] == 2 {
                p.stdin(Stdio::null());
            } else {
                let fds_prev = pipes[i - 1];
                let pipe_in = unsafe { Stdio::from_raw_fd(fds_prev.0) };
                p.stdin(pipe_in);
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

        // redirect output if needed
        if redirect_to != "" && i == length - 1 {
            let mut oos = OpenOptions::new();
            if append {
                oos.append(true);
            } else {
                oos.write(true);
                oos.truncate(true);
            }
            match oos.create(true).open(redirect_to) {
                Ok(x) => {
                    let fd = x.into_raw_fd();
                    let file_out = unsafe { Stdio::from_raw_fd(fd) };
                    p.stdout(file_out);
                }
                Err(e) => {
                    println_stderr!("cicada: redirect file create error - {:?}", e);
                }
            }
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


#[cfg(test)]
mod tests {
    use super::tokens_to_cmds;
    use super::extend_alias;
    use super::run_pipeline;
    use shell;

    #[test]
    fn test_args_to_cmd() {
        let str_empty = String::new();
        let s = vec![(str_empty.clone(), "ls".to_string())];
        let result = tokens_to_cmds(s);
        let expected = vec![vec!["ls".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            (str_empty.clone(), String::from("ls")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("wc")),
        ];
        let result = tokens_to_cmds(s);
        let expected = vec![vec!["ls".to_string()], vec!["wc".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            (str_empty.clone(), String::from("echo")),
            (str_empty.clone(), String::from(" ")),
        ];
        let result = tokens_to_cmds(s);
        let expected = vec![vec!["echo".to_string(), " ".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            (str_empty.clone(), String::from("ls")),
            (str_empty.clone(), String::from("-lh")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("wc")),
            (str_empty.clone(), String::from("-l")),
            (str_empty.clone(), String::from("|")),
            (str_empty.clone(), String::from("less")),
        ];
        let result = tokens_to_cmds(s);
        let expected = vec![
            vec!["ls".to_string(), "-lh".to_string()],
            vec!["wc".to_string(), "-l".to_string()],
            vec!["less".to_string()],
        ];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            (str_empty.clone(), String::from("echo")),
            (str_empty.clone(), String::from("")),
        ];
        let result = tokens_to_cmds(s);
        let expected = vec![vec!["echo".to_string(), "".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

    }

    #[test]
    fn test_extend_alias() {
        let mut sh = shell::Shell::new();
        sh.add_alias("ll", "ls -lh");
        sh.add_alias("wc", "wc -l");
        let mut args = vec![("".to_string(), "ll".to_string())];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "-lh".to_string()),
            ]
        );

        args = vec![
            ("".to_string(), "ll".to_string()),
            ("".to_string(), "|".to_string()),
            ("".to_string(), "wc".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "-lh".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "wc".to_string()),
                (String::new(), "-l".to_string()),
            ]
        );

        args = vec![
            (String::new(), "ls".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "wc".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "wc".to_string()),
                (String::new(), "-l".to_string()),
            ]
        );

        args = vec![
            (String::new(), "ls".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "cat".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "wc".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "cat".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "wc".to_string()),
                (String::new(), "-l".to_string()),
            ]
        );

        args = vec![
            (String::new(), "ls".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "wc".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "cat".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "wc".to_string()),
                (String::new(), "-l".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "cat".to_string()),
            ]
        );

        args = vec![
            (String::new(), "ll".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "cat".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "wc".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "-lh".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "cat".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "wc".to_string()),
                (String::new(), "-l".to_string()),
            ]
        );

        sh.add_alias("grep", "grep -I --color=auto --exclude-dir=.git");
        args = vec![
            (String::new(), "ps".to_string()),
            (String::new(), "ax".to_string()),
            (String::new(), "|".to_string()),
            (String::new(), "grep".to_string()),
            (String::new(), "foo".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ps".to_string()),
                (String::new(), "ax".to_string()),
                (String::new(), "|".to_string()),
                (String::new(), "grep".to_string()),
                (String::new(), "-I".to_string()),
                (String::new(), "--color=auto".to_string()),
                (String::new(), "--exclude-dir=.git".to_string()),
                (String::new(), "foo".to_string()),
            ]
        );

        sh.add_alias("ls", "ls -G");
        args = vec![
            (String::new(), "ls".to_string()),
            (String::new(), "a\\.b".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "ls".to_string()),
                (String::new(), "-G".to_string()),
                (String::new(), "a\\.b".to_string()),
            ]
        );

        sh.add_alias("tx", "tmux");
        sh.add_alias("ls", "ls -G");
        args = vec![
            (String::new(), "tx".to_string()),
            (String::new(), "ls".to_string()),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(
            args,
            vec![
                (String::new(), "tmux".to_string()),
                (String::new(), "ls".to_string()),
            ]
        );
    }

    #[test]
    fn test_run_pipeline() {
        let str_empty = String::new();
        let cmd = vec![(str_empty.clone(), String::from("ls"))];
        let (result, term_given, output) = run_pipeline(cmd, "", "", false, false, false, true);
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
        let (result, term_given, output) = run_pipeline(cmd, "", "", false, false, false, true);
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
        let (result, term_given, output) = run_pipeline(cmd, "", "", false, false, false, true);
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
        let (result, term_given, output) = run_pipeline(cmd, "", "", false, false, false, true);
        assert_eq!(result, 0);
        assert_eq!(term_given, false);
        if let Some(x) = output {
            let stdout = String::from_utf8_lossy(&x.stdout);
            assert_eq!(stdout, "baz bar foo\n");
        } else {
            assert_eq!(1, 2);
        }
    }
}
