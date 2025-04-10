use std::env::var_os;
use std::path::PathBuf;

use dirs::home_dir;

pub fn env_init_file() -> Option<PathBuf> {
    var_os("INPUTRC").map(PathBuf::from)
}

pub fn system_init_file() -> Option<PathBuf> {
    Some(PathBuf::from("/etc/inputrc"))
}

pub fn user_init_file() -> Option<PathBuf> {
    home_dir().map(|p| p.join(".inputrc"))
}
