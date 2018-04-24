use libs;

pub fn run(_tokens: &Vec<(String, String)>) -> i32 {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("Cicada Version: {}", VERSION);
    println!("Commit: {}", env!("GIT_HASH"));
    let os_name = libs::os_type::get_os_name();
    println!("OS: {}", os_name);
    println!("Built with: {}", env!("BUILD_RUSTC_VERSION"));
    println!("Built at: {}", env!("BUILD_DATE"));
    0
}
