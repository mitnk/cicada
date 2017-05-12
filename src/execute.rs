use std::error::Error as STDError;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Error;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use nix::unistd::pipe;
use nix::sys::signal;
use nom::IResult;
use libc;
use shlex;

use builtins;
use jobs;
use parsers;
use shell;
use tools;

extern "C" fn handle_sigchld(_: i32) {
    // When handle waitpid here & for commands like `ls | cmd-not-exist`
    // will got panic: "wait() should either return Ok or panic"
    // which I currently don't know how to fix.

    /*
    unsafe {
        let mut stat: i32 = 0;
        let ptr: *mut i32 = &mut stat;
        let pid = libc::waitpid(-1, ptr, libc::WNOHANG);
    }
    */
}

fn args_to_cmds(args: Vec<String>) -> Vec<Vec<String>> {
    let mut cmd: Vec<String> = Vec::new();
    let mut cmds: Vec<Vec<String>> = Vec::new();
    for token in &args {
        if token != "|" {
            if token == "" {
                return Vec::new();
            }
            cmd.push(token.trim().to_string());
        } else {
            if cmd.len() == 0 {
                return Vec::new();
            }
            cmds.push(cmd.clone());
            cmd = Vec::new();
        }
    }
    if cmd.len() == 0 {
        return Vec::new();
    }
    cmds.push(cmd.clone());
    cmds
}

fn args_to_redirections(args: Vec<String>) -> (Vec<String>, Vec<i32>) {
    let mut vec_redirected = Vec::new();
    let mut args_new = args.clone();
    let mut redirected_to = 0;
    for arg in &args_new {
        if arg == "2>&1" {
            redirected_to = 1;
        }
        if arg == "1>&2" {
            redirected_to = 2;
        }
        if arg == "|" {
            vec_redirected.push(redirected_to);
            redirected_to = 0;
        }
    }
    vec_redirected.push(redirected_to);

    while args_new.iter().position(|x| *x == "2>&1").is_some() {
        if let Some(index) = args_new.iter().position(|x| *x == "2>&1") {
            args_new.remove(index);
        }
    }
    while args_new.iter().position(|x| *x == "1>&2").is_some() {
        if let Some(index) = args_new.iter().position(|x| *x == "1>&2") {
            args_new.remove(index);
        }
    }
    (args_new, vec_redirected)
}

