use regex::Regex;
use std::env;

use crate::libs;
use crate::tools;

pub fn expand_home_string(text: &mut String) {
    let v = vec![
        r"(?P<head> +)~(?P<tail> +)",
        r"(?P<head> +)~(?P<tail>/)",
        r"^(?P<head> *)~(?P<tail>/)",
        r"(?P<head> +)~(?P<tail> *$)",
    ];
    for item in &v {
        let re;
        if let Ok(x) = Regex::new(item) {
            re = x;
        } else {
            return;
        }
        let home = tools::get_user_home();
        let ss = text.clone();
        let to = format!("$head{}$tail", home);
        let result = re.replace_all(ss.as_str(), to.as_str());
        *text = result.to_string();
    }
}

pub fn expand_env_string(text: &mut String) {
    // expand "$HOME/.local/share" to "/home/tom/.local/share"
    if !text.starts_with('$') {
        return;
    }
    let ptn = r"^\$([A-Za-z_][A-Za-z0-9_]*)";
    let mut env_value = String::new();
    match libs::re::find_first_group(ptn, &text) {
        Some(x) => {
            if let Ok(val) = env::var(&x) {
                env_value = val;
            }
        }
        None => {
            return;
        }
    }

    if env_value.is_empty() {
        return;
    }
    let t = text.clone();
    *text = libs::re::replace_all(&t, &ptn, &env_value);
}
