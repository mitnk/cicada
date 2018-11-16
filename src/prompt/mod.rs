mod main;
mod preset;

use shell;

use self::main::get_prompt_string;
use self::main::render_prompt;

pub fn get_prompt(sh: &shell::Shell) -> String {
    let ps = get_prompt_string();
    render_prompt(sh, &ps)
}
