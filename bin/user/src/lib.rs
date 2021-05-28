#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]

pub extern crate alloc;
pub extern crate cstr_core;
pub extern crate file_control_lib;
pub extern crate file_system_lib;

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