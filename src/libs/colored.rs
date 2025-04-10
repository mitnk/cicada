// setting prompt in crate lineread needs wrap every SEQ chars
// with prefixing with '\x01' and suffix with '\x02'.
// Color Reference: https://misc.flogisoft.com/bash/tip_colors_and_formatting

// cicada special
pub const SEQ: &str = "\x01";
pub const END_SEQ: &str = "\x02";
pub const ESC: &str = "\x1B";

// Set
pub const BOLD: &str = "\x01\x1B[1m\x02";
pub const DIM: &str = "\x01\x1B[2m\x02";
pub const UNDERLINED: &str = "\x01\x1B[4m\x02";
pub const BLINK: &str = "\x01\x1B[5m\x02";
pub const REVERSE: &str = "\x01\x1B[7m\x02";
pub const HIDDEN: &str = "\x01\x1B[8m\x02";

// Reset
pub const RESET: &str = "\x01\x1B[0m\x02";
pub const RESET_BOLD: &str = "\x01\x1B[21m\x02";
pub const RESET_DIM: &str = "\x01\x1B[22m\x02";
pub const RESET_UNDERLINED: &str = "\x01\x1B[24m\x02";
pub const RESET_BLINK: &str = "\x01\x1B[25m\x02";
pub const RESET_REVERSE: &str = "\x01\x1B[27m\x02";
pub const RESET_HIDDEN: &str = "\x01\x1B[28m\x02";

// Foreground (text)
pub const DEFAULT: &str = "\x01\x1B[39m\x02";
pub const BLACK: &str = "\x01\x1B[30m\x02";
pub const RED: &str = "\x01\x1B[31m\x02";
pub const GREEN: &str = "\x01\x1B[32m\x02";
pub const YELLOW: &str = "\x01\x1B[33m\x02";
pub const BLUE: &str = "\x01\x1B[34m\x02";
pub const MAGENTA: &str = "\x01\x1B[35m\x02";
pub const CYAN: &str = "\x01\x1B[36m\x02";
pub const GRAY_L: &str = "\x01\x1B[37m\x02";

pub const GRAY_D: &str = "\x01\x1B[90m\x02";
pub const RED_L: &str = "\x01\x1B[91m\x02";
pub const GREEN_L: &str = "\x01\x1B[92m\x02";
pub const YELLOW_L: &str = "\x01\x1B[93m\x02";
pub const BLUE_L: &str = "\x01\x1B[94m\x02";
pub const MAGENTA_L: &str = "\x01\x1B[95m\x02";
pub const CYAN_L: &str = "\x01\x1B[96m\x02";
pub const WHITE: &str = "\x01\x1B[97m\x02";

pub const BLUE_B: &str = "\x01\x1B[34m\x1B[1m\x02";
pub const BLACK_B: &str = "\x01\x1B[30m\x1B[1m\x02";
pub const WHITE_B: &str = "\x01\x1B[97m\x1B[1m\x02";
pub const RED_B: &str = "\x01\x1B[31m\x1B[1m\x02";
pub const GREEN_B: &str = "\x01\x1B[32m\x1B[1m\x02";

// Background
pub const DEFAULT_BG: &str = "\x01\x1B[49m\x02";
pub const BLACK_BG: &str   = "\x01\x1B[40m\x02";
pub const RED_BG: &str     = "\x01\x1B[41m\x02";
pub const GREEN_BG: &str   = "\x01\x1B[42m\x02";
pub const YELLOW_BG: &str   = "\x01\x1B[43m\x02";
pub const BLUE_BG: &str    = "\x01\x1B[44m\x02";
pub const MAGENTA_BG: &str    = "\x01\x1B[45m\x02";
pub const CYAN_BG: &str    = "\x01\x1B[46m\x02";
pub const GRAY_L_BG: &str    = "\x01\x1B[47m\x02";

pub const GRAY_D_BG: &str   = "\x01\x1B[100m\x02";
pub const RED_L_BG: &str   = "\x01\x1B[101m\x02";
pub const GREEN_L_BG: &str   = "\x01\x1B[102m\x02";
pub const YELLOW_L_BG: &str   = "\x01\x1B[103m\x02";
pub const BLUE_L_BG: &str   = "\x01\x1B[104m\x02";
pub const MAGENTA_L_BG: &str   = "\x01\x1B[105m\x02";
pub const CYAN_L_BG: &str   = "\x01\x1B[106m\x02";
pub const WHITE_BG: &str   = "\x01\x1B[107m\x02";
