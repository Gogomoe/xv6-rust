use core::ptr::null_mut;

use lazy_static::lazy_static;
use spin::Mutex;

use super::*;
use super::layout::*;

extern {
    fn end();
}

#[repr(C)]
struct FreePage {
    next: *mut FreePage
}

#[repr(C)]
struct FreeMemory {
    head: *mut FreePage
}

unsafe impl Send for FreeMemory {}

pub struct Frame {
    number: usize,
}

impl Frame {
    pub fn from_physical_address(address: usize) -> Frame {
        Frame { number: address >> 12 }
    }

    pub fn addr(&self) -> usize {
        self.number << 12
    }
}

pub struct PhysicalMemory {
    start: usize,
    end: usize,
    memory: Mutex<FreeMemory>,
}

lazy_static! {
    pub static ref PHYSICAL_MEMORY: PhysicalMemory = {
        let phy_end = end as usize;
        let start = page_round_up(phy_end);
        let end = PHY_STOP;
        println!("physical memory: {:#x} - {:#x}", start, end);

        let memory = PhysicalMemory {
            start: start,
            end: end,
            memory: Mutex::new(FreeMemory { head: null_mut() }),
        };

        memory.free_range(start, end);

        memory
    };
}

impl PhysicalMemory {
    pub fn init(&self) {
        assert!(PHYSICAL_MEMORY.start == page_round_up(end as usize) && PHYSICAL_MEMORY.end == PHY_STOP)
    }

    pub fn free_range(&self, start: usize, end: usize) {
        let mut addr = page_round_up(start);
        while addr + PAGE_SIZE <= end {
            self.free(addr);
            addr += PAGE_SIZE;
        }
    }

    pub fn free(&self, addr: usize) {
        assert!(addr % PAGE_SIZE == 0 && addr >= self.start && addr < self.end);

        // unsafe { memset(addr, 1, PAGE_SIZE); }

        unsafe {
            let page = addr as *mut FreePage;
            let mut lock = self.memory.lock();
            let mut free = &mut *lock;

            let next_page = free.head;
            (*page).next = next_page;
            free.head = page
        }
    }

    pub fn alloc(&self) -> Option<Frame> {
        let mut lock = self.memory.lock();
        let mut free = &mut *lock;

        let addr = free.head;
        if addr.is_null() {
            return None;
        }

        unsafe {
            free.head = (*addr).next;
            // memset(addr as usize, 5, PAGE_SIZE);
        }

        return Some(Frame::from_physical_address(addr as usize));
    }

    pub fn dealloc(&self, frame: Frame) {
        self.free(frame.addr());
    }
}
