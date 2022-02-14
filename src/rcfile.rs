use std::path::Path;

use crate::scripting;
use crate::shell;
use crate::tools;

pub fn get_rc_file() -> String {
    let dir_config = tools::get_config_dir();
    let rc_file = format!("{}/cicadarc", dir_config);
    if Path::new(&rc_file).exists() {
        return rc_file;
    }

    // fail back to $HOME/.cicadarc
    let home = tools::get_user_home();
    let rc_file_home = format!("{}/{}", home, ".cicadarc");
    if Path::new(&rc_file_home).exists() {
        return rc_file_home;
    }

    // use std path if both absent
    rc_file
}

pub fn load_rc_files(sh: &mut shell::Shell) {
    let rc_file = get_rc_file();
    if !Path::new(&rc_file).exists() {
        return;
    }

    let args = vec!["source".to_string(), rc_file];
    scripting::run_script(sh, &args);
}
