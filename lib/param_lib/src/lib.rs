#![no_std]

pub const MAX_PROCESS_NUMBER: usize = 64;
pub const MAX_CPU_NUMBER: usize = 8;
// open files per process
pub const MAX_OPEN_FILE_NUMBER: usize = 16;
// open files per system
pub const MAX_FILE_NUMBER: usize = 100;
// maximum number of active i-nodes
pub const MAX_INODE_NUMBER: usize = 50;
// maximum major device number
pub const MAX_DEV_NUMBER: usize = 10;

pub const ROOT_DEV: u32 = 1;

pub const MAX_ARG: usize = 32;
pub const MAX_OP_BLOCKS: usize = 10;
pub const LOG_SIZE: usize = 3 * MAX_OP_BLOCKS;
pub const BUFFER_SIZE: usize = 3 * MAX_OP_BLOCKS;

pub const FILE_SYSTEM_SIZE: u32 = 200000;
