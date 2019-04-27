use regex;

pub fn find_first_group(ptn: &str, text: &str) -> Option<String> {
    let re;
    match regex::Regex::new(ptn) {
        Ok(x) => re = x,
        Err(_) => return None,
    }
    match re.captures(text) {
        Some(caps) => {
            if let Some(x) = caps.get(1) {
                return Some(x.as_str().to_owned());
            }
        }
        None => {
            return None;
        }
    }
    None
}

pub fn re_contains(text: &str, ptn: &str) -> bool {
    let re;
    match regex::Regex::new(ptn) {
        Ok(x) => {
            re = x;
        }
        Err(e) => {
            println!("Regex new error: {:?}", e);
            return false;
        }
    }
    re.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::find_first_group;

    #[test]
    fn test_find_first_group() {
        let s = find_first_group(r"", "");
        assert_eq!(s, None);

        let s = find_first_group(r"abc", "123");
        assert_eq!(s, None);

        let s = find_first_group(r"\$\((.+)\)", "ls -l $(find x) -h");
        assert_eq!(s, Some("find x".to_string()));

        let s = find_first_group(r"(\d+)-(\d+)-(\d+)", "2017-09-16");
        assert_eq!(s, Some("2017".to_string()));
    }
}
