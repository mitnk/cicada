pub fn is_login(args: &Vec<String>) -> bool {
    if args.len() > 0 && args[0].starts_with("-") {
        return true;
    }

    if args.len() > 1 && (args[1] == "--login" || args[1] == "-l") {
        return true;
    }

    false
}

pub fn is_script(args: &Vec<String>) -> bool {
    args.len() > 1 && !args[1].starts_with("-")
}

pub fn is_command_string(args: &Vec<String>) -> bool {
    args.len() > 1 && args[1] == "-c"
}

pub fn is_non_tty() -> bool {
    unsafe { libc::isatty(0) == 0 }
}
