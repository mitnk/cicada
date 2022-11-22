use crate::builtins::utils::print_stdout_with_capture;
use crate::jobc;
use crate::shell::Shell;
use crate::types::{CommandResult, CommandLine, Command};

pub fn run(sh: &mut Shell, cl: &CommandLine, cmd: &Command,
           capture: bool) -> CommandResult {
    let mut cr = CommandResult::new();
    if sh.jobs.is_empty() {
        return cr;
    }

    // update status of jobs if any
    jobc::try_wait_bg_jobs(sh, false, false);

    let mut lines = Vec::new();
    let jobs = sh.jobs.clone();
    let no_trim = cmd.tokens.len() >= 2 && cmd.tokens[1].1 == "-f";
    for (_i, job) in jobs.iter() {
        let line = jobc::get_job_line(job, !no_trim);
        lines.push(line);
    }
    let buffer = lines.join("\n");

    print_stdout_with_capture(&buffer, &mut cr, cl, cmd, capture);
    cr
}
