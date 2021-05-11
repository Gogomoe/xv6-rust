use alloc::string::String;
use core::ptr::null_mut;

use crate::file_system::file_system_init;
use crate::memory::{KERNEL_PAGETABLE, Page, PAGE_SIZE, PHYSICAL_MEMORY, user_virtual_memory};
use crate::memory::layout::{TRAMPOLINE, TRAPFRAME};
use crate::memory::page_table::PageEntryFlags;
use crate::param::{MAX_PROCESS_NUMBER, ROOT_DEV};
use crate::process::context::Context;
use crate::process::CPU_MANAGER;
use crate::process::process::Process;
use crate::process::process::ProcessState::{RUNNABLE, RUNNING, SLEEPING, UNUSED};
use crate::process::trap_frame::TrapFrame;
use crate::riscv::{intr_on, sfence_vma};
use crate::spin_lock::SpinLock;
use crate::trap::user_trap_return;

pub struct ProcessManager {
    processes: [Process; MAX_PROCESS_NUMBER],
    pid: SpinLock<usize>,
}

unsafe impl Send for ProcessManager {}

pub static PROCESS_MANAGER: ProcessManager = ProcessManager::new();

impl ProcessManager {
    const fn new() -> ProcessManager {
        ProcessManager {
            processes: array![_ => Process::new(); MAX_PROCESS_NUMBER],
            pid: SpinLock::new(0, "pid"),
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

            let process = unsafe { self.processes[i].data.get().as_mut() }.unwrap();
            process.kernel_stack = va;
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

                    let data = process.data.get().as_mut().unwrap();
                    swtch(&mut cpu.context, &mut data.context);

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
                info.state = RUNNABLE;
            }
        }
    }

    pub fn print_processes(&self) {
        for process in self.processes.iter() {
            let info_lock = process.info.lock();
            let info = &*info_lock;
            let data = unsafe { process.data.get().as_ref() }.unwrap();
            if info.state != UNUSED {
                println!("{:5} {:10} {:20}", info.pid, info.state, data.name);
            }
        }
    }

    pub unsafe fn user_init(&self) {
        let process = self.alloc_process().unwrap();

        let mut data = process.data.get().as_mut().unwrap();
        let mut info_guard = process.info.lock();
        let info = &mut info_guard;

        user_virtual_memory::init(data.page_table.as_mut().unwrap());
        data.size = PAGE_SIZE;

        (*data.trap_frame).epc = 0;
        (*data.trap_frame).sp = PAGE_SIZE as u64;

        data.name = String::from("initcode");
        // TODO
        // data.current_dir = namei("/");

        info.state = RUNNABLE;

        drop(info_guard);
    }

    pub fn alloc_process(&self) -> Option<&Process> {
        for process in self.processes.iter() {
            let mut guard = process.info.lock();
            let info = &mut *guard;
            if info.state == UNUSED {
                let mut data = unsafe { process.data.get().as_mut() }.unwrap();

                info.pid = self.alloc_pid();

                // Allocate a trapframe page.
                data.trap_frame = match PHYSICAL_MEMORY.alloc() {
                    Some(frame) => {
                        frame.addr() as *mut TrapFrame
                    }
                    None => {
                        drop(guard);
                        return None;
                    }
                };

                // An empty user page table.
                data.page_table = user_virtual_memory::alloc_page_table(data.trap_frame);
                if data.page_table.is_none() {
                    self.free_precess(process);
                    drop(guard);
                    return None;
                }

                // Set up new context to start executing at forkret,
                // which returns to user space.
                data.context.clear();
                data.context.ra = fork_return as u64;
                data.context.sp = (data.kernel_stack + PAGE_SIZE) as u64;

                drop(guard);
                return Some(process);
            }

            drop(guard);
        }

        None
    }

    fn alloc_pid(&self) -> usize {
        let mut guard = self.pid.lock();
        (*guard) = (*guard) + 1;
        let pid = *guard;
        drop(guard);
        return pid;
    }

    pub fn free_precess(&self, process: &Process) {
        let data_guard = unsafe { process.data.get().as_mut() }.unwrap();
        let data = &mut *data_guard;
        let mut info_guard = process.info.lock();
        let info = &mut info_guard;

        if !data.trap_frame.is_null() {
            PHYSICAL_MEMORY.free(data.trap_frame as usize);
        }
        data.trap_frame = null_mut();

        if data.page_table.is_some() {
            let mut page_table = data.page_table.take().unwrap();
            page_table.unmap_pages(TRAMPOLINE, PAGE_SIZE);
            page_table.unmap_pages(TRAPFRAME, PAGE_SIZE);
            user_virtual_memory::free_page_table(page_table, data.size);
        }
        data.page_table = None;
        data.size = 0;
        data.name.clear();

        info.pid = 0;
        info.channel = 0;
        info.state = UNUSED;

        // TODO parent, killed, exit state
    }
}

// A fork child's very first scheduling by scheduler()
// will swtch to forkret.
unsafe fn fork_return() {
    static mut IS_FIRST_PROCESS: bool = true;

    // Still holding p->lock from scheduler.
    (*CPU_MANAGER.my_proc()).info.unlock();

    if IS_FIRST_PROCESS {
        // File system initialization must be run in the context of a
        // regular process (e.g., because it calls sleep), and thus cannot
        // be run from main().
        IS_FIRST_PROCESS = false;
        file_system_init(ROOT_DEV);
    }

    user_trap_return();
}