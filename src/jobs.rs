use errno::errno;
use libc;
use std::mem;
use tools::rlog;


pub unsafe fn give_terminal_to(gid: i32) -> bool {
    let mut mask: libc::sigset_t = mem::zeroed();
    let mut old_mask: libc::sigset_t = mem::zeroed();

    libc::sigemptyset(&mut mask);
    libc::sigaddset(&mut mask, libc::SIGTSTP);
    libc::sigaddset(&mut mask, libc::SIGTTIN);
    libc::sigaddset(&mut mask, libc::SIGTTOU);
    libc::sigaddset(&mut mask, libc::SIGCHLD);

    let rcode = libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut old_mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    let rcode = libc::tcsetpgrp(1, gid);
    let given;
    if rcode == -1 {
        given = false;
        let e = errno();
        let code = e.0;
        log!("Error {}: {}", code, e);
    } else {
        given = true;
    }
    let rcode = libc::pthread_sigmask(libc::SIG_SETMASK, &old_mask, &mut mask);
    if rcode != 0 {
        log!("failed to call pthread_sigmask");
    }
    given
}
