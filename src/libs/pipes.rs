// via: nix v0.26.4. We do not want to use OwnedFd in newer version nix.
use std::os::fd::RawFd;
use nix::Error;
use std::mem;
use libc::c_int;

pub fn pipe() -> std::result::Result<(RawFd, RawFd), Error> {
    let mut fds = mem::MaybeUninit::<[c_int; 2]>::uninit();
    let res = unsafe { libc::pipe(fds.as_mut_ptr() as *mut c_int) };
    Error::result(res)?;
    unsafe { Ok((fds.assume_init()[0], fds.assume_init()[1])) }
}
