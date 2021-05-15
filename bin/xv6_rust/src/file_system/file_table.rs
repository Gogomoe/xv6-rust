use core::cell::UnsafeCell;

use param_lib::MAX_FILE_NUMBER;

use crate::file_system::file::File;
use crate::spin_lock::SpinLock;
use crate::file_system::file::FileType::{NONE, PIPE, INODE, DEVICE};
use crate::file_system::LOG;
use crate::file_system::inode::ICACHE;

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
        let pipe = file.data().pipe;
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
}
