use shell;

pub fn cleanup_process_groups(sh: &mut shell::Shell, pgid: i32, pid: i32) {
    let mut empty_pids = false;
    if let Some(x) = sh.pgs.get_mut(&pgid) {
        if let Ok(i) = x.binary_search(&pid) {
            x.remove(i);
        }
        empty_pids = x.is_empty();
    }

    if empty_pids {
        sh.pgs.remove(&pgid);
    }
}
