use crate::*;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;

pub fn gets(buf: &mut [u8], max: usize) {
    let mut i = 0;
    let mut c = 0u8;

    while i + 1 < max {
        let cc = read(0, &mut c as *mut u8, 1);
        if cc < 1 {
            break;
        }
        buf[i] = c;
        i += 1;
        if c == b'\n' || c == b'\r' {
            break;
        }
    }
    buf[i] = b'\0';
}

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
