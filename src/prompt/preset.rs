use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::libs;
use crate::shell;
use crate::tools;

fn apply_seq(prompt: &mut String) {
    prompt.push_str(libs::colored::SEQ);
}

fn apply_end_seq(prompt: &mut String) {
    prompt.push_str(libs::colored::END_SEQ);
}

fn apply_esc(prompt: &mut String) {
    prompt.push_str(libs::colored::ESC);
}

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

fn apply_hidden(prompt: &mut String) {
    prompt.push_str(libs::colored::HIDDEN);
}

fn apply_reset(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET);
}

fn apply_reverse(prompt: &mut String) {
    prompt.push_str(libs::colored::REVERSE);
}

fn apply_dim(prompt: &mut String) {
    prompt.push_str(libs::colored::DIM);
}

fn apply_blink(prompt: &mut String) {
    prompt.push_str(libs::colored::BLINK);
}

fn apply_reset_underlined(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET_UNDERLINED);
}

fn apply_reset_dim(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET_DIM);
}

fn apply_reset_reverse(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET_REVERSE);
}

fn apply_reset_hidden(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET_HIDDEN);
}

fn apply_reset_blink(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET_BLINK);
}

fn apply_reset_bold(prompt: &mut String) {
    prompt.push_str(libs::colored::RESET_BOLD);
}

fn apply_default(prompt: &mut String) {
    prompt.push_str(libs::colored::DEFAULT);
}

fn apply_default_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::DEFAULT_BG);
}

fn apply_cyan(prompt: &mut String) {
    prompt.push_str(libs::colored::CYAN);
}

fn apply_cyan_l(prompt: &mut String) {
    prompt.push_str(libs::colored::CYAN_L);
}

fn apply_cyan_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::CYAN_BG);
}

fn apply_cyan_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::CYAN_L_BG);
}

fn apply_red_l(prompt: &mut String) {
    prompt.push_str(libs::colored::RED_L);
}

fn apply_red_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::RED_L_BG);
}

fn apply_green_l(prompt: &mut String) {
    prompt.push_str(libs::colored::GREEN_L);
}

fn apply_green_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::GREEN_L_BG);
}

fn apply_gray_l(prompt: &mut String) {
    prompt.push_str(libs::colored::GRAY_L);
}

fn apply_gray_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::GRAY_L_BG);
}

fn apply_gray_d(prompt: &mut String) {
    prompt.push_str(libs::colored::GRAY_D);
}

fn apply_gray_d_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::GRAY_D_BG);
}

fn apply_magenta(prompt: &mut String) {
    prompt.push_str(libs::colored::MAGENTA);
}

fn apply_magenta_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::MAGENTA_BG);
}

fn apply_magenta_l(prompt: &mut String) {
    prompt.push_str(libs::colored::MAGENTA_L);
}

fn apply_magenta_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::MAGENTA_L_BG);
}

fn apply_yellow(prompt: &mut String) {
    prompt.push_str(libs::colored::YELLOW);
}

fn apply_yellow_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::YELLOW_BG);
}

fn apply_yellow_l(prompt: &mut String) {
    prompt.push_str(libs::colored::YELLOW_L);
}

fn apply_yellow_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::YELLOW_L_BG);
}

fn apply_blue_l(prompt: &mut String) {
    prompt.push_str(libs::colored::BLUE_L);
}

fn apply_blue_l_bg(prompt: &mut String) {
    prompt.push_str(libs::colored::BLUE_L_BG);
}

fn apply_color_status(sh: &shell::Shell, prompt: &mut String) {
    if sh.previous_status == 0 {
        prompt.push_str(libs::colored::GREEN_B);
    } else {
        prompt.push_str(libs::colored::RED_B);
    }
}

fn _find_git_root() -> String {
    let current_dir = libs::path::current_dir();
    let dir_git = format!("{}/.git", current_dir);
    if Path::new(&dir_git).exists() {
        return current_dir;
    }

    let mut _dir = current_dir.clone();
    while Path::new(&_dir).parent().is_some() {
        match Path::new(&_dir).parent() {
            Some(p) => {
                _dir = p.to_string_lossy().to_string();
                let dir_git = format!("{}/.git", _dir);
                if Path::new(&dir_git).exists() {
                    return _dir;
                }
            }
            None => {
                break;
            }
        }
    }

    String::new()
}

