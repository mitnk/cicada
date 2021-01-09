use std::collections::HashMap;
use std::io::{self, Read, Write};

use libc;
use regex::Regex;

use crate::builtins;
use crate::core;
use crate::libs;
use crate::parsers;
use crate::shell;
use crate::tools::{self, clog};
use crate::types::{CommandResult, Tokens};

/// Entry point for non-ttys (e.g. Cmd-N on MacVim)
pub fn run_procs_for_non_tty(sh: &mut shell::Shell) {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    match handle.read_to_string(&mut buffer) {
        Ok(_) => {
            log!("run non tty command: {}", &buffer);
            run_procs(sh, &buffer, false, false);
        }
        Err(e) => {
            println!("cicada: stdin.read_to_string() failed: {:?}", e);
        }
    }
}

pub fn run_procs(sh: &mut shell::Shell,
                 line: &str,
                 tty: bool,
                 capture: bool) -> Vec<CommandResult> {
    let mut cr_list = Vec::new();

    if tools::is_arithmetic(line) {
        match core::run_calculator(line) {
            Ok(result) => {
                let mut cr = CommandResult::new();
                if capture {
                    cr.stdout = result.clone();
                } else {
                    println!("{}", result);
                }
                sh.previous_status = cr.status;
                cr_list.push(cr);
                return cr_list;
            }
            Err(e) => {
                let mut cr = CommandResult::from_status(0, 1);
                if capture {
                    cr.stderr = e.to_string();
                } else {
                    println_stderr!("cicada: calculator: {}", e);
                }
                sh.previous_status = cr.status;
                cr_list.push(cr);
                return cr_list;
            }
        }
    }

    let mut status = 0;
    let mut sep = String::new();
    for token in parsers::parser_line::line_to_cmds(&line) {
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
        let cmd = token.clone();
        let cr = run_proc(sh, &cmd, tty, capture);
        status = cr.status;
        sh.previous_status = status;
        cr_list.push(cr);
    }
    cr_list
}

fn drain_env_tokens(tokens: &mut Tokens) -> HashMap<String, String> {
    let mut envs: HashMap<String, String> = HashMap::new();
    let mut n = 0;
    let re = Regex::new(r"^([a-zA-Z0-9_]+)=(.*)$").unwrap();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !libs::re::re_contains(text, r"^([a-zA-Z0-9_]+)=(.*)$") {
            break;
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

fn with_pipeline(tokens: &Tokens) -> bool {
    for item in tokens {
        if item.1 == "|" || item.1 == ">" {
            return true;
        }
    }
    false
}

/// Run simple command or pipeline without using `&&`, `||`, `;`.
/// example 1: `ls`
/// example 2: `ls | wc`
fn run_proc(sh: &mut shell::Shell,
            line: &str,
            tty: bool,
            capture: bool) -> CommandResult {
    let (mut tokens, envs) = line_to_tokens(sh, line);
    if tokens.is_empty() {
        return CommandResult::new();
    }

    let cmd = tokens[0].1.clone();
    // for builtins
    if cmd == "alias" && !with_pipeline(&tokens) {
        let status = builtins::alias::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "bg" {
        let status = builtins::bg::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "cd" {
        let status = builtins::cd::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "export" {
        let status = builtins::export::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "exec" {
        let status = builtins::exec::run(&tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "exit" {
        let status = builtins::exit::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "fg" {
        let status = builtins::fg::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "vox" && tokens.len() > 1 && (tokens[1].1 == "enter" || tokens[1].1 == "exit") {
        let status = builtins::vox::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if (cmd == "source" || cmd == ".") && tokens.len() <= 2 {
        let status = builtins::source::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "ulimit" {
        let status = builtins::ulimit::run(sh, &tokens);
        return CommandResult::from_status(0, status);
    } else if cmd == "unalias" {
        let status = builtins::unalias::run(sh, &tokens);
        return CommandResult::from_status(0, status);
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
                println_stderr!("cicada: invalid command: cannot get redirect from");
                return CommandResult::from_status(0, 1);
            }
        }
    }
    if len == 0 {
        return CommandResult::new();
    }

    let log_cmd = !sh.cmd.starts_with(' ');
    let (term_given, cr) = core::run_pipeline(
        sh,
        &tokens,
        &redirect_from,
        background,
        tty,
        capture,
        log_cmd,
        Some(envs),
    );

    if term_given {
        unsafe {
            let gid = libc::getpgid(0);
            shell::give_terminal_to(gid);
        }
    }

    cr
}

fn run_with_shell<'a, 'b>(sh: &'a mut shell::Shell, line: &'b str) -> CommandResult {
    let (mut tokens, envs) = line_to_tokens(sh, &line);
    if tokens.is_empty() {
        return CommandResult::new();
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
        return CommandResult::new();
    }

    let (_, cmd_result) = core::run_pipeline(
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
    use super::core::run_calculator;
    use super::run_with_shell;
    use super::shell;
    use super::libs;

    #[test]
    fn test_run_calculator() {
        assert_eq!(
            run_calculator("(1 + 2 * 3.0 - 1.5) / 0.2"),
            Ok("27.5".to_string())
        );
        assert_eq!(
            run_calculator("(5 + 2 * 3 - 4) / 3"),
            Ok("2".to_string())
        );
        assert_eq!(
            run_calculator("((2 ^ 35) + (3^7) - 9740555) / 10000000"),
            Ok("3435".to_string())
        );
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
                        let matched = libs::re::re_contains(&cr.stdout.trim(), &ptn);
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
                        let matched = libs::re::re_contains(&cr.stderr.trim(), &ptn);
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
