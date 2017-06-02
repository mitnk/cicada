use std::env;
use shell;
use tools;

#[allow(needless_pass_by_value)]
pub fn run(sh: &mut shell::Shell, args: Vec<String>) -> i32 {
    if args.len() > 2 {
        println!("invalid cd command");
        return 1;
    }
    let mut dir_to: String;
    let _current_dir = env::current_dir().expect("cd: get current_dir error");
    let current_dir = _current_dir.to_str().expect("cd: to_str error");
    if args.len() == 1 {
        let home = tools::get_user_home();
        dir_to = home.to_string();
    } else {
        dir_to = args[1..].join("");
    }
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
            println!("{:?}", e);
            1
        }
    }
}
