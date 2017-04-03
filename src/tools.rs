use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use libc;
use shlex;


pub fn rlog(s: String) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/cicada-debug.log")
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
    return format!("{}/.cicada/completers", home);
}

/// in docs of `linefeed::reader::Reader.set_prompt()`:
/// If prompt contains any terminal escape sequences, such escape sequences
/// should be immediately preceded by the character '\x01' and immediately
/// followed by the character '\x02'.
pub fn wrap_seq_chars(s: String) -> String {
    return format!("\x01{}\x02", s);
}

pub fn get_rc_file() -> String {
    let home = get_user_home();
    return format!("{}/{}", home, ".cicadarc");
}

pub fn unquote(s: &str) -> String {
    let args;
    if let Some(x) = shlex::split(s.trim()) {
        args = x;
    } else {
        return String::new();
    }
    if args.len() != 1 {
        return String::new();
    }
    return args[0].clone();
}
