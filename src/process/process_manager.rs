use core::ptr::null_mut;

use spin::Mutex;

use crate::memory::{KERNEL_PAGETABLE, Page, PAGE_SIZE, PHYSICAL_MEMORY};
use crate::memory::layout::TRAMPOLINE;
use crate::memory::page_table::PageEntryFlags;
use crate::param::MAX_PROCESS_NUMBER;
use crate::process::context::Context;
use crate::process::CPU_MANAGER;
use crate::process::process::Process;
use crate::process::process::ProcessState::{RUNNABLE, RUNNING, SLEEPING, UNUSED};
use crate::riscv::{intr_on, sfence_vma};

pub struct ProcessManager {
    processes: [Process; MAX_PROCESS_NUMBER],
    pid: Mutex<usize>,
}

unsafe impl Send for ProcessManager {}

pub static PROCESS_MANAGER: ProcessManager = ProcessManager::new();

impl ProcessManager {
    const fn new() -> ProcessManager {
        ProcessManager {
            processes: array![_ => Process::new(); MAX_PROCESS_NUMBER],
            pid: Mutex::new(0),
        }
    }

    pub fn init(&self) {
        let mut pt_lock = KERNEL_PAGETABLE.lock();
        let page_table = &mut *pt_lock;

        for i in 0..MAX_PROCESS_NUMBER {
            let pa = PHYSICAL_MEMORY.alloc().unwrap();
            let va = TRAMPOLINE - (i + 1) * 2 * PAGE_SIZE;
            let rw = PageEntryFlags::READABLE | PageEntryFlags::WRITEABLE;
            page_table.map(Page::from_virtual_address(va), pa, rw);
            self.processes[i].data.borrow_mut().kernel_stack = va;
        }

        unsafe {
            sfence_vma();
        }
    }

    pub unsafe fn scheduler(&self) -> ! {
        extern {
            fn swtch(old: *mut Context, new: *mut Context);
        }
        let cpu = CPU_MANAGER.my_cpu_mut();

        cpu.process = null_mut();

        loop {
            intr_on();

            let mut found = false;
            for process in self.processes.iter() {
                let mut proc_lock = process.info.lock();
                let proc = &mut *proc_lock;
                if proc.state == RUNNABLE {
                    proc.state = RUNNING;
                    cpu.process = process as *const Process;

                    swtch(&mut cpu.context, &mut process.data.borrow_mut().context);

                    cpu.process = null_mut();

                    found = true;
                }
            }
            if !found {
                intr_on();
                llvm_asm!("wfi"::::"volatile");
            }
        }
    }

    pub fn wakeup(&self, channel: usize) {
        for process in self.processes.iter() {
            let mut info_lock = process.info.lock();
            let info = &mut *info_lock;
            if info.state == SLEEPING && info.channel == channel {
                info.state = RUNNING;
            }
        }
    }

    pub fn print_processes(&self) {
        for process in self.processes.iter() {
            let info_lock = process.info.lock();
            let info = &*info_lock;
            let data = process.data.borrow();
            if info.state != UNUSED {
                println!("{:5} {:10} {:20}", info.pid, info.state, data.name);
            }
        }
    }
}