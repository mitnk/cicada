use std::env;
use std::error::Error;
use shell;
use tools;

#[allow(needless_pass_by_value)]
pub fn run(sh: &mut shell::Shell, args: Vec<String>) -> i32 {
    if args.len() > 2 {
        println!("invalid cd command");
        return 1;
    }
    let _current_dir;
    match env::current_dir() {
        Ok(x) => _current_dir = x,
        Err(e) => {
            println!("current_dir() failed: {:?}", e);
            return 1;
        }
    }
    let current_dir;
    match _current_dir.to_str() {
        Some(x) => current_dir = x,
        None => {
            println!("current dir is None?");
            return 1;
        }
    }
    let mut dir_to = if args.len() == 1 {
        let home = tools::get_user_home();
        home.to_string()
    } else {
        args[1..].join("")
    };

    if dir_to == "-" {
        if sh.previous_dir == "" {
            println!("no previous dir");
            return 0;
        }
        dir_to = sh.previous_dir.clone();
    } else if !dir_to.starts_with('/') {
        dir_to = format!("{}/{}", current_dir.to_string(), dir_to);
    }
    if current_dir != dir_to {
        sh.previous_dir = current_dir.to_string();
    }
    match env::set_current_dir(&dir_to) {
        Ok(_) => 0,
        Err(e) => {
            println!("cd: {}", e.description());
            1
        }
    }
}
