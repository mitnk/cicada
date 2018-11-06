use std::env;
use std::fs::read_dir;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

pub fn find_first_exec(filename: &str) -> String {
    let env_path;
    match env::var("PATH") {
        Ok(x) => env_path = x,
        Err(e) => {
            println_stderr!("cicada: error in env:var(): {:?}", e);
            return String::new();
        }
    }
    let vec_path: Vec<&str> = env_path.split(':').collect();
    for p in &vec_path {
        if let Ok(list) = read_dir(p) {
            for entry in list {
                if let Ok(entry) = entry {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name != filename {
                            continue;
                        }
                        let _mode;
                        match entry.metadata() {
                            Ok(x) => _mode = x,
                            Err(e) => {
                                println_stderr!("cicada: metadata error: {:?}", e);
                                continue;
                            }
                        }
                        let mode = _mode.permissions().mode();
                        if mode & 0o111 == 0 {
                            // not binary
                            continue;
                        }
                        return entry.path().to_string_lossy().to_string();
                    }
                }
            }
        }
    }
    String::new()
}
