#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate cstr_core;
extern crate file_control_lib;
extern crate file_system_lib;
extern crate ufmt;

#[macro_use]
pub mod print;
pub mod syscall;
pub mod umalloc;
pub mod ulib;

pub use syscall::*;
pub use ulib::*;