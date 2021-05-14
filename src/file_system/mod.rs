use core::ptr;

pub use buffer_cache::BLOCK_CACHE;
use define::{BLOCK_SIZE, FSMAGIC, ROOT_INO, SuperBlock};
pub use logging::LOG;

use crate::file_system::define::{bblock, BPB};

pub mod define;
pub mod buffer_cache;
pub mod logging;
pub mod inode;
pub mod path;
pub mod elf;

pub fn file_system_init(dev: u32) {
    unsafe {
        SUPER_BLOCK.read(dev);
        assert_eq!(SUPER_BLOCK.magic, FSMAGIC);
        LOG.init(dev, &SUPER_BLOCK);
    }
}

pub static mut SUPER_BLOCK: SuperBlock = SuperBlock::new();

impl SuperBlock {
    fn read(&mut self, dev: u32) {
        let block = BLOCK_CACHE.read(dev, 1);
        unsafe {
            ptr::copy(block.data() as *const SuperBlock, self as *mut SuperBlock, 1);
        }
        BLOCK_CACHE.release(block);
    }
}

struct Block {}

impl Block {
    pub fn zero(dev: u32, block_no: u32) {
        let block = BLOCK_CACHE.read(dev, block_no);
        unsafe {
            ptr::write_bytes(block.data(), 0, BLOCK_SIZE);
        }
        BLOCK_CACHE.write(&block);
        BLOCK_CACHE.release(block);
    }

    pub fn alloc(dev: u32) -> u32 {
        let sb = unsafe { &SUPER_BLOCK };
        let log = unsafe { &mut LOG };
        for b in (0..sb.size).step_by(BPB as usize) {
            let block = BLOCK_CACHE.read(dev, bblock(b, sb));
            for bi in 0..(BPB as u32) {
                if b + bi >= sb.size {
                    break;
                }

                let m = 1 << (bi % 8);
                let data = unsafe { &mut *block.data() };
                if data[(bi / 8) as usize] & m == 0 { // Is the block free?
                    data[(bi / 8) as usize] |= m;
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

    pub fn free(dev: u32, b: u32) {
        let sb = unsafe { &SUPER_BLOCK };
        let log = unsafe { &mut LOG };

        let block = BLOCK_CACHE.read(dev, bblock(b, sb));
        let data = unsafe { &mut *block.data() };
        let bi = b % (BPB as u32);
        let m = 1 << (bi % 8);

        assert_ne!(data[(bi / 8) as usize] & m, 0);

        data[(bi / 8) as usize] &= !m;
        log.write(&block);

        BLOCK_CACHE.release(block);
    }
}