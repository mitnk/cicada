use std::env;
use std::io;
use std::io::Write;
use std::process::Command;

fn main() {
    println!("Rust v0.1.0 Tell me what to do!");
    let user = env::var("USER").unwrap();
    loop {
        print!("{}@rush$ ", user);
        io::stdout().flush().unwrap();
        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd)
            .expect("failed to read line");
        if cmd.trim() == "exit" {
            println!("Bye.");
            break;
        } else if cmd.trim() == "" {
            continue;
        }
        // let args = cmd.trim().split(' ').collect();
        let args : Vec<&str> = cmd.trim().split(' ').collect();
        let output = Command::new(args[0])
            .args(&(args[1..]))
            .output()
            .expect("Failed to run Command");
        print!("{}", String::from_utf8(output.stdout).unwrap());
    }
}
