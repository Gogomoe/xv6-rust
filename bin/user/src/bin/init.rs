#![no_std]
#![no_main]

use core::panic::PanicInfo;
use user::*;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("myinit");
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}