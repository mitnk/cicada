// setting prompt in crate linefeed needs wrap every SEQ chars
// with prefixing with '\x01' and suffix with '\x02'.
pub const BLUE: &str = "\x01\x1B[34m\x02";
pub const RED: &str = "\x01\x1B[31m\x02";
pub const GREEN: &str = "\x01\x1B[32m\x02";
pub const RESET: &str = "\x01\x1B[0m\x02";
