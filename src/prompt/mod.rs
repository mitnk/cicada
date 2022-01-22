mod main;
mod preset;

use crate::libs;
use crate::shell;
use crate::tools::clog;

use self::main::get_prompt_string;
use self::main::render_prompt;

fn get_prompt_len(prompt: &str) -> i32 {
    let mut count = 0;
    let mut met_x01 = false;
    for c in prompt.chars() {
        if c == '\x01' {
            met_x01 = true;
            continue;
        } else if c == '\x02' {
            met_x01 = false;
            continue;
        }
        if !met_x01 {
            count += 1;
        }
    }
    count
}

pub fn get_prompt(sh: &shell::Shell) -> String {
    let ps = get_prompt_string();
    let mut prompt = render_prompt(sh, &ps);
    if let Some((w, _h)) = libs::term_size::dimensions() {
        if get_prompt_len(&prompt) > (w / 2) as i32
            && !libs::re::re_contains(&ps, r#"(?i)\$\{?newline.\}?"#)
        {
            prompt.push_str("\n$ ");
        }
    } else {
        log!("ERROR: Failed to get term size");
    }
    prompt
}
