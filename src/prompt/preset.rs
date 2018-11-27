use std::env;
use std::error::Error;
use std::io::Write;

use libs;
use shell;
use tools;
use tools::clog;

fn apply_underlined(prompt: &mut String) {
    prompt.push_str(libs::colored::UNDERLINED);
}

fn apply_user(prompt: &mut String) {
    let username = tools::get_user_name();
    prompt.push_str(&username);
}

fn apply_black(prompt: &mut String) {
    prompt.push_str(libs::colored::BLACK);
}

fn apply_black_b(prompt: &mut String) {
    prompt.push_str(libs::colored::BLACK_B);
}

fn apply_black_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::BLACK_BG);
}

fn apply_blue(prompt: &mut String) {
    prompt.push_str(libs::colored::BLUE);
}

fn apply_blue_b(prompt: &mut String) {
    prompt.push_str(libs::colored::BLUE_B);
}

fn apply_blue_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::BLUE_BG);
}

fn apply_bold(prompt: &mut String) {
    prompt.push_str(libs::colored::BOLD);
}

fn apply_green(prompt: &mut String) {
    prompt.push_str(libs::colored::GREEN);
}

fn apply_green_b(prompt: &mut String) {
    prompt.push_str(libs::colored::GREEN_B);
}

fn apply_green_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::GREEN_BG);
}

fn apply_red(prompt: &mut String) {
    prompt.push_str(libs::colored::RED);
}

fn apply_red_b(prompt: &mut String) {
    prompt.push_str(libs::colored::RED_B);
}

fn apply_red_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::RED_BG);
}

fn apply_white(prompt: &mut String) {
    prompt.push_str(libs::colored::WHITE);
}

fn apply_white_b(prompt: &mut String) {
    prompt.push_str(libs::colored::WHITE_B);
}

fn apply_white_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::WHITE_BG);
}

fn apply_reset(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET);
}

fn apply_color_status(sh: &shell::Shell, prompt: &mut String) {
    if sh.previous_status == 0 {
        prompt.push_str(libs::colored::GREEN_B);
    } else {
        prompt.push_str(libs::colored::RED_B);
    }
}

fn apply_cwd(prompt: &mut String) {
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
    let pwd = if last.is_empty() {
        "/"
    } else if current_dir == home {
        "~"
    } else {
        last
    };
    prompt.push_str(pwd);
}

fn apply_hostname(prompt: &mut String) {
    let hostname = tools::get_hostname();
    prompt.push_str(&hostname);
}

fn apply_newline(prompt: &mut String) {
    prompt.push('\n');
}

pub fn apply_pyenv(prompt: &mut String) {
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
            apply_blue_b(prompt);
            prompt.push('(');
            prompt.push_str(env_name);
            prompt.push(')');
            apply_reset(prompt);
        }
    }
}

fn apply_others(prompt: &mut String, token: &str) {
    log!("unknown prompt item: {:?}", token);
    let s = format!("<{}>", token);
    prompt.push_str(&s);
}

pub fn apply_preset_item(sh: &shell::Shell, prompt: &mut String, token: &str) {
    match token.to_ascii_lowercase().as_ref() {
        "black" => apply_black(prompt),
        "black_b" => apply_black_b(prompt),
        "black_bg" => apply_black_bg(prompt),
        "blue" => apply_blue(prompt),
        "blue_b" => apply_blue_b(prompt),
        "blue_bg" => apply_blue_bg(prompt),
        "bold" => apply_bold(prompt),
        "color_status" => apply_color_status(sh, prompt),
        "cwd" => apply_cwd(prompt),
        "green" => apply_green(prompt),
        "green_b" => apply_green_b(prompt),
        "green_bg" => apply_green_bg(prompt),
        "hostname" => apply_hostname(prompt),
        "newline" => apply_newline(prompt),
        "red" => apply_red(prompt),
        "red_b" => apply_red_b(prompt),
        "red_bg" => apply_red_bg(prompt),
        "reset" => apply_reset(prompt),
        "underlined" => apply_underlined(prompt),
        "user" => apply_user(prompt),
        "white" => apply_white(prompt),
        "white_b" => apply_white_b(prompt),
        "white_bg" => apply_white_bg(prompt),
        _ => apply_others(prompt, token),
    }
}
