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
            match std::fs::OpenOptions::new().append(true).create(true).open(&log_file) {
                Ok(mut cfile) => {
                    let pid = $crate::tlog::getpid();
                    let now = $crate::ctime::DateTime::now();
                    let msg = format!("[{}][{}] {}", now, pid, msg);
                    let msg = if msg.ends_with('\n') { msg } else { format!("{}\n", msg) };
                    match cfile.write_all(msg.as_bytes()) {
                        Ok(_) => {}
                        Err(_) => println!("tlog: write_all error")
                    }
                }
                Err(_) => println!("tlog: open file error"),
            }

        }
    );

    ($fmt:expr, $($arg:tt)*) => (
        let msg = format!($fmt, $($arg)*);
        log!(&msg);
    );
}
