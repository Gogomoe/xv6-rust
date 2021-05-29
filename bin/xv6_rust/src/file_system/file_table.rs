use core::cell::UnsafeCell;
use core::intrinsics::size_of;

use file_system_lib::{BLOCK_SIZE, FileStatus};
use param_lib::{MAX_DEV_NUMBER, MAX_FILE_NUMBER, MAX_OP_BLOCKS};

use crate::file_system::device::DEVICES;
use crate::file_system::file::File;
use crate::file_system::file::FileType::{DEVICE, INODE, NONE, PIPE};
use crate::file_system::inode::ICACHE;
use crate::file_system::LOG;
use crate::memory::either_copy_out;
use crate::spin_lock::SpinLock;

pub struct FileTable {
    lock: SpinLock<()>,
    file: UnsafeCell<[File; MAX_FILE_NUMBER]>,
}

pub static FILE_TABLE: FileTable = FileTable::new();

unsafe impl Sync for FileTable {}

impl FileTable {
    pub const fn new() -> FileTable {
        FileTable {
            lock: SpinLock::new((), "filetable"),
            file: UnsafeCell::new(array![_ => File::new(); MAX_FILE_NUMBER]),
        }
    }

    fn file(&self) -> &mut [File; MAX_FILE_NUMBER] {
        unsafe { self.file.get().as_mut() }.unwrap()
    }

    // Allocate a file structure.
    pub fn alloc(&self) -> Option<&File> {
        let guard = self.lock.lock();

        for file in self.file().iter() {
            if file.data().ref_count == 0 {
                file.data().ref_count = 1;
                drop(guard);
                return Some(file);
            }
        }
        drop(guard);
        return None;
    }

    // Increment ref count for file f.
    pub fn dup<'a>(&self, file: &'a File) -> &'a File {
        let guard = self.lock.lock();
        assert!(file.data().ref_count >= 1);
        file.data().ref_count += 1;
        drop(guard);
        return file;
    }

    // Close file f.  (Decrement ref count, close when reaches 0.)
    pub fn close(&self, file: &File) {
        let guard = self.lock.lock();
        assert!(file.data().ref_count >= 1);

        file.data().ref_count -= 1;
        if file.data().ref_count > 0 {
            drop(guard);
            return;
        }

        let types = file.data().types;
        #[allow(unused_variables)]
        let pipe = file.data().pipe;
        #[allow(unused_variables)]
        let writable = file.data().writable;
        let ip = file.data().ip;

        file.data().ref_count = 0;
        file.data().types = NONE;
        drop(guard);

        if types == PIPE {
            // pipeclose(pipe, writable);
            todo!();
        } else if types == INODE || types == DEVICE {
            let log = unsafe { &mut LOG };
            log.begin_op();
            ICACHE.put(ip.as_ref().unwrap());
            log.end_op();
        }
    }

    // Get metadata about file f.
    // addr is a user virtual address, pointing to a struct stat.
    pub fn stat(&self, file: &File, addr: usize) -> bool {
        if file.data().types == INODE || file.data().types == DEVICE {
            let ip = file.data().ip.unwrap();
            let guard = ip.lock();
            let status = ip.status();
            ip.unlock(guard);

            return either_copy_out(true, addr, &status as *const _ as usize, size_of::<FileStatus>());
        }
        return false;
    }

    // Read from file f.
    // addr is a user virtual address.
    pub fn read(&self, file: &File, addr: usize, size: usize) -> u64 {
        if !file.data().readable {
            return u64::max_value();
        }

        if file.data().types == PIPE {
            // piperead(file.pipe, addr, size)
            todo!();
        } else if file.data().types == DEVICE {
            let major = file.data().major;
            let devices = unsafe { &mut DEVICES };
            if major >= MAX_DEV_NUMBER as u16 || devices[major as usize].read.is_none() {
                return u64::max_value();
            }
            devices[major as usize].read.unwrap().call((true, addr, size)) as u64
        } else if file.data().types == INODE {
            let ip = file.data().ip.unwrap();
            let guard = ip.lock();
            let read = ip.read(true, addr, file.data().off, size as u32);
            if read > 0 {
                file.data().off += read;
            }
            drop(guard);
            read as u64
        } else {
            panic!("fileread");
        }
    }

    // Write to file f.
    // addr is a user virtual address.
    pub fn write(&self, file: &File, addr: usize, size: usize) -> u64 {
        if !file.data().writable {
            return u64::max_value();
        }

        if file.data().types == PIPE {
            // pipewrite(file.pipe, addr, size)
            todo!();
        } else if file.data().types == DEVICE {
            let major = file.data().major;
            let devices = unsafe { &mut DEVICES };
            if major >= MAX_DEV_NUMBER as u16 || devices[major as usize].write.is_none() {
                return u64::max_value();
            }
            devices[major as usize].write.unwrap().call((true, addr, size)) as u64
        } else if file.data().types == INODE {
            // write a few blocks at a time to avoid exceeding
            // the maximum log transaction size, including
            // i-node, indirect block, allocation blocks,
            // and 2 blocks of slop for non-aligned writes.
            // this really belongs lower down, since writei()
            // might be writing a device like the console.
            let max = ((MAX_OP_BLOCKS - 1 - 1 - 2) / 2) * BLOCK_SIZE;
            let log = unsafe { &mut LOG };
            let ip = file.data().ip.unwrap();

            let mut i = 0;
            while i < size {
                let mut n1 = size - i;
                if n1 > max {
                    n1 = max;
                }

                log.begin_op();
                let guard = ip.lock();
                let write = ip.write(true, addr + i, file.data().off, n1 as u32);
                if write > 0 {
                    file.data().off += write;
                }
                ip.unlock(guard);
                log.end_op();

                if write as usize != n1 {
                    // error from writei
                    break;
                }
                i += write as usize;
            }

            if i == size { size as u64 } else { u64::max_value() }
        } else {
            panic!("filewrite");
        }
    }
}
