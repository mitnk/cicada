#![allow(unreachable_code)]
use std::io::Write;
use std::process;

pub fn run(tokens: &Vec<(String, String)>) -> i32 {
    if tokens.len() > 2 {
        println_stderr!("cicada: exit: too many arguments");
        return 1;
    }

    let mut code = 0;
    if tokens.len() == 2 {
        let _code = &tokens[1].1;
        match _code.parse::<i32>() {
            Ok(x) => {
                code = x;
            }
            Err(_) => {
                println_stderr!("cicada: exit: a: numeric argument required");
                code = 255;
            }
        }
    }
    process::exit(code);
    0
}
