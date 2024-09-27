use std::collections::HashMap;
use std::io::{self, Read, Write};

use regex::Regex;

use crate::core;
use crate::libs;
use crate::parsers;
use crate::shell::{self, Shell};
use crate::types::{CommandLine, CommandResult, Tokens};

/// Entry point for non-ttys (e.g. Cmd-N on MacVim)
pub fn run_procs_for_non_tty(sh: &mut Shell) {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    match handle.read_to_string(&mut buffer) {
        Ok(_) => {
            log!("run non tty command: {}", &buffer);
            run_command_line(sh, &buffer, false, false);
        }
        Err(e) => {
            println!("cicada: stdin.read_to_string() failed: {:?}", e);
        }
    }
}

pub fn run_command_line(sh: &mut Shell, line: &str, tty: bool,
                        capture: bool) -> Vec<CommandResult> {
    let mut cr_list = Vec::new();
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
    let ptn_env_exp = r"^([a-zA-Z_][a-zA-Z0-9_]*)=(.*)$";
    let re = Regex::new(ptn_env_exp).unwrap();
    for (sep, text) in tokens.iter() {
        if !sep.is_empty() || !libs::re::re_contains(text, ptn_env_exp) {
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

fn line_to_tokens(sh: &mut Shell, line: &str) -> (Tokens, HashMap<String, String>) {
    let linfo = parsers::parser_line::parse_line(line);
    let mut tokens = linfo.tokens;
    shell::do_expansion(sh, &mut tokens);
    let envs = drain_env_tokens(&mut tokens);
    (tokens, envs)
}

fn set_shell_vars(sh: &mut Shell, envs: &HashMap<String, String>) {
    for (name, value) in envs.iter() {
        sh.set_env(name, value);
    }
}

/// Run simple command or pipeline without using `&&`, `||`, `;`.
/// example 1: `ls`
/// example 2: `ls | wc`
fn run_proc(sh: &mut Shell, line: &str, tty: bool,
            capture: bool) -> CommandResult {
    let log_cmd = !sh.cmd.starts_with(' ');
    match CommandLine::from_line(line, sh) {
        Ok(cl) => {
            if cl.is_empty() {
                // for commands with only envs, e.g.
                // $ FOO=1 BAR=2
                // we need to define these **Shell Variables**.
                if !cl.envs.is_empty() {
                    set_shell_vars(sh, &cl.envs);
                }
                return CommandResult::new();
            }

            let (term_given, cr) = core::run_pipeline(sh, &cl, tty, capture, log_cmd);
            if term_given {
                unsafe {
                    let gid = libc::getpgid(0);
                    shell::give_terminal_to(gid);
                }
            }

            cr
        }
        Err(e) => {
            println_stderr!("cicada: {}", e);
            CommandResult::from_status(0, 1)
        }
    }
}

fn run_with_shell(sh: &mut Shell, line: &str) -> CommandResult {
    let (tokens, envs) = line_to_tokens(sh, line);
    if tokens.is_empty() {
        set_shell_vars(sh, &envs);
        return CommandResult::new();
    }

    match CommandLine::from_line(line, sh) {
        Ok(c) => {
            let (term_given, cr) = core::run_pipeline(sh, &c, false, true, false);
            if term_given {
                unsafe {
                    let gid = libc::getpgid(0);
                    shell::give_terminal_to(gid);
                }
            }

            cr
        }
        Err(e) => {
            println_stderr!("cicada: {}", e);
            CommandResult::from_status(0, 1)
        }
    }
}

pub fn run(line: &str) -> CommandResult {
    let mut sh = Shell::new();
    run_with_shell(&mut sh, line)
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
