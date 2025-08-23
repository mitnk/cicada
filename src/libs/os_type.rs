use crate::execute;

pub fn get_os_name() -> String {
    let uname = get_uname();
    if uname.to_lowercase() == "darwin" {
        get_macos_name()
    } else {
        get_other_os_name()
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
    name = get_release_value("IMAGE_DESCRIPTION");
    if !name.is_empty() {
        return name;
    }
    get_uname_mo()
}

fn get_release_value(ptn: &str) -> String {
    let line = format!(
        "grep -i '{}' /etc/*release* 2>&1 | grep -o '=.*' | tr '\"=' ' '",
        ptn
    );
    let cr = execute::run(&line);
    cr.stdout.trim().to_string()
}

fn get_uname() -> String {
    let cr = execute::run("uname");
    cr.stdout.trim().to_string()
}

fn get_uname_mo() -> String {
    let cr = execute::run("uname -m -o");
    cr.stdout.trim().to_string()
}

fn get_macos_name() -> String {
    let mut os_name = get_osx_codename();
    let ver = get_osx_version();
    if !ver.is_empty() {
        os_name.push(' ');
        os_name.push_str(&ver);
    }
    os_name
}

fn get_osx_codename() -> String {
    let cr = execute::run("grep -o 'SOFTWARE LICENSE AGREEMENT FOR .*[a-zA-Z]' '/System/Library/CoreServices/Setup Assistant.app/Contents/Resources/en.lproj/OSXSoftwareLicense.rtf' | sed 's/SOFTWARE LICENSE AGREEMENT FOR *//'");
    cr.stdout.trim().to_string()
}

fn get_osx_version() -> String {
    let cr = execute::run("sw_vers -productVersion");
    cr.stdout.trim().to_string()
}
