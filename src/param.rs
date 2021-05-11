pub const MAX_PROCESS_NUMBER: usize = 64;
pub const MAX_CPU_NUMBER: usize = 8;
pub const MAX_INODE_NUMBER: usize = 50;

pub const ROOT_DEV: usize = 1;

pub const MAX_OP_BLOCKS: usize = 10;
pub const LOG_SIZE: usize = 3 * MAX_OP_BLOCKS;
pub const BUFFER_SIZE: usize = 3 * MAX_OP_BLOCKS;