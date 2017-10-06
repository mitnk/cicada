use os_type;

pub fn run(_tokens: &Vec<(String, String)>) -> i32 {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("Cicada Version: {}", VERSION);
    println!("Git Hash: {}", env!("GIT_HASH"));
    println!("Built at: {}", env!("BUILD_DATE"));
    let os = os_type::current_platform();
    println!("OS Type: {:?}", os.os_type);
    println!("OS Version: {}", os.version);
    0
}
