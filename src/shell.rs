use std::collections::HashMap;
use tools;

pub struct Shell {
    pub alias: HashMap<String, String>,
    pub previous_dir: String,
    pub previous_cmd: String,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            alias: HashMap::new(),
            previous_dir: String::new(),
            previous_cmd: String::new(),
        }
    }

    pub fn add_alias(&mut self, name: &str, value: &str) {
        self.alias.insert(name.to_string(), value.to_string());
    }

    pub fn extend_alias(&mut self, name: &str) -> String {
        let mut result;
        match self.alias.get(name) {
            Some(x) => {
                result = x.to_string();
            }
            None => {
                result = name.to_string();
            }
        }
        tools::pre_handle_cmd_line(&mut result);
        result
    }
}
