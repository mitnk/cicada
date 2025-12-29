pub mod colored;
pub mod fork;
pub mod os_type;
pub mod path;
pub mod pipes;
pub mod prefix;
pub mod progopts;
pub mod re;
pub mod term_size;

pub fn close(fd: i32) {
    unsafe {
        libc::close(fd);
    }
}

pub fn dup(fd: i32) -> i32 {
    unsafe { libc::dup(fd) }
}

pub fn dup2(src: i32, dst: i32) {
    unsafe {
        libc::dup2(src, dst);
    }
}
