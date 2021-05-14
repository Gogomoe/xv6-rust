use core::intrinsics::size_of;
use core::ptr;

pub use buffer_cache::BLOCK_CACHE;
pub use logging::LOG;

pub mod buffer_cache;
pub mod logging;
pub mod inode;
pub mod path;
pub mod elf;

pub const ROOT_INO: u32 = 1;
pub const BLOCK_SIZE: usize = 1024;
pub const FSMAGIC: u32 = 0x10203040;

pub const DIRECTORY_COUNT: usize = 12;
pub const DIRECTORY_INNER_COUNT: usize = BLOCK_SIZE / size_of::<usize>();
pub const MAX_FILE_COUNT: usize = DIRECTORY_COUNT + DIRECTORY_INNER_COUNT;

pub const BPB: u32 = (BLOCK_SIZE * 8) as u32;

pub const DIRECTORY_SIZE: usize = 14;

#[repr(C)]
pub struct Dirent {
    inum: u16,
    name: [u8; DIRECTORY_SIZE],
}

#[inline]
fn bblock(block: u32, sb: &SuperBlock) -> u32 {
    block / BPB as u32 + sb.block_map_start
}

pub fn file_system_init(dev: u32) {
    unsafe {
        SUPER_BLOCK.read(dev);
        assert_eq!(SUPER_BLOCK.magic, FSMAGIC);
        LOG.init(dev, &SUPER_BLOCK);
    }
}

pub static mut SUPER_BLOCK: SuperBlock = SuperBlock::new();

#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    size: u32,
    blocks_number: u32,
    inode_number: u32,
    log_number: u32,
    log_start: u32,
    inode_start: u32,
    block_map_start: u32,
}

impl SuperBlock {
    fn read(&mut self, dev: u32) {
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