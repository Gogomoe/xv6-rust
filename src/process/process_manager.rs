use alloc::vec::Vec;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::memory::{KERNEL_PAGETABLE, Page, PAGE_SIZE, PHYSICAL_MEMORY};
use crate::memory::layout::TRAMPOLINE;
use crate::memory::page_table::PageEntryFlags;
use crate::param::MAX_PROCESS_NUMBER;
use crate::process::process::Process;

pub struct ProcessManager {
    processes: Vec<Process>,
    pid: Mutex<usize>,
}

unsafe impl Send for ProcessManager {}

lazy_static! {
    pub static ref PROCESS_MANAGER: ProcessManager = {
        let mut processes = Vec::new();
        for _ in 0..MAX_PROCESS_NUMBER  {
            processes.push(Process::new());
        }

        let manager = ProcessManager {
            processes,
            pid: Mutex::new(0),
        };

        manager
    };
}

impl ProcessManager {
    pub fn init(&self) {
        let mut pt_lock = KERNEL_PAGETABLE.lock();
        let page_table = &mut *pt_lock;

        for i in 0..MAX_PROCESS_NUMBER {
            let pa = PHYSICAL_MEMORY.alloc().unwrap();
            let va = TRAMPOLINE - (i + 1) * 2 * PAGE_SIZE;
            let rw = PageEntryFlags::READABLE | PageEntryFlags::WRITEABLE;
            page_table.map(Page::from_virtual_address(va), pa, rw);
            self.processes[i].data.lock().kernel_stack = va;
        }
    }
}