fn apply_gitbr(prompt: &mut String) {
    let git_root = _find_git_root();
    if git_root.is_empty() {
        return;
    }

    let file_head = format!("{}/.git/HEAD", git_root);
    if !Path::new(&file_head).exists() {
        return;
    }

    let mut file;
    match File::open(&file_head) {
        Ok(x) => file = x,
        Err(e) => {
            println!("cicada: .git/HEAD err: {:?}", e);
            return;
        }
    }
    let mut text = String::new();
    match file.read_to_string(&mut text) {
        Ok(_) => {}
        Err(e) => {
            println!("cicada: read_to_string error: {:?}", e);
            return;
        }
    }

    if let Some(branch) = libs::re::find_first_group(r"^[a-z]+: ?[a-z]+/[a-z]+/(.+)$", text.trim())
    {
        apply_blue_b(prompt);
        if let Ok(x) = env::var("CICADA_GITBR_PREFIX") {
            prompt.push_str(&x);
        }

        let _len_default: i32 = 32;
        let mut len_max = if let Ok(x) = env::var("CICADA_GITBR_MAX_LEN") {
            match x.parse::<i32>() {
                Ok(n) => n,
                Err(_) => _len_default,
            }
        } else {
            _len_default
        };
        if len_max <= 0 {
            len_max = _len_default;
        }

        if branch.len() as i32 <= len_max {
            prompt.push_str(&branch);
        } else {
            let len = branch.len() as i32;
            let offset = (len - len_max + 2) as usize;
            let branch_short = format!("..{}", &branch[offset..]);
            prompt.push_str(&branch_short);
        }
        if let Ok(x) = env::var("CICADA_GITBR_SUFFIX") {
            prompt.push_str(&x);
        }
        apply_reset(prompt);
    }
}

fn apply_cwd(prompt: &mut String) {
    let _current_dir;
    match env::current_dir() {
        Ok(x) => _current_dir = x,
        Err(e) => {
            println_stderr!("cicada: PROMPT: env current_dir error: {}", e);
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

pub fn apply_preset_item(sh: &shell::Shell, prompt: &mut String, token: &str) {
    match token.to_ascii_lowercase().as_ref() {
        "black" => apply_black(prompt),
        "black_b" => apply_black_b(prompt),
        "black_bg" => apply_black_bg(prompt),
        "blink" => apply_blink(prompt),
        "blue" => apply_blue(prompt),
        "blue_b" => apply_blue_b(prompt),
        "blue_bg" => apply_blue_bg(prompt),
        "blue_l" => apply_blue_l(prompt),
        "blue_l_bg" => apply_blue_l_bg(prompt),
        "bold" => apply_bold(prompt),
        "color_status" => apply_color_status(sh, prompt),
        "cwd" => apply_cwd(prompt),
        "cyan" => apply_cyan(prompt),
        "cyan_bg" => apply_cyan_bg(prompt),
        "cyan_l" => apply_cyan_l(prompt),
        "cyan_l_bg" => apply_cyan_l_bg(prompt),
        "default" => apply_default(prompt),
        "default_bg" => apply_default_bg(prompt),
        "dim" => apply_dim(prompt),
        "end_seq" => apply_end_seq(prompt),
        "esc" => apply_esc(prompt),
        "gitbr" => apply_gitbr(prompt),
        "gray_d" => apply_gray_d(prompt),
        "gray_d_bg" => apply_gray_d_bg(prompt),
        "gray_l" => apply_gray_l(prompt),
        "gray_l_bg" => apply_gray_l_bg(prompt),
        "green" => apply_green(prompt),
        "green_b" => apply_green_b(prompt),
        "green_bg" => apply_green_bg(prompt),
        "green_l" => apply_green_l(prompt),
        "green_l_bg" => apply_green_l_bg(prompt),
        "hidden" => apply_hidden(prompt),
        "hostname" => apply_hostname(prompt),
        "magenta" => apply_magenta(prompt),
        "magenta_bg" => apply_magenta_bg(prompt),
        "magenta_l" => apply_magenta_l(prompt),
        "magenta_l_bg" => apply_magenta_l_bg(prompt),
        "newline" => apply_newline(prompt),
        "red" => apply_red(prompt),
        "red_b" => apply_red_b(prompt),
        "red_bg" => apply_red_bg(prompt),
        "red_l" => apply_red_l(prompt),
        "red_l_bg" => apply_red_l_bg(prompt),
        "reset" => apply_reset(prompt),
        "reset_blink" => apply_reset_blink(prompt),
        "reset_bold" => apply_reset_bold(prompt),
        "reset_dim" => apply_reset_dim(prompt),
        "reset_hidden" => apply_reset_hidden(prompt),
        "reset_reverse" => apply_reset_reverse(prompt),
        "reset_underlined" => apply_reset_underlined(prompt),
        "reverse" => apply_reverse(prompt),
        "seq" => apply_seq(prompt),
        "underlined" => apply_underlined(prompt),
        "user" => apply_user(prompt),
        "white" => apply_white(prompt),
        "white_b" => apply_white_b(prompt),
        "white_bg" => apply_white_bg(prompt),
        "yellow" => apply_yellow(prompt),
        "yellow_bg" => apply_yellow_bg(prompt),
        "yellow_l" => apply_yellow_l(prompt),
        "yellow_l_bg" => apply_yellow_l_bg(prompt),
        _ => (),
    }
}
