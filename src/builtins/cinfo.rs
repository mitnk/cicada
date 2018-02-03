use libs;

pub fn run(_tokens: &Vec<(String, String)>) -> i32 {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("Cicada Version: {}", VERSION);
    println!("Git Hash: {}", env!("GIT_HASH"));
    println!("Built at: {}", env!("BUILD_DATE"));
    let os_name = libs::os_type::get_os_name();
    println!("OS: {}", os_name);
    0
}
