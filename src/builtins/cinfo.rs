use libs;

pub fn run(_tokens: &Vec<(String, String)>) -> i32 {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    println!("Cicada Version: {}", VERSION);

    let git_hash = env!("GIT_HASH");
    if !git_hash.is_empty() {
        println!("Git Commit: {}", env!("GIT_HASH"));
    }
    let git_branch = env!("GIT_BRANCH");
    if !git_branch.is_empty() {
        println!("Git Branch: {}", env!("GIT_BRANCH"));
    }

    let os_name = libs::os_type::get_os_name();
    println!("OS: {}", os_name);
    println!("Built with: {}", env!("BUILD_RUSTC_VERSION"));
    println!("Built at: {}", env!("BUILD_DATE"));
    0
}
