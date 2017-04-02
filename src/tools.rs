use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use libc;


pub fn rlog(s: String) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/mtsh-debug.log")
        .unwrap();
    let pid = unsafe { libc::getpid() };
    let s = format!("[{}] {}", pid, s);
    file.write_all(s.as_bytes()).unwrap();
}

pub fn get_user_home() -> String {
    let home = env::var("HOME").unwrap();
    return home;
}

pub fn get_user_completer_dir() -> String {
    let home = get_user_home();
    return format!("{}/.mtsh/completers", home);
}
