use alloc::string::String;
use core::cell::UnsafeCell;
use core::ptr::null_mut;

use crate::file_system::file_system_init;
use crate::memory::{KERNEL_PAGETABLE, Page, PAGE_SIZE, PHYSICAL_MEMORY, user_virtual_memory};
use crate::memory::layout::{TRAMPOLINE, TRAPFRAME};
use crate::memory::page_table::PageEntryFlags;
use crate::param::{MAX_PROCESS_NUMBER, ROOT_DEV};
use crate::process::context::Context;
use crate::process::CPU_MANAGER;
use crate::process::process::{Process, ProcessInfo};
use crate::process::process::ProcessState::{RUNNABLE, RUNNING, SLEEPING, UNUSED, ZOMBIE};
use crate::process::trap_frame::TrapFrame;
use crate::riscv::{intr_on, sfence_vma};
use crate::spin_lock::{SpinLock, SpinLockGuard};
use crate::trap::user_trap_return;

pub struct ProcessManager {
    processes: [Process; MAX_PROCESS_NUMBER],
    pid: SpinLock<usize>,
    init_process: UnsafeCell<*const Process>,
}

unsafe impl Send for ProcessManager {}

unsafe impl Sync for ProcessManager {}

pub static PROCESS_MANAGER: ProcessManager = ProcessManager::new();

impl ProcessManager {
    const fn new() -> ProcessManager {
        ProcessManager {
            processes: array![_ => Process::new(); MAX_PROCESS_NUMBER],
            pid: SpinLock::new(0, "pid"),
            init_process: UnsafeCell::new(null_mut()),
        }
    }

    fn init_process(&self) -> &Process {
        unsafe { (*self.init_process.get()).as_ref() }.unwrap()
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

    pub fn wake_up(&self, channel: usize) {
        for process in self.processes.iter() {
            let mut info_lock = process.info.lock();
            let info = &mut *info_lock;
            if info.state == SLEEPING && info.channel == channel {
                info.state = RUNNABLE;
            }
        }
    }

    pub fn wake_up_process(&self, process: &Process, info_guard: &mut SpinLockGuard<ProcessInfo>) {
        if info_guard.channel == process as *const _ as usize && info_guard.state == SLEEPING {
            info_guard.state = RUNNABLE;
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

        (*self.init_process.get()) = process as *const Process;

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
        info.parent = None;
        info.killed = false;
        info.exit_state = 0;
    }

    pub fn exit(&self, exit_state: i32) {
        let process = CPU_MANAGER.my_proc().unwrap();

        assert_ne!(process as *const _, self.init_process() as *const _);

        // TODO close open files

        // we might re-parent a child to init. we can't be precise about
        // waking up init, since we can't acquire its lock once we've
        // acquired any other proc lock. so wake up init whether that's
        // necessary or not. init may miss this wakeup, but that seems
        // harmless.
        let mut info_guard = self.init_process().info.lock();
        self.wake_up_process(process, &mut info_guard);
        drop(info_guard);

        // grab a copy of p->parent, to ensure that we unlock the same
        // parent we locked. in case our parent gives us away to init while
        // we're waiting for the parent lock. we may then race with an
        // exiting parent, but the result will be a harmless spurious wakeup
        // to a dead or wrong process; proc structs are never re-allocated
        // as anything else.
        let info_guard = process.info.lock();
        let parent = info_guard.parent.unwrap();
        drop(info_guard);

        // we need the parent's lock in order to wake it up from wait().
        // the parent-then-child rule says we have to lock it first.
        let mut parent_guard = parent.info.lock();
        let mut self_guard = process.info.lock();

        // Give any children to init.
        self.reparent(&self_guard);

        // Parent might be sleeping in wait().
        self.wake_up_process(process, &mut parent_guard);

        self_guard.exit_state = exit_state;
        self_guard.state = ZOMBIE;

        drop(parent_guard);

        unsafe {
            CPU_MANAGER.my_cpu_mut().scheduled(&self_guard);
        }

        panic!("zombie exit");
    }

    fn reparent(&self, p: &SpinLockGuard<ProcessInfo>) {
        todo!();
    }
}

// A fork child's very first scheduling by scheduler()
// will swtch to forkret.
unsafe fn fork_return() {
    static mut IS_FIRST_PROCESS: bool = true;

    // Still holding p->lock from scheduler.
    CPU_MANAGER.my_proc().unwrap().info.unlock();

    if IS_FIRST_PROCESS {
        // File system initialization must be run in the context of a
        // regular process (e.g., because it calls sleep), and thus cannot
        // be run from main().
        IS_FIRST_PROCESS = false;
        file_system_init(ROOT_DEV);
    }

    user_trap_return();
}