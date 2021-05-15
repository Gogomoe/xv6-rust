use core::cell::UnsafeCell;
use core::ptr::null_mut;

use crate::file_system::inode::INode;
use crate::file_system::pipe::Pipe;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FileType {
    NONE,
    PIPE,
    INODE,
    DEVICE,
}

pub struct FileData {
    pub types: FileType,
    pub ref_count: usize,
    pub readable: bool,
    pub writable: bool,

    // FD_PIPE
    pub pipe: *mut Pipe,
    // FD_INODE and FD_DEVICE
    pub ip: Option<&'static INode>,
    // FD_INODE
    pub off: u32,
    // FD_DEVICE
    pub major: u16,
}

pub struct File {
    data: UnsafeCell<FileData>,
}

impl File {
    pub const fn new() -> File {
        File {
            data: UnsafeCell::new(FileData {
                types: FileType::NONE,
                ref_count: 0,
                readable: false,
                writable: false,
                pipe: null_mut(),
                ip: None,
                off: 0,
                major: 0,
            })
        }
    }

    pub fn data(&self) -> &mut FileData {
        unsafe { self.data.get().as_mut() }.unwrap()
    }
}