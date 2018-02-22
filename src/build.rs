extern crate time;
use std::process::Command;

fn main() {
    match Command::new("git").args(&["rev-parse", "--short", "HEAD"]).output() {
        Ok(x) => {
            let git_hash = String::from_utf8_lossy(&x.stdout);
            println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        }
        Err(e) => {
            println!("cargo:rustc-env=GIT_HASH={:?}", e);
        }
    }
    let tm = time::now();
    println!("cargo:rustc-env=BUILD_DATE={}", tm.rfc822());
}
