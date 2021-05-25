#![no_std]
#![no_main]

use core::panic::PanicInfo;
use file_control_lib::{OPEN_READ_WRITE, CONSOLE_ID};
use user::*;

#[allow(non_upper_case_globals)]
const argv: [*const u8; 2] = ["sh\0".as_ptr(), 0 as *const u8];

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let ptr = "console\0".as_ptr();
    if open(ptr, OPEN_READ_WRITE) < 0 {
        mknod(ptr, CONSOLE_ID, 0);
        open(ptr, OPEN_READ_WRITE);
    }
    dup(0); // stdout
    dup(0); // stderr

    loop {
        println!("init: starting sh");
        let pid = fork();
        if pid < 0 {
            println!("init: fork failed");
            exit(1);
        }
        if pid == 0 {
            exec("sh\0".as_ptr(), &argv as *const _ as usize);
            println!("init: exec sh failed");
            exit(1);
        }

        loop {
            // this call to wait() returns if the shell exits,
            // or if a parentless process exits.
            let wpid = wait(0 as *const usize);
            if wpid == pid {
                // the shell exited; restart it.
                break;
            } else if wpid < 0 {
                println!("init: wait returned an error");
                exit(1);
            } else {
                // it was a parentless process; do nothing.
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}