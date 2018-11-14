use std::env;

use shell;
use tools;

const DEFAULT_PROMPT: &str = "${COLOR_STATUS}$USER${RESET}@${COLOR_STATUS}$HOSTNAME${RESET}: ${COLOR_STATUS}$CWD${RESET}$ ";
use super::preset::apply_preset_token;
use super::preset::apply_pyenv;

fn is_prompt_item_char(c: char) -> bool {
    let s = c.to_string();
    tools::re_contains(&s, r#"^[a-zA-Z_]$"#)
}

pub fn get_prompt_string() -> String {
    if let Ok(x) = env::var("PROMPT") {
        return x;
    }
    DEFAULT_PROMPT.to_string()
}

fn apply_token(sh: &shell::Shell, result: &mut String, token: &str) {
    if let Ok(x) = env::var(token) {
        result.push_str(&x);
        return;
    }
    apply_preset_token(sh, result, token);
}

pub fn render_prompt(sh: &shell::Shell, ps: &str) -> String {
    let mut prompt = String::new();
    apply_pyenv(&mut prompt);

    let mut met_dollar = false;
    let mut token = String::new();
    for c in ps.chars() {
        if met_dollar {
            if c == '{' {
                continue;
            } else if c == '}' {
                apply_token(sh, &mut prompt, &token);
                token.clear();
                met_dollar = false;
                continue;
            } else if c == '$' {
                if token.is_empty() {
                    // to make single $ as a plain $
                    prompt.push('$');
                    met_dollar = true;
                    continue;
                } else {
                    apply_token(sh, &mut prompt, &token);
                    token.clear();
                    // met_dollar is still true
                    continue;
                }
            } else if is_prompt_item_char(c) {
                token.push(c);
                continue;
            } else if token.is_empty() {
                prompt.push('$');
                prompt.push(c);
                met_dollar = false;
                continue;
            }
        }

        if c == '$' {
            met_dollar = true;
            continue;
        }

        if !token.is_empty() {
            apply_token(sh, &mut prompt, &token);
            token.clear();
        }
        prompt.push(c);
        met_dollar = false;
    }

    if met_dollar {
        // for cases like PROMPT='$$'
        prompt.push('$');
    }

    if prompt.trim().is_empty() {
        return format!("cicada-{} >> ", env!("CARGO_PKG_VERSION"));
    }
    prompt
}
