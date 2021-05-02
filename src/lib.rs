#![no_std]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(core_intrinsics)]

#[macro_use]
extern crate bitflags;

global_asm!(include_str!("asm/entry.S"));
global_asm!(include_str!("asm/trampoline.S"));

#[macro_use]
mod print;

mod riscv;
mod console;
mod start;
mod memory;
