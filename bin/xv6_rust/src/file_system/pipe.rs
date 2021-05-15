use crate::spin_lock::SpinLock;

pub const PIPE_SIZE: usize = 512;

pub struct Pipe {
    lock: SpinLock<()>,
    data: [u8; PIPE_SIZE],

    read_number: usize,
    write_number: usize,

    read_open: bool,
    write_open: bool,
}