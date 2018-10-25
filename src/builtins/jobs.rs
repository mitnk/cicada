use shell;

pub fn run(sh: &shell::Shell) -> i32 {
    if sh.jobs.is_empty() {
        return 0;
    }

    for (gid, _) in sh.jobs.iter() {
        println!("[{}] Status-todo    cmd line todo", gid);
    }
    return 0;
}
