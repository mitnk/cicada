use nix::sys::signal;
use nix::sys::wait::waitpid;

use tools::clog;

extern "C" fn handle_sigchld(_: i32) {
    match waitpid(None, None) {
        Ok(x) => {
            log!("waitpid ok: {:?}", x);
        }
        Err(e) => {
            log!("waitpid error: {:?}", e);
        }
    }
}

pub fn set_sigchld_handler() {
    let sig_action = signal::SigAction::new(
        signal::SigHandler::Handler(handle_sigchld),
        signal::SaFlags::empty(),
        signal::SigSet::empty(),
    );
    unsafe {
        match signal::sigaction(signal::SIGCHLD, &sig_action) {
            Ok(_) => {}
            Err(e) => log!("sigaction error: {:?}", e),
        }
    }
}
