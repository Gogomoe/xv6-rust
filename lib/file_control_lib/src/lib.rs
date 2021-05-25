#![no_std]
#![allow(dead_code)]

pub const OPEN_READ_ONLY: usize = 0x000;
pub const OPEN_WRITE_ONLY: usize = 0x001;
pub const OPEN_READ_WRITE: usize = 0x002;
pub const OPEN_CREATE: usize = 0x200;
pub const OPEN_TRUNC: usize = 0x400;

pub const CONSOLE_ID: usize = 1;