pub const TYPE_DIR: u16 = 1;
pub const TYPE_FILE: u16 = 2;
pub const TYPE_DEVICE: u16 = 3;

pub struct FileStatus {
    pub dev: u32,
    pub ino: u32,
    pub types: u16,
    pub nlink: u16,
    pub size: u32,
}