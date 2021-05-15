use crate::file_system::inode::INode;
use crate::file_system::pipe::Pipe;

pub enum FileType {
    NONE,
    PIPE,
    INODE,
    DEVICE,
}

pub struct File {
    types: FileType,
    ref_count: usize,
    readable: bool,
    writable: bool,

    // FD_PIPE
    pipe: *mut Pipe,
    // FD_INODE and FD_DEVICE
    ip: *mut INode,
    // FD_INODE
    off: u32,
    // FD_DEVICE
    major: u16,
}