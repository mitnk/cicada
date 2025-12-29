use std::fs::{self, File};
use std::io::Read;

use crate::builtins::utils::{print_stderr_with_capture, print_stdout_with_capture};
use crate::libs;
use crate::shell::Shell;
use crate::tools;
use crate::types::{Command, CommandLine, CommandResult};

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command, capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();

    if cmd.tokens.len() != 2 {
        print_stderr_with_capture("check: usage: check <command>", &mut cr, cl, cmd, capture);
        cr.status = 1;
        return cr;
    }

    let name = &cmd.tokens[1].1;

    // Check alias
    if let Some(value) = sh.get_alias_content(name) {
        let mut out = format!("alias {}=\"{}\"", name, value);
        if let Some(base) = value.split_whitespace().next() {
            if tools::is_builtin(base) {
                out.push('\n');
                out.push_str("builtin");
            } else if let Some(info) = get_path_info(&find_path(base)) {
                out.push('\n');
                out.push_str(&info);
            }
        }
        print_stdout_with_capture(&out, &mut cr, cl, cmd, capture);
        return cr;
    }

    // Check builtin
    if tools::is_builtin(name) {
        print_stdout_with_capture("builtin", &mut cr, cl, cmd, capture);
        return cr;
    }

    // Check PATH
    if let Some(info) = get_path_info(&find_path(name)) {
        print_stdout_with_capture(&info, &mut cr, cl, cmd, capture);
    } else {
        let msg = format!("{}: not found", name);
        print_stderr_with_capture(&msg, &mut cr, cl, cmd, capture);
        cr.status = 1;
    }
    cr
}

fn find_path(cmd: &str) -> String {
    if cmd.contains('/') {
        if fs::metadata(cmd).is_ok() {
            cmd.to_string()
        } else {
            String::new()
        }
    } else {
        libs::path::find_file_in_path(cmd, true)
    }
}

fn get_path_info(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let mut out = format!("{}: {}", path, get_file_type(path));
    let is_link = fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    if is_link {
        if let Ok(real) = fs::canonicalize(path) {
            out.push_str(&format!("\nrealpath: {}", real.display()));
        }
    }
    Some(out)
}

fn get_file_type(path: &str) -> &'static str {
    let mut buf = [0u8; 512];
    let Ok(n) = File::open(path).and_then(|mut f| f.read(&mut buf)) else {
        return "cannot read";
    };
    if n == 0 {
        return "empty";
    }
    let b = &buf[..n];

    // Shebang
    if b.starts_with(b"#!") {
        return detect_script(b);
    }
    // ELF
    if b.starts_with(b"\x7FELF") {
        return match b.get(4) {
            Some(1) => "ELF 32-bit executable",
            Some(2) => "ELF 64-bit executable",
            _ => "ELF executable",
        };
    }
    // Mach-O (handle both endianness)
    if b.len() >= 4 {
        match [b[0], b[1], b[2], b[3]] {
            [0xfe, 0xed, 0xfa, 0xce] | [0xce, 0xfa, 0xed, 0xfe] => {
                return "Mach-O 32-bit executable"
            }
            [0xfe, 0xed, 0xfa, 0xcf] | [0xcf, 0xfa, 0xed, 0xfe] => {
                return "Mach-O 64-bit executable"
            }
            [0xca, 0xfe, 0xba, 0xbe] => return "Mach-O universal executable",
            _ => {}
        }
    }
    // PE
    if b.starts_with(b"MZ") {
        return "PE executable";
    }
    // Archives
    if b.starts_with(&[0x1f, 0x8b]) {
        return "gzip compressed data";
    }
    if b.starts_with(b"PK\x03\x04") {
        return "Zip archive";
    }
    if b.len() >= 262 && &b[257..262] == b"ustar" {
        return "POSIX tar archive";
    }

    if is_text(b) {
        "ASCII text"
    } else {
        "data"
    }
}

fn detect_script(b: &[u8]) -> &'static str {
    let end = b.iter().position(|&c| c == b'\n').unwrap_or(b.len());
    let line = String::from_utf8_lossy(b.get(2..end).unwrap_or_default());
    let mut parts = line.split_whitespace();
    let interp = parts.next().unwrap_or("");
    let s = if interp.ends_with("/env") {
        parts.next().unwrap_or("")
    } else {
        interp
    };

    match s {
        _ if s.contains("python") => "Python script",
        _ if s.contains("bash") => "Bash script",
        _ if s.contains("ruby") => "Ruby script",
        _ if s.contains("perl") => "Perl script",
        _ if s.contains("node") => "Node.js script",
        _ if s.contains("php") => "PHP script",
        _ if s.contains("lua") => "Lua script",
        _ if s.contains("zsh") => "Zsh script",
        _ if s.contains("fish") => "Fish script",
        _ if s.contains("awk") => "AWK script",
        _ if s.contains("sh") => "shell script",
        _ => "script",
    }
}

fn is_text(b: &[u8]) -> bool {
    b.iter()
        .all(|&c| c == b'\t' || c == b'\n' || c == b'\r' || (0x20..0x7f).contains(&c))
}
