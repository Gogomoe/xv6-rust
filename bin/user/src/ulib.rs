use crate::*;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;

pub fn stat(n: *const u8, st: &mut FileStatus) -> isize {
    let path = unsafe { from_utf8_unchecked(from_raw_parts(n, strlen(n))) };
    let fd = open(path, OPEN_READ_ONLY);
    if fd < 0 {
        return -1;
    }
    let fd = fd as usize;
    let r = fstat(fd, st as *mut FileStatus);
    close(fd);
    return r;
}

pub fn strlen(s: *const u8) -> usize {
    let mut n = 0;
    while unsafe { *(s.add(n)) } != b'\0' {
        n += 1;
    }
    n
}
