use std::env::var_os;
use std::path::PathBuf;

use dirs::data_dir;

pub fn env_init_file() -> Option<PathBuf> {
    var_os("INPUTRC").map(PathBuf::from)
}

pub fn system_init_file() -> Option<PathBuf> {
    None
}

pub fn user_init_file() -> Option<PathBuf> {
    data_dir().map(|p| p.join(r"linefeed\inputrc"))
}
