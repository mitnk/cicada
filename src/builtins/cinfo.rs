use crate::history;
use crate::libs;

pub fn run() -> i32 {
    let mut info = vec![];
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    info.push(("version", VERSION));
    info.push(("rustc-version", env!("BUILD_RUSTC_VERSION")));

    let os_name = libs::os_type::get_os_name();
    info.push(("os-name", &os_name));

    let hfile = history::get_history_file();
    info.push(("history-file", &hfile));

    let git_hash = env!("GIT_HASH");
    if !git_hash.is_empty() {
        info.push(("git-commit", env!("GIT_HASH")));
    }

    let git_branch = env!("GIT_BRANCH");
    let mut branch = String::new();
    if !git_branch.is_empty() {
        branch.push_str(git_branch);
        let git_status = env!("GIT_STATUS");
        if git_status != "0" {
            branch.push_str(" (dirty)");
        }
        info.push(("git-branch", &branch));
    }

    info.push(("built-at", env!("BUILD_DATE")));

    for (k, v) in &info {
        // longest key above is 13-char length
        println!("{: >13}: {}", k, v);
    }
    0
}
