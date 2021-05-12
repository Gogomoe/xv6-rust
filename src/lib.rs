#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(core_intrinsics)]
#![feature(alloc_error_handler)]
#![feature(const_fn_union)]

#[macro_use]
extern crate alloc;
#[macro_use]
extern crate array_macro;
#[macro_use]
extern crate bitflags;
extern crate linked_list_allocator;

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
mod param;
mod trap;
mod plic;
mod spin_lock;
mod sleep_lock;
mod driver;
mod file_system;
mod syscall;