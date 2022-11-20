pub fn getpid() -> i32 {
    unsafe { libc::getpid() }
}

#[macro_export]
macro_rules! log {
    ($fmt:expr) => (
        let log_file = if let Ok(x) = std::env::var("CICADA_LOG_FILE") {
            x.clone()
        } else {
            String::new()
        };

        if !log_file.is_empty() {
            use std::io::Write as _;

            let msg = $fmt;
            let mut cfile;
            match std::fs::OpenOptions::new().append(true).create(true).open(&log_file) {
                Ok(x) => cfile = x,
                Err(_) => panic!("tlog: open file error"),
            }

            let pid = crate::tlog::getpid();
            let now = crate::ctime::DateTime::now();
            let msg = format!("[{}][{}] {}", now, pid, msg);
            let msg = if msg.ends_with('\n') { msg } else { format!("{}\n", msg) };
            match cfile.write_all(msg.as_bytes()) {
                Ok(_) => {}
                Err(_) => panic!("tlog: write_all error")
            }
        }
    );

    ($fmt:expr, $($arg:tt)*) => (
        let msg = format!($fmt, $($arg)*);
        log!(&msg);
    );
}
