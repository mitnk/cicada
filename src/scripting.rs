use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::execute;
use crate::libs;
use crate::shell;

pub fn run_script(sh: &mut shell::Shell, args: &Vec<String>) -> i32 {
    let mut status = 0;

    let src_file = &args[1];
    let full_src_file: String;
    if src_file.contains('/') {
        full_src_file = src_file.clone();
    } else {
        let full_path = libs::path::find_file_in_path(src_file, false);
        if full_path.is_empty() {
            // not in PATH and not in current work directory
            if !Path::new(src_file).exists() {
                println_stderr!("cicada: {}: no such file", src_file);
                return 1;
            }
            full_src_file = format!("./{}", src_file);
        } else {
            full_src_file = full_path.clone();
        }
    }

    if !Path::new(&full_src_file).exists() {
        println_stderr!("cicada: {}: no such file", src_file);
        return 1;
    }

    let mut file;
    match File::open(&full_src_file) {
        Ok(x) => file = x,
        Err(e) => {
            println_stderr!("cicada: open script file err: {:?}", e);
            return 1;
        }
    }
    let mut text = String::new();
    match file.read_to_string(&mut text) {
        Ok(_) => {}
        Err(e) => {
            println_stderr!("cicada: read_to_string error: {:?}", e);
            return 1;
        }
    }
    for line in text.lines() {
        if line.trim().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        status = execute::run_procs(sh, line, true);
    }

    status
}
