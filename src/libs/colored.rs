// setting prompt in crate linefeed needs wrap every SEQ chars
// with prefixing with '\x01' and suffix with '\x02'.
const RED: &str = "\x01\x1B[31m\x02";
const GREEN: &str = "\x01\x1B[32m\x02";
const RESET: &str = "\x01\x1B[0m\x02";

pub fn green(s: &str) -> String {
    return format!("{}{}{}", GREEN, s, RESET);
}

pub fn red(s: &str) -> String {
    return format!("{}{}{}", RED, s, RESET);
}
