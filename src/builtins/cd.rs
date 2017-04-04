use std::env;
use shlex;
use tools;

pub fn run(line: String, previous_dir:&mut String) -> i32 {
    let args;
    if let Some(x) = shlex::split(line.trim()) {
        args = x;
    } else {
        println!("shlex split error: does not support multiple line");
        return 1;
    }
    if args.len() > 2 {
        println!("invalid cd command");
        return 1;
    } else {
        let mut dir_to: String;
        let _current_dir = env::current_dir().unwrap();
        let current_dir = _current_dir.to_str().unwrap();
        if args.len() == 1 {
            let home = tools::get_user_home();
            dir_to = home.to_string();
        } else {
            dir_to = args[1..].join("");
        }
        if dir_to == "-" {
            if previous_dir == "" {
                println!("no previous dir");
                return 0;
            }
            dir_to = previous_dir.clone();
        } else {
            if !dir_to.starts_with("/") {
                dir_to = format!("{}/{}", current_dir.to_string(), dir_to);
            }
        }
        if current_dir != dir_to {
            *previous_dir = current_dir.to_string();
        }
        match env::set_current_dir(&dir_to) {
            Ok(_) => {
                return 0;
            }
            Err(e) => {
                println!("{:?}", e);
                return 1;
            }
        }
    }
}
