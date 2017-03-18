use std::fs::OpenOptions;
use std::io::Write;


pub fn rlog(s: String) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/rush-debug.log")
        .unwrap();
    file.write_all(s.as_bytes()).unwrap();
}
