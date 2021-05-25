#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]

extern crate ufmt;
extern crate file_control_lib;

#[macro_use]
pub mod print;

pub mod syscall;
pub use syscall::*;