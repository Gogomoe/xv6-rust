#![no_std]
#![no_main]

use core::mem::size_of;

use user::*;

static mut BUF: [u8; 512] = [0; 512];

pub fn cat(fd: usize) {
    let mut n: isize;
    unsafe {
        //println!("{}", fd);
        n = read(fd, BUF.as_mut_ptr(), size_of::<[u8; 512]>());
        while n > 0 {
            if write(1, BUF.as_ptr(), n as usize) != n {
                eprintln!("cat: write error");
                exit(1);
            }
            n = read(fd, BUF.as_mut_ptr(), size_of::<[u8; 512]>());
        }
    }
    if n < 0 {
        eprintln!("cat: read error");
        exit(1);
    }
}

#[no_mangle]
pub fn main(_args: Vec<&str>) {
    if _args.is_empty() {
        cat(0);
    } else {
        for i in 0.._args.len() {
            let fd = open(_args[i], OPEN_READ_ONLY);
            if fd < 0 {
                eprintln!("cat: cannot open {}", _args[i]);
                exit(1);
            }
            cat(fd as usize);
            close(fd as usize);
        }
    }
}
