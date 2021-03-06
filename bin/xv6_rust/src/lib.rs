#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(const_fn_union)]
#![feature(fn_traits)]
#![feature(const_fn_fn_ptr_basics)]

extern crate alloc;
#[macro_use]
extern crate array_macro;
#[macro_use]
extern crate bitflags;
extern crate cstr_core;
extern crate file_system_lib;
extern crate file_control_lib;
extern crate linked_list_allocator;
extern crate param_lib;

global_asm!(include_str!("asm/entry.S"));
global_asm!(include_str!("asm/kernelvec.S"));
global_asm!(include_str!("asm/swtch.S"));
global_asm!(include_str!("asm/trampoline.S"));

#[macro_use]
mod print;

mod riscv;
mod console;
mod start;
mod memory;
mod process;
mod trap;
mod plic;
mod spin_lock;
mod sleep_lock;
mod driver;
mod file_system;
mod syscall;