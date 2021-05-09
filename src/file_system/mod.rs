pub use buffer_cache::BLOCK_CACHE;
pub use logging::LOG;

pub mod buffer_cache;
pub mod logging;

pub const BLOCK_SIZE: usize = 1024;

pub struct SuperBlock {
    magic: usize,
    size: usize,
    blocks_number: usize,
    inode_number: usize,
    log_number: usize,
    log_start: usize,
    inode_start: usize,
    block_map_start: usize,
}