use std::env;
use std::path::Path;

use crate::scripting;
use crate::shell;

pub fn load_rc_files(sh: &mut shell::Shell) {
    // make "/usr/local/bin" as the first item in PATH
    if let Ok(env_path) = env::var("PATH") {
        if !env_path.contains("/usr/local/bin:") {
            let env_path_new = format!("/usr/local/bin:{}", env_path);
            env::set_var("PATH", &env_path_new);
        }
    }

    let rc_file = shell::get_rc_file();
    if !Path::new(&rc_file).exists() {
        return;
    }

    let args = vec!["source".to_string(), rc_file];
    scripting::run_script(sh, &args);
}
