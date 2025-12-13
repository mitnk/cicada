pub fn is_login(args: &[String]) -> bool {
    if !args.is_empty() && args[0].starts_with("-") {
        return true;
    }

    if args.len() > 1 && (args[1] == "--login" || args[1] == "-l") {
        return true;
    }

    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        if term_program == "vscode" {
            return true;
        }
    }

    false
}

pub fn is_script(args: &[String]) -> bool {
    args.len() > 1 && !args[1].starts_with("-")
}

pub fn is_command_string(args: &[String]) -> bool {
    args.len() > 1 && args[1] == "-c"
}

pub fn is_non_tty() -> bool {
    unsafe { nix::libc::isatty(0) == 0 }
}