pub fn run_procs(sh: &mut shell::Shell, line: String, tty: bool) -> i32 {
    if tools::is_arithmetic(line.as_str()) {
        if line.contains(".") {
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

    let mut args;
    if let Some(x) = shlex::split(line.trim()) {
        args = x;
    } else {
        println!("shlex split error: does not support multiple line");
        return 1;
    }
    if args.len() == 0 {
        return 0;
    }
    extend_alias(sh, &mut args);

    // for built-ins
    if args[0] == "cd" {
        return builtins::cd::run(sh, args);
    }
    if args[0] == "export" {
        return builtins::export::run(line.as_str());
    }
    if args[0] == "vox" {
        return builtins::vox::run(args);
    }
    if args[0] == "history" {
        return builtins::history::run(args);
    }
    // for any other situations
    let mut background = false;
    let mut len = args.len();
    if len > 1 {
        if args[len - 1] == "&" {
            args.pop().expect("args pop error");
            background = true;
            len -= 1;
        }
    }
    let mut redirect_from = String::new();
    let has_redirect_from = args.iter().any(|x| x == "<");
    if has_redirect_from {
        if let Some(idx) = args.iter().position(|x| x == "<") {
            args.remove(idx);
            len -= 1;
            if len >= idx + 1 {
                redirect_from = args.remove(idx);
                len -= 1;
            } else {
                println!("cicada: invalid command");
                return 1;
            }
        }
    }
    if len <= 0 {
        return 0;
    }

    let (result, term_given) = if len > 2 && (args[len - 2] == ">" || args[len - 2] == ">>") {
        let append = args[len - 2] == ">>";
        let mut args_new = args.clone();
        let redirect_to = args_new.pop().expect("cicada: redirect_to pop error");
        args_new.pop();
        run_pipeline(args_new, redirect_from.as_str(), redirect_to.as_str(), append, background, tty)
    } else {
        run_pipeline(args.clone(), redirect_from.as_str(), "", false, background, tty)
    };
    if term_given {
        unsafe {
            let gid = libc::getpgid(0);
            tools::rlog(format!("try return term to {}\n", gid));
            jobs::give_terminal_to(gid);
        }
    }
    return result;
}

fn extend_alias(sh: &mut shell::Shell, args: &mut Vec<String>) {
    let args_new = args.clone();
    let mut is_cmd = false;
    let mut insert_pos: usize = 0;
    for (i, arg) in args_new.iter().enumerate() {
        if i == 0 {
            is_cmd = true;
        }
        else if arg == "|" {
            is_cmd = true;
            insert_pos += 1;
            continue;
        }
        if !is_cmd {
            insert_pos += 1;
            continue;
        }

        let program = arg;
        let extended = sh.extend_alias(program.as_str());
        if extended != *program {
            if let Some(_args) = shlex::split(extended.trim()) {
                for (i, item) in _args.iter().enumerate() {
                    if i == 0 {
                        args[insert_pos] = item.clone();
                        insert_pos += 1;
                        continue;
                    }
                    args.insert(insert_pos, item.clone());
                    insert_pos += 1;
                }
            } else {
                args[insert_pos] = extended;
                insert_pos += 1;
            }
            continue;
        }
        insert_pos += 1;
        is_cmd = false;
    }
}

fn run_pipeline(args: Vec<String>,
                redirect_from: &str,
                redirect_to: &str,
                append: bool,
                background: bool,
                tty: bool) -> (i32, bool) {
    let sig_action = signal::SigAction::new(signal::SigHandler::Handler(handle_sigchld),
                                            signal::SaFlags::empty(),
                                            signal::SigSet::empty());
    unsafe {
        signal::sigaction(signal::SIGCHLD, &sig_action).expect("sigaction error");
    }

    let mut term_given = false;
    let (args_new, vec_redirected) = args_to_redirections(args);
    let mut cmds = args_to_cmds(args_new);
    let length = cmds.len();
    if length == 0 {
        println!("cicada: invalid command");
        return (1, false);
    }
    let mut pipes = Vec::new();
    for _ in 0..length - 1 {
        let fds = pipe().expect("pipe error");
        pipes.push(fds);
    }
    if pipes.len() + 1 != length {
        println!("cicada: invalid command");
        return (1, false);
    }
    tools::rlog(format!("needs pipes count: {}\n", pipes.len()));

    let mut info = String::from("run: ");
    for cmd in &cmds {
        for x in cmd {
            info.push_str(format!("{} ", x).as_str());
        }
        info.push_str(format!("| ").as_str());
    }
    info.pop().expect("cicada: debug pop error");
    info.pop().expect("cicada: debug pop error");
    info.push_str("\n");
    tools::rlog(info);

    let isatty = if tty { unsafe { libc::isatty(0) == 1 } } else { false };
    let mut i = 0;
    let mut pgid: u32 = 0;
    let mut children: Vec<u32> = Vec::new();
    let mut status = 0;
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
                        tools::rlog(format!("set self as pgroup lead {}\n", pid));
                    } else {
                        libc::setpgid(0, pgid as i32);
                        tools::rlog(format!("set pgroup to {}\n", pgid));
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
                Err(why) => panic!("couldn't open {}: {}",
                                   display,
                                   why.description()),
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
                    tools::println_stderr(format!("cicada: redirect file create error - {:?}", e).as_str());
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
                println!("{}: {:?}", program, e.description());
                status = 1;
                continue;
            }
        }

        if isatty && !background && i == 0 {
            pgid = child.id();
            unsafe {
                tools::rlog(format!("try give term to {} [{}]\n", program, pgid));
                term_given = jobs::give_terminal_to(pgid as i32);
            }
        }

        if !background && i == length - 1 {
            tools::rlog(format!("waiting pid {}: {}\n", child.id(), program));
            match child.wait() {
                Ok(ecode) => {
                    if ecode.success() {
                        status = 0;
                    } else {
                        status = 1;
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

            // ack of the zombies
            // FIXME: better wait children in signal handler, but ..
            // .. see comments in `handle_sigchld()` above.
            for pid in &children {
                unsafe {
                    let mut stat: i32 = 0;
                    let ptr: *mut i32 = &mut stat;
                    tools::rlog(format!("waiting pid {}\n", pid));
                    libc::waitpid(*pid as i32, ptr, 0);
                }
            }
        }
        i += 1;
    }
    return (status, term_given);
}


#[cfg(test)]
mod tests {
    use super::args_to_cmds;
    use super::extend_alias;
    use shell;

    #[test]
    fn test_args_to_cmd() {
        let s = vec![String::from("ls")];
        let result = args_to_cmds(s);
        let expected = vec![vec!["ls".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            String::from("ls"),
            String::from("|"),
            String::from("wc"),
        ];
        let result = args_to_cmds(s);
        let expected = vec![vec!["ls".to_string()], vec!["wc".to_string()]];
        assert_eq!(result.len(), expected.len());
        for (i, item) in result.iter().enumerate() {
            assert_eq!(*item, expected[i]);
        }

        let s = vec![
            String::from("  ls   "),
            String::from("-lh"),
            String::from("|"),
            String::from("wc  "),
            String::from("-l"),
            String::from("|"),
            String::from("  less"),
        ];
        let result = args_to_cmds(s);
        let expected = vec![
            vec!["ls".to_string(), "-lh".to_string()],
            vec!["wc".to_string(), "-l".to_string()],
            vec!["less".to_string()],
        ];
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
        let mut args = vec!["ll".to_string()];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec!["ls".to_string(), "-lh".to_string()]);

        args = vec!["ll".to_string(), "|".to_string(), "wc".to_string()];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec![
           "ls".to_string(),
           "-lh".to_string(),
           "|".to_string(),
           "wc".to_string(),
           "-l".to_string(),
        ]);

        args = vec!["ls".to_string(), "|".to_string(), "wc".to_string()];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec![
           "ls".to_string(),
           "|".to_string(),
           "wc".to_string(),
           "-l".to_string(),
        ]);

        args = vec![
            "ls".to_string(),
            "|".to_string(),
            "cat".to_string(),
            "|".to_string(),
            "wc".to_string(),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec![
            "ls".to_string(),
            "|".to_string(),
            "cat".to_string(),
            "|".to_string(),
            "wc".to_string(),
            "-l".to_string(),
        ]);

        args = vec![
            "ls".to_string(),
            "|".to_string(),
            "wc".to_string(),
            "|".to_string(),
            "cat".to_string(),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec![
            "ls".to_string(),
            "|".to_string(),
            "wc".to_string(),
            "-l".to_string(),
            "|".to_string(),
            "cat".to_string(),
        ]);

        args = vec![
            "ll".to_string(),
            "|".to_string(),
            "cat".to_string(),
            "|".to_string(),
            "wc".to_string(),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec![
            "ls".to_string(),
            "-lh".to_string(),
            "|".to_string(),
            "cat".to_string(),
            "|".to_string(),
            "wc".to_string(),
            "-l".to_string(),
        ]);

        sh.add_alias("grep", "grep -I --color=auto --exclude-dir=.git");
        args = vec![
            "ps".to_string(),
            "ax".to_string(),
            "|".to_string(),
            "grep".to_string(),
            "foo".to_string(),
        ];
        extend_alias(&mut sh, &mut args);
        assert_eq!(args, vec![
            "ps".to_string(),
            "ax".to_string(),
            "|".to_string(),
            "grep".to_string(),
            "-I".to_string(),
            "--color=auto".to_string(),
            "--exclude-dir=.git".to_string(),
            "foo".to_string(),
        ]);
    }
}
