#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate bitflags;
extern crate linked_list_allocator;

global_asm!(include_str!("asm/entry.S"));
global_asm!(include_str!("asm/kernelvec.S"));
global_asm!(include_str!("asm/trampoline.S"));

#[macro_use]
mod print;

mod riscv;
mod console;
mod start;
mod memory;
mod process;
mod param;
mod trap;
mod plic;