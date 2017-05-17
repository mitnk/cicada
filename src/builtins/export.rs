use std::env;
use regex::Regex;
use tools;

pub fn run(line: &str) -> i32 {
    if !tools::is_env(line) {
        return 1;
    }

    let re = Regex::new(r"^ *export *([a-zA-Z0-9_\.-]+)=(.*)$").expect("export: Regex error");
    for cap in re.captures_iter(line) {
        let value = tools::unquote(&cap[2]);
        env::set_var(&cap[1], &value);
    }
    0
}
