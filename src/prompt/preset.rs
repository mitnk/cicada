use std::env;
use std::error::Error;
use std::io::Write;

use libs;
use shell;
use tools;
use tools::clog;

fn apply_user(result: &mut String) {
    let username = tools::get_user_name();
    result.push_str(&username);
}

fn apply_blue(result: &mut String) {
    result.push_str(libs::colored::BLUE);
}

fn apply_green(result: &mut String) {
    result.push_str(libs::colored::GREEN);
}

fn apply_red(result: &mut String) {
    result.push_str(libs::colored::RED);
}

fn apply_reset(result: &mut String) {
    result.push_str(libs::colored::RESET);
}

fn apply_color_status(sh: &shell::Shell, result: &mut String) {
    if sh.previous_status == 0 {
        result.push_str(libs::colored::GREEN);
    } else {
        result.push_str(libs::colored::RED);
    }
}

fn apply_cwd(result: &mut String) {
    let _current_dir;
    match env::current_dir() {
        Ok(x) => _current_dir = x,
        Err(e) => {
            println_stderr!("cicada: PROMPT: env current_dir error: {}", e.description());
            return;
        }
    }
    let current_dir;
    match _current_dir.to_str() {
        Some(x) => current_dir = x,
        None => {
            println_stderr!("cicada: PROMPT: to_str error");
            return;
        }
    }
    let _tokens: Vec<&str> = current_dir.split('/').collect();

    let last;
    match _tokens.last() {
        Some(x) => last = x,
        None => {
            log!("cicada: PROMPT: token last error");
            return;
        }
    }

    let home = tools::get_user_home();
    let pwd = if last.is_empty() { "/" } else if current_dir == home { "~" } else { last };
    result.push_str(pwd);
}

fn apply_hostname(result: &mut String) {
    let hostname = tools::get_hostname();
    result.push_str(&hostname);
}

fn apply_newline(result: &mut String) {
    result.push('\n');
}

pub fn apply_pyenv(result: &mut String) {
    if let Ok(x) = env::var("VIRTUAL_ENV") {
        if !x.is_empty() {
            let _tokens: Vec<&str> = x.split('/').collect();
            let env_name;
            match _tokens.last() {
                Some(x) => env_name = x,
                None => {
                    log!("prompt token last error");
                    return;
                }
            }
            result.push('(');
            apply_blue(result);
            result.push_str(env_name);
            apply_reset(result);
            result.push(')');
        }
    }
}

fn apply_others(result: &mut String, token: &str) {
    log!("unknown prompt item: {:?}", token);
    let s = format!("<{}>", token);
    result.push_str(&s);
}

pub fn apply_preset_token(sh: &shell::Shell, result: &mut String, token: &str) {
    match token.to_ascii_lowercase().as_ref() {
        "blue" => apply_blue(result),
        "cwd" => apply_cwd(result),
        "green" => apply_green(result),
        "hostname" => apply_hostname(result),
        "newline" => apply_newline(result),
        "red" => apply_red(result),
        "reset" => apply_reset(result),
        "color_status" => apply_color_status(sh, result),
        "user" => apply_user(result),
        _ => apply_others(result, token),
    }
}
