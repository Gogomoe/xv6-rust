use core::ptr;

pub use buffer_cache::BLOCK_CACHE;
pub use logging::LOG;

pub mod buffer_cache;
pub mod logging;
pub mod inode;

pub const BLOCK_SIZE: usize = 1024;
pub const FSMAGIC: usize = 0x10203040;

pub const DIRECTORY_COUNT: usize = 12;

pub const BPB: usize = BLOCK_SIZE * 8;

#[inline]
fn bblock(block: usize, sb: &SuperBlock) -> usize {
    block / BPB + sb.block_map_start
}

pub fn file_system_init(dev: usize) {
    unsafe {
        SUPER_BLOCK.read(dev);
        assert_eq!(SUPER_BLOCK.magic, FSMAGIC);
        LOG.init(dev, &SUPER_BLOCK);
    }
}

pub static mut SUPER_BLOCK: SuperBlock = SuperBlock::new();

#[repr(C)]
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

impl SuperBlock {
    fn read(&mut self, dev: usize) {
        let block = BLOCK_CACHE.read(dev, 1);
        unsafe {
            ptr::copy(block.data() as *const SuperBlock, self as *mut SuperBlock, 1);
        }
        BLOCK_CACHE.release(block);
    }
}

impl SuperBlock {
    pub const fn new() -> SuperBlock {
        SuperBlock {
            magic: 0,
            size: 0,
            blocks_number: 0,
            inode_number: 0,
            log_number: 0,
            log_start: 0,
            inode_start: 0,
            block_map_start: 0,
        }
    }
}

struct Block {}

impl Block {
    pub fn zero(dev: usize, block_no: usize) {
        let block = BLOCK_CACHE.read(dev, block_no);
        unsafe {
            ptr::write_bytes(block.data(), 0, BLOCK_SIZE);
        }
        BLOCK_CACHE.write(&block);
        BLOCK_CACHE.release(block);
    }

    pub fn alloc(dev: usize) -> usize {
        let sb = unsafe { &SUPER_BLOCK };
        let log = unsafe { &mut LOG };
        for b in (0..sb.size).step_by(BPB) {
            let block = BLOCK_CACHE.read(dev, bblock(b, sb));
            for bi in 0..BPB {
                if b + bi >= sb.size {
                    break;
                }

                let m = 1 << (bi % 8);
                let data = unsafe { &mut *block.data() };
                if data[bi / 8] & m == 0 { // Is the block free?
                    data[bi / 8] |= m;
                    log.write(&block);
                    BLOCK_CACHE.release(block);
                    Block::zero(dev, b + bi);
                    return b + bi;
                }
            }
            BLOCK_CACHE.release(block);
        }

        panic!("out of blocks");
    }

    pub fn free(dev: usize, b: usize) {
        let sb = unsafe { &SUPER_BLOCK };
        let log = unsafe { &mut LOG };

        let block = BLOCK_CACHE.read(dev, bblock(b, sb));
        let data = unsafe { &mut *block.data() };
        let bi = b % BPB;
        let m = 1 << (bi % 8);

        assert_ne!(data[bi / 8] & m, 0);

        data[bi / 8] &= !m;
        log.write(&block);

        BLOCK_CACHE.release(block);
    }
}