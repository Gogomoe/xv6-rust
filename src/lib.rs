#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]

global_asm!(include_str!("asm/entry.S"));
global_asm!(include_str!("asm/trampoline.S"));

#[macro_use]
mod print;

mod riscv;
mod console;
mod start;

#[allow(unused_variables)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}