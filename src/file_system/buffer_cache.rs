use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::driver::DISK;
use crate::file_system::BLOCK_SIZE;
use crate::param::BUFFER_SIZE;
use crate::sleep_lock::{SleepLock, SleepLockGuard};
use crate::spin_lock::SpinLock;

type BufferData = [u8; BLOCK_SIZE];

pub struct Buffer {
    valid: bool,
    dev: u32,
    block_no: u32,
    data: SleepLock<BufferData>,
    ref_count: u32,
    last_used_time: usize,
}

impl Buffer {
    pub const fn new() -> Buffer {
        Buffer {
            valid: false,
            dev: 0,
            block_no: 0,
            data: SleepLock::new([0; BLOCK_SIZE]),
            ref_count: 0,
            last_used_time: 0,
        }
    }
}

pub struct BufferGuard<'a> {
    index: usize,
    dev: u32,
    block_no: u32,
    data: SleepLockGuard<'a, BufferData>,
}

impl BufferGuard<'_> {
    pub fn data(&self) -> *mut [u8; BLOCK_SIZE] {
        self.data.as_ptr() as *const _ as *mut [u8; BLOCK_SIZE]
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn dev(&self) -> u32 {
        self.dev
    }

    pub fn block_no(&self) -> u32 {
        self.block_no
    }
}

pub struct BlockCache {
    buffers: UnsafeCell<[Buffer; BUFFER_SIZE]>,
    lock: SpinLock<()>,
    lru_release_count: AtomicUsize,
}

pub static BLOCK_CACHE: BlockCache = BlockCache::new();

unsafe impl Sync for BlockCache {}

impl BlockCache {
    pub const fn new() -> BlockCache {
        BlockCache {
            buffers: UnsafeCell::new(array![_ => Buffer::new(); BUFFER_SIZE]),
            lock: SpinLock::new((), "block cache"),
            lru_release_count: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, dev: u32, block_no: u32) -> BufferGuard {
        let lock_guard = self.lock.lock();

        match self.find(dev, block_no).or(self.alloc(dev, block_no)) {
            Some(index) => {
                let buffers = unsafe { self.buffers.get().as_mut().unwrap() };
                buffers[index].ref_count += 1;
                let data = buffers[index].data.lock();
                drop(lock_guard);
                return BufferGuard {
                    index,
                    dev,
                    block_no,
                    data,
                };
            }
            None => {
                panic!("no buffers");
            }
        }
    }

    /// should hold lock
    fn find(&self, dev: u32, block_no: u32) -> Option<usize> {
        let buffers = unsafe { self.buffers.get().as_mut().unwrap() };

        for index in 0..buffers.len() {
            if buffers[index].dev == dev && buffers[index].block_no == block_no {
                return Some(index);
            }
        }
        None
    }

    /// should hold lock
    fn alloc(&self, dev: u32, block_no: u32) -> Option<usize> {
        let buffers = unsafe { self.buffers.get().as_mut().unwrap() };

        let mut lru_index: Option<usize> = None;
        for index in 0..buffers.len() {
            if buffers[index].ref_count == 0 && (lru_index.is_none() || buffers[lru_index.unwrap()].last_used_time < buffers[index].last_used_time) {
                lru_index = Some(index);
            }
        }
        if lru_index.is_none() {
            return None;
        }

        let index = lru_index.unwrap();
        buffers[index].dev = dev;
        buffers[index].block_no = block_no;
        buffers[index].valid = false;

        lru_index
    }

    pub fn read(&self, dev: u32, block_no: u32) -> BufferGuard {
        let buffer = self.get(dev, block_no);
        let buffers = unsafe { self.buffers.get().as_mut().unwrap() };
        let valid = &mut buffers[buffer.index].valid;
        if !*valid {
            unsafe {
                DISK.read(buffer.block_no, buffer.data.as_ptr() as *mut BufferData);
            }
            *valid = true;
        }
        return buffer;
    }

    pub fn write(&self, buffer: &BufferGuard) {
        unsafe {
            DISK.write(buffer.block_no, buffer.data.as_ptr() as *mut BufferData);
        }
    }

    pub fn release(&self, buffer: BufferGuard) {
        drop(buffer.data);

        let lock_guard = self.lock.lock();
        let buffers = unsafe { self.buffers.get().as_mut().unwrap() };
        buffers[buffer.index].ref_count -= 1;

        if buffers[buffer.index].ref_count == 0 {
            let used_time = self.lru_release_count.load(Ordering::Relaxed);
            buffers[buffer.index].last_used_time = used_time;
            self.lru_release_count.store(used_time + 1, Ordering::Relaxed);
        }

        drop(lock_guard);
    }

    pub fn pin(&self, buffer: &BufferGuard) {
        let lock_guard = self.lock.lock();
        let buffers = unsafe { self.buffers.get().as_mut().unwrap() };
        buffers[buffer.index].ref_count += 1;
        drop(lock_guard);
    }

    pub fn unpin(&self, buffer: &BufferGuard) {
        let lock_guard = self.lock.lock();
        let buffers = unsafe { self.buffers.get().as_mut().unwrap() };
        buffers[buffer.index].ref_count -= 1;
        drop(lock_guard);
    }
}