use os_type;

pub fn run(_tokens: &Vec<(String, String)>) -> i32 {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("Cicada Version: {}", VERSION);
    let os = os_type::current_platform();
    println!("OS Type: {:?}", os.os_type);
    println!("OS Version: {}", os.version);
    0
}
