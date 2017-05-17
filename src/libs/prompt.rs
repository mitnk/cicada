use std::env;
use libs;
use tools;

pub fn get_prompt(status: i32) -> String {
    let home = tools::get_user_home();
    let user = env::var("USER").expect("cicada: env USER error");
    let hostname = tools::get_hostname();
    let _current_dir = env::current_dir().expect("cicada: env current_dir error");
    let current_dir = _current_dir.to_str().expect("cicada: to_str error");
    let _tokens: Vec<&str> = current_dir.split('/').collect();

    let last = _tokens.last().expect("cicada: prompt token last error");
    let pwd: String;
    if last.is_empty() {
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
    if let Ok(x) = env::var("VIRTUAL_ENV") {
        if x != "" {
            let _tokens: Vec<&str> = x.split('/').collect();
            let env_name = _tokens.last().expect("prompt token last error");
            prompt = format!("({}){}", libs::colored::green(env_name), prompt);
        }
    }
    prompt
}
