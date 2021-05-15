use core::cell::UnsafeCell;
use core::ptr;

pub use buffer_cache::BLOCK_CACHE;
use file_system_lib::{bblock, BLOCK_SIZE, BPB, FSMAGIC, SuperBlock};
pub use file_table::FILE_TABLE;
pub use logging::LOG;

pub mod buffer_cache;
pub mod logging;
pub mod inode;
pub mod path;
pub mod elf;
pub mod file;
pub mod pipe;
pub mod file_table;

pub fn file_system_init(dev: u32) {
    unsafe {
        SUPER_BLOCK.read(dev);
        assert_eq!(SUPER_BLOCK.get().magic, FSMAGIC);
        LOG.init(dev, SUPER_BLOCK.get());
    }
}

pub static SUPER_BLOCK: SuperBlockWrapper = SuperBlockWrapper::new();

pub struct SuperBlockWrapper {
    super_block: UnsafeCell<SuperBlock>,
}

unsafe impl Sync for SuperBlockWrapper {}

impl SuperBlockWrapper {
    const fn new() -> SuperBlockWrapper {
        SuperBlockWrapper {
            super_block: UnsafeCell::new(SuperBlock::new())
        }
    }

    fn read(&self, dev: u32) {
        let block = BLOCK_CACHE.read(dev, 1);
        unsafe {
            ptr::copy(block.data() as *const SuperBlock, self.super_block.get(), 1);
        }
        BLOCK_CACHE.release(block);
    }

    pub fn get(&self) -> &mut SuperBlock {
        unsafe {
            self.super_block.get().as_mut().unwrap()
        }
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
        let sb = SUPER_BLOCK.get();
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
        let sb = SUPER_BLOCK.get();
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