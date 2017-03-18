use std::env;

pub fn run(args: Vec<String>, home: &str, current_dir: &str, previous_dir: &mut String) -> i32 {
    if args.len() > 2 {
        println!("invalid cd command");
        return 1;
    } else {
        let mut dir_to: String;
        if args.len() == 1 {
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
