use std::env;
use libs;
use tools;

pub fn get_prompt(status: i32) -> String {
    let home = tools::get_user_home();
    let user = env::var("USER").unwrap();
    let hostname = tools::get_hostname();
    let _current_dir = env::current_dir().unwrap();
    let current_dir = _current_dir.to_str().unwrap();
    let _tokens: Vec<&str> = current_dir.split("/").collect();

    let last = _tokens.last().unwrap();
    let pwd: String;
    if last.to_string() == "" {
        pwd = String::from("/");
    } else if current_dir == home {
        pwd = String::from("~");
    } else {
        pwd = last.to_string();
    }

    let mut prompt = if status == 0 {
        format!("{}@{}: {}$ ",
                libs::colored::green(user.as_str()),
                libs::colored::green(hostname.as_str()),
                libs::colored::green(pwd.as_str()))
    } else {
        format!("{}@{}: {}$ ",
                libs::colored::red(user.as_str()),
                libs::colored::red(hostname.as_str()),
                libs::colored::red(pwd.as_str()))
    };
    match env::var("VIRTUAL_ENV") {
        Ok(x) => {
            if x != "" {
                let _tokens: Vec<&str> = x.split("/").collect();
                let env_name = _tokens.last().unwrap();
                prompt = format!("({}){}", libs::colored::green(env_name), prompt);
            }
        }
        Err(_) => {}
    }
    prompt
}
