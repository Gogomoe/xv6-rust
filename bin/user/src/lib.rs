#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate cstr_core;
extern crate file_control_lib;
extern crate file_system_lib;

#[macro_use]
pub mod _start;
pub mod print;
pub mod syscall;
pub mod ulib;
pub mod umalloc;

pub use alloc::string::String;
pub use alloc::vec::Vec;
pub use syscall::*;
pub use ulib::*;