#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]

extern crate ufmt;

global_asm!(include_str!("usys.S"));

#[macro_use]
pub mod print;

pub use print::*;

extern {
    pub fn write(_fd: usize, _str: *const u8, _size: usize) -> isize;
}