use crate::*;
use core::panic::PanicInfo;
use core::slice::from_raw_parts;
use core::str::from_utf8_unchecked;

extern "Rust" {
    fn main(_args: Vec<&str>);
}

#[no_mangle]
pub extern "C" fn _start(argc: usize, argv: *const *const u8) -> ! {
    unsafe {
        main(
            from_raw_parts(argv, argc)
                .into_iter()
                .skip(1)
                .map(|&a| from_utf8_unchecked(from_raw_parts(a, strlen(a))))
                .collect(),
        );
    }
    exit(0);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
