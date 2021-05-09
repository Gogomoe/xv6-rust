use core::intrinsics::size_of;
use core::ptr;

use spin::Mutex;

use crate::file_system::{BLOCK_CACHE, BLOCK_SIZE, SuperBlock};
use crate::file_system::buffer_cache::BufferGuard;
use crate::param::{LOG_SIZE, MAX_OP_BLOCKS};
use crate::process::{CPU_MANAGER, PROCESS_MANAGER};

struct LogHeader {
    n: usize,
    block: [usize; LOG_SIZE],
}

impl LogHeader {
    const fn new() -> LogHeader {
        LogHeader {
            n: 0,
            block: [0; LOG_SIZE],
        }
    }
}

pub struct Log {
    lock: Mutex<()>,
    start: usize,
    size: usize,
    // how many FS sys calls are executing.
    outstanding: usize,
    // in commit(), please wait.
    committing: bool,
    dev: usize,
    header: LogHeader,
}

pub static mut LOG: Log = Log::new();

impl Log {
    pub const fn new() -> Log {
        Log {
            lock: Mutex::new(()),
            start: 0,
            size: 0,
            outstanding: 0,
            committing: false,
            dev: 0,
            header: LogHeader::new(),
        }
    }

    pub fn init(&mut self, dev: usize, sb: &SuperBlock) {
        assert!(size_of::<LogHeader>() < BLOCK_SIZE);

        self.start = sb.log_start;
        self.size = sb.log_number;
        self.dev = dev;

        self.recover_from_log();
    }

    fn recover_from_log(&mut self) {
        self.read_head();
        self.install_transaction(true); // if committed, copy from log to disk
        self.header.n = 0;
        self.write_head(); // clear the log
    }

    // Read the log header from disk into the in-memory log header
    fn read_head(&mut self) {
        let buffer = BLOCK_CACHE.read(self.dev, self.start);
        let header = buffer.data() as *const LogHeader;
        unsafe {
            self.header.n = (*header).n;
            for i in 0..self.header.n {
                self.header.block[i] = (*header).block[i];
            }
        }
        BLOCK_CACHE.release(buffer);
    }

    // Write in-memory log header to disk.
    // This is the true point at which the
    // current transaction commits.
    fn write_head(&mut self) {
        let buffer = BLOCK_CACHE.read(self.dev, self.start);
        let header = buffer.data() as *mut LogHeader;
        unsafe {
            (*header).n = self.header.n;
            for i in 0..self.header.n {
                (*header).block[i] = self.header.block[i];
            }
        }

        BLOCK_CACHE.write(&buffer);
        BLOCK_CACHE.release(buffer);
    }

    // Copy committed blocks from log to their home location
    fn install_transaction(&mut self, recovering: bool) {
        for i in 0..self.header.n {
            let log_buffer = BLOCK_CACHE.read(self.dev, self.start + i + 1);
            let dest_buffer = BLOCK_CACHE.read(self.dev, self.header.block[i]);
            unsafe {
                ptr::copy(log_buffer.data(), dest_buffer.data(), 1);
            }
            BLOCK_CACHE.write(&dest_buffer);
            if !recovering {
                BLOCK_CACHE.unpin(&dest_buffer);
            }
            BLOCK_CACHE.release(log_buffer);
            BLOCK_CACHE.release(dest_buffer);
        }
    }

    pub fn begin_op(&mut self) {
        let mut guard = self.lock.lock();
        loop {
            if self.committing {
                CPU_MANAGER.my_cpu_mut().sleep(self as *const _ as usize, guard);
                guard = self.lock.lock();
            } else if self.header.n + (self.outstanding + 1) * MAX_OP_BLOCKS > LOG_SIZE {
                CPU_MANAGER.my_cpu_mut().sleep(self as *const _ as usize, guard);
                guard = self.lock.lock();
            } else {
                self.outstanding += 1;
                drop(guard);
                break;
            }
        }
    }

    pub fn end_op(&mut self) {
        let mut commit = false;

        {
            let guard = self.lock.lock();

            self.outstanding -= 1;
            assert!(!self.committing);

            if self.outstanding == 0 {
                commit = true;
                self.committing = true;
            } else {
                // begin_op() may be waiting for log space,
                // and decrementing log.outstanding has decreased
                // the amount of reserved space.
                PROCESS_MANAGER.wakeup(self as *const _ as usize);
            }

            drop(guard);
        }


        if commit {
            // call commit w/o holding locks, since not allowed
            // to sleep with locks.
            self.commit();
            let guard = self.lock.lock();
            self.committing = false;
            PROCESS_MANAGER.wakeup(self as *const _ as usize);
            drop(guard);
        }
    }

    fn commit(&mut self) {
        if self.header.n > 0 {
            self.write_log();
            self.write_head();
            self.install_transaction(false);
            self.header.n = 0;
            self.write_head();
        }
    }

    fn write_log(&mut self) {
        for i in 0..self.header.n {
            let log_buffer = BLOCK_CACHE.read(self.dev, self.start + i + 1);
            let cache_buffer = BLOCK_CACHE.read(self.dev, self.header.block[i]);
            unsafe {
                ptr::copy(cache_buffer.data(), log_buffer.data(), 1);
            }
            BLOCK_CACHE.write(&log_buffer);
            BLOCK_CACHE.release(cache_buffer);
            BLOCK_CACHE.release(log_buffer);
        }
    }

    pub fn write(&mut self, buffer: &BufferGuard) {
        assert!(self.header.n < LOG_SIZE && self.header.n < self.size - 1);
        assert!(self.outstanding >= 1);

        let guard = self.lock.lock();

        for i in 0..self.header.n {
            if self.header.block[i] == buffer.block_no() {
                drop(guard);
                return;
            }
        }

        // Add new block to log
        BLOCK_CACHE.pin(&buffer);
        self.header.n += 1;
        drop(guard);
    }
}