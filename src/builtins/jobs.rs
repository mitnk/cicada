use shell;
use jobc;

pub fn run(sh: &shell::Shell) -> i32 {
    if sh.jobs.is_empty() {
        return 0;
    }

    for (_i, job) in sh.jobs.iter() {
        jobc::print_job(job);
    }
    return 0;
}
