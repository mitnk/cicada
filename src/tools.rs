use std::fs::OpenOptions;
use std::io::Write;
use libc;


pub fn rlog(s: String) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/rush-debug.log")
        .unwrap();
    let pid = unsafe { libc::getpid() };
    let s = format!("[{}] {}", pid, s);
    file.write_all(s.as_bytes()).unwrap();
}
