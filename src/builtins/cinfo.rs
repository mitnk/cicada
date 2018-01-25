use execute;

pub fn run(_tokens: &Vec<(String, String)>) -> i32 {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("Cicada Version: {}", VERSION);
    println!("Git Hash: {}", env!("GIT_HASH"));
    println!("Built at: {}", env!("BUILD_DATE"));
    match execute::run("grep -i DISTRIB_DESCRIPTION /etc/*release*") {
        Ok(x) => {
            println!("OS: {}", x.stdout);
        }
        Err(e) => {
            println!("OS: {:?}", e);
        }
    }
    0
}
