use errno::errno;
use libc;
pub use tools::rlog;
use std::mem;


pub unsafe fn give_terminal_to(pid: i32) {
    let mut mask: libc::sigset_t = mem::zeroed();
    let mut old_mask: libc::sigset_t = mem::zeroed();

    libc::sigemptyset(&mut mask);
    libc::sigaddset(&mut mask, libc::SIGTSTP);
    libc::sigaddset(&mut mask, libc::SIGTTIN);
    libc::sigaddset(&mut mask, libc::SIGTTOU);
    libc::sigaddset(&mut mask, libc::SIGCHLD);

    let rcode = libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut old_mask);
    if rcode != 0 {
        rlog(format!("failed to call pthread_sigmask\n"));
    }
    let rcode = libc::tcsetpgrp(1, pid);
    if rcode == -1 {
        let e = errno();
        let code = e.0;
        rlog(format!("Error {}: {}\n", code, e));
    } else {
        rlog(format!("return term back to {} rcode: {}\n", pid, rcode));
    }
    let rcode = libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, &mut mask);
    if rcode != 0 {
        rlog(format!("failed to call pthread_sigmask\n"));
    }
}
