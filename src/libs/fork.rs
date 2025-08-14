use nix::unistd::{fork as nix_fork, ForkResult};
use nix::Result;

// make fork "safe again", in order not to touch the code in core.rs,
// see https://github.com/nix-rust/nix/issues/586
// we can have refactorings any time needed.
pub fn fork() -> Result<ForkResult> {
    unsafe { nix_fork() }
}
