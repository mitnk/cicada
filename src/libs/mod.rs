pub mod colored;
pub mod fork;
pub mod os_type;
pub mod path;
pub mod pipes;
pub mod progopts;
pub mod re;
pub mod term_size;

pub fn close(fd: i32) {
    unsafe {
        nix::libc::close(fd);
    }
}

pub fn dup(fd: i32) -> i32 {
    unsafe { nix::libc::dup(fd) }
}

pub fn dup2(src: i32, dst: i32) {
    unsafe {
        nix::libc::dup2(src, dst);
    }
}
