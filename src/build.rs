extern crate time;
use std::process::Command;

fn main() {
    match Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        Ok(x) => {
            let git_hash = String::from_utf8_lossy(&x.stdout);
            println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        }
        Err(_) => {
            println!("cargo:rustc-env=GIT_HASH=");
        }
    }

    match Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        Ok(x) => {
            let git_branch = String::from_utf8_lossy(&x.stdout);
            println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
        }
        Err(_) => {
            println!("cargo:rustc-env=GIT_BRANCH=");
        }
    }

    match Command::new("git")
        .args(&["status", "--porcelain"])
        .output()
    {
        Ok(x) => {
            let git_status = String::from_utf8_lossy(&x.stdout);
            println!("cargo:rustc-env=GIT_STATUS={}", git_status.len());
        }
        Err(_) => {
            println!("cargo:rustc-env=GIT_STATUS=0");
        }
    }

    match Command::new("rustc").args(&["-V"]).output() {
        Ok(x) => {
            let output = String::from_utf8_lossy(&x.stdout);
            println!("cargo:rustc-env=BUILD_RUSTC_VERSION={}", output);
        }
        Err(_) => {
            println!("cargo:rustc-env=BUILD_RUSTC_VERSION=");
        }
    }

    let tm = time::now();
    println!("cargo:rustc-env=BUILD_DATE={}", tm.rfc822());
}
