#![no_std]
#![no_main]

use core::mem::size_of;
use core::ptr;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;

use file_system_lib::{Dirent, FileStatus, DIRECTORY_SIZE, TYPE_DEVICE, TYPE_DIR, TYPE_FILE};
use user::*;

fn fmtname(path: &str) -> &str {
    #[allow(non_upper_case_globals)]
    static mut buf: [u8; DIRECTORY_SIZE] = [0u8; DIRECTORY_SIZE];
    let mut p: *const u8;
    unsafe {
        p = path.as_ptr().add(path.len() - 1);
        while p >= path.as_ptr() && *p != b'/' {
            p = p.sub(1);
        }
        p = p.add(1);
    }

    let length = path.len() - (p as usize - path.as_ptr() as usize);
    unsafe {
        if length >= DIRECTORY_SIZE {
            from_utf8_unchecked(from_raw_parts(p, length))
        } else {
            ptr::copy(p, buf.as_mut_ptr(), length);
            let mut p: *mut u8 = buf.as_mut_ptr().add(length);
            while p < buf.as_mut_ptr().add(buf.len()) {
                *p = b' ';
                p = p.add(1);
            }
            from_utf8_unchecked(from_raw_parts(buf.as_ptr(), buf.len()))
        }
    }
}

fn ls(path: &str) {
    let mut buf = [0u8; 512];
    let mut de = Dirent::new();
    let mut st = FileStatus::new();

    let fd = open(path, OPEN_READ_ONLY);
    if fd < 0 {
        fprintln!(2, "ls: cannot open {}.", path);
        return;
    }
    let fd = fd as usize;

    if fstat(fd, &mut st) < 0 {
        fprintln!(2, "ls: cannot stat {}", path);
        close(fd);
        return;
    }

    match st.types {
        TYPE_FILE => println!("{} {} {} {}", fmtname(path), st.types, st.ino, st.size),
        TYPE_DIR => {
            if path.len() + 1 + DIRECTORY_SIZE + 1 > size_of::<[u8; 512]>() {
                println!("ls: path too long");
            } else {
                let mut p: *mut u8;
                unsafe {
                    ptr::copy(path.as_ptr(), buf.as_mut_ptr(), path.len());
                    p = (buf.as_mut_ptr() as *mut u8).add(path.len());
                    *p = b'/';
                    p = p.add(1);
                }

                while read(fd, &mut de as *mut _ as *mut u8, size_of::<Dirent>())
                    == size_of::<Dirent>() as isize
                {
                    if de.inum != 0 {
                        unsafe {
                            ptr::copy(de.name.as_ptr(), p, DIRECTORY_SIZE);
                            *(p.add(DIRECTORY_SIZE)) = 0;
                        }
                        if stat(buf.as_ptr(), &mut st) < 0 {
                            println!("ls: cannot stat {}", unsafe { from_utf8_unchecked(&buf) });
                        } else {
                            match st.types {
                                TYPE_DIR => println!(
                                    "\x1b[34m{}\x1b[0m {} {} {}",
                                    fmtname(unsafe {
                                        from_utf8_unchecked(&buf[..strlen(buf.as_ptr())])
                                    }),
                                    st.types,
                                    st.ino,
                                    st.size
                                ),
                                TYPE_FILE => println!(
                                    "{} {} {} {}",
                                    fmtname(unsafe {
                                        from_utf8_unchecked(&buf[..strlen(buf.as_ptr())])
                                    }),
                                    st.types,
                                    st.ino,
                                    st.size
                                ),
                                TYPE_DEVICE => println!(
                                    "\x1b[33m{}\x1b[0m {} {} {}",
                                    fmtname(unsafe {
                                        from_utf8_unchecked(&buf[..strlen(buf.as_ptr())])
                                    }),
                                    st.types,
                                    st.ino,
                                    st.size
                                ),
                                _ => unreachable!(),
                            }
                        }
                    }
                }
            }
        }
        _ => panic!(),
    }
}

#[no_mangle]
pub fn main(_args: Vec<&str>) {
    if _args.len() == 0 {
        ls(".");
    } else {
        for i in _args {
            ls(i);
        }
    }
}
