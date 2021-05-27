#![no_std]
#![feature(core_intrinsics)]
#![allow(dead_code)]

use core::intrinsics::size_of;

pub const ROOT_INO: u32 = 1;
pub const BLOCK_SIZE: usize = 1024;
pub const FSMAGIC: u32 = 0x10203040;

#[repr(C)]
pub struct SuperBlock {
    pub magic: u32,
    pub size: u32,
    pub blocks_number: u32,
    pub inode_number: u32,
    pub log_number: u32,
    pub log_start: u32,
    pub inode_start: u32,
    pub block_map_start: u32,
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

pub const DIRECT_COUNT: usize = 12;
pub const INDIRECT_COUNT: usize = BLOCK_SIZE / size_of::<u32>();
pub const MAX_FILE_COUNT: usize = DIRECT_COUNT + INDIRECT_COUNT;

#[repr(C)]
pub struct INodeDisk {
    pub types: u16,
    pub major: u16,
    pub minor: u16,
    pub nlink: u16,

    pub size: u32,
    pub addr: [u32; DIRECT_COUNT + 1],
}

impl INodeDisk {
    pub const fn new() -> INodeDisk {
        INodeDisk {
            types: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addr: [0; DIRECT_COUNT + 1],
        }
    }
}

// Inodes per block
pub const IPB: u32 = (BLOCK_SIZE / size_of::<INodeDisk>()) as u32;

// Block containing inode i
#[inline]
pub fn iblock(i: u32, sb: &SuperBlock) -> u32 {
    i / IPB + sb.inode_start
}

// Bitmap bits per block
pub const BPB: u32 = (BLOCK_SIZE * 8) as u32;

// Block of free map containing bit for block b
#[inline]
pub fn bblock(block: u32, sb: &SuperBlock) -> u32 {
    block / BPB as u32 + sb.block_map_start
}

// Directory is a file containing a sequence of dirent structures.
pub const DIRECTORY_SIZE: usize = 14;

#[repr(C)]
pub struct Dirent {
    pub inum: u16,
    pub name: [u8; DIRECTORY_SIZE],
}

impl Dirent {
    pub const fn new() -> Dirent {
        Dirent {
            inum: 0,
            name: [0; DIRECTORY_SIZE],
        }
    }
}

pub const TYPE_DIR: u16 = 1;
pub const TYPE_FILE: u16 = 2;
pub const TYPE_DEVICE: u16 = 3;

#[repr(C)]
pub struct FileStatus {
    pub dev: u32,
    pub ino: u32,
    pub types: u16,
    pub nlink: u16,
    pub size: u64,
}

impl FileStatus {
    pub const fn new() -> FileStatus {
        FileStatus {
            dev: 0,
            ino: 0,
            types: 0,
            nlink: 0,
            size: 0,
        }
    }
}
