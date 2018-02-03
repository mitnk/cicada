use execute;

pub fn get_os_name() -> String {
    let uname = get_uname();
    if uname.to_lowercase() == "darwin" {
        return get_macos_name();
    } else {
        return get_other_os_name();
    }
}

fn get_other_os_name() -> String {
    let mut name = get_release_value("PRETTY_NAME");
    if !name.is_empty() {
        return name;
    }
    name = get_release_value("DISTRIB_DESCRIPTION");
    if !name.is_empty() {
        return name;
    }
    String::new()
}

fn get_release_value(ptn: &str) -> String {
    let line = format!("grep -i '{}' /etc/*release* | grep -o '=.*' | tr '\"=' ' '", ptn);
    match execute::run(&line) {
        Ok(x) => {
            return x.stdout.trim().to_string();
        }
        Err(_) => {
            return String::new();
        }
    }
}

fn get_uname() -> String {
    match execute::run("uname") {
        Ok(x) => {
            return x.stdout.trim().to_string();
        }
        Err(_) => {
            return String::new();
        }
    }
}

fn get_macos_name() -> String {
    let mut os_name = String::from("macOS");
    let ver = get_osx_version();
    if !ver.is_empty() {
        os_name.push(' ');
        os_name.push_str(&ver);
    }
    os_name
}

fn get_osx_version() -> String {
    match execute::run("sw_vers -productVersion") {
        Ok(x) => {
            return x.stdout.trim().to_string();
        }
        Err(_) => {
            return String::new();
        }
    }
}
