// setting prompt in crate linefeed needs wrap every SEQ chars
// with prefixing with '\x01' and suffix with '\x02'.
// todo full list - https://misc.flogisoft.com/bash/tip_colors_and_formatting
pub const RESET: &str = "\x01\x1B[0m\x02";
pub const BOLD: &str = "\x01\x1B[1m\x02";
pub const UNDERLINED: &str = "\x01\x1B[4m\x02";

pub const BLUE: &str = "\x01\x1B[34m\x02";
pub const BLACK: &str = "\x01\x1B[30m\x02";
pub const WHITE: &str = "\x01\x1B[97m\x02";
pub const RED: &str = "\x01\x1B[31m\x02";
pub const GREEN: &str = "\x01\x1B[32m\x02";

pub const BLUE_B: &str = "\x01\x1B[34m\x1B[1m\x02";
pub const BLACK_B: &str = "\x01\x1B[30m\x1B[1m\x02";
pub const WHITE_B: &str = "\x01\x1B[97m\x1B[1m\x02";
pub const RED_B: &str = "\x01\x1B[31m\x1B[1m\x02";
pub const GREEN_B: &str = "\x01\x1B[32m\x1B[1m\x02";

pub const BLUE_BG: &str = "\x01\x1B[44m\x02";
pub const BLACK_BG: &str = "\x01\x1B[40m\x02";
pub const WHITE_BG: &str = "\x01\x1B[107m\x02";
pub const RED_BG: &str = "\x01\x1B[41m\x02";
pub const GREEN_BG: &str = "\x01\x1B[42m\x02";
