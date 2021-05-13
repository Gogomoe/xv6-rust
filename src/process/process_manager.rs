use alloc::string::String;
use core::cell::UnsafeCell;
use core::ptr::null_mut;

use crate::file_system::file_system_init;
use crate::file_system::path::find_inode;
use crate::memory::{KERNEL_PAGETABLE, Page, PAGE_SIZE, PHYSICAL_MEMORY, user_virtual_memory};
use crate::memory::layout::{KERNEL_STACK_PAGE_COUNT, TRAMPOLINE, TRAPFRAME};
use crate::memory::page_table::PageEntryFlags;
use crate::param::{MAX_PROCESS_NUMBER, ROOT_DEV};
use crate::process::context::Context;
use crate::process::CPU_MANAGER;
use crate::process::process::Process;
use crate::process::process::ProcessState::{RUNNABLE, RUNNING, SLEEPING, UNUSED, ZOMBIE};
use crate::process::trap_frame::TrapFrame;
use crate::riscv::{intr_on, sfence_vma};
use crate::spin_lock::SpinLock;
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

    fn init_process(&self) -> &'static Process {
        unsafe { (*self.init_process.get()).as_ref() }.unwrap()
    }

    pub fn init(&self) {
        let mut pt_lock = KERNEL_PAGETABLE.lock();
        let page_table = &mut *pt_lock;

        let rw = PageEntryFlags::READABLE | PageEntryFlags::WRITEABLE;
        let top_guard = TRAMPOLINE - PAGE_SIZE;

        for i in 0..MAX_PROCESS_NUMBER {
            let stack_top = top_guard - i * (KERNEL_STACK_PAGE_COUNT + 1) * PAGE_SIZE;

            for j in 0..KERNEL_STACK_PAGE_COUNT {
                let pa = PHYSICAL_MEMORY.alloc().unwrap();
                let va = stack_top - (j + 1) * PAGE_SIZE;
                page_table.map(Page::from_virtual_address(va), pa, rw);
            }

            let process = self.processes[i].data();
            process.kernel_stack = stack_top;
        }

        unsafe {
            sfence_vma();
        }
    }

    // Per-CPU process scheduler.
    // Each CPU calls scheduler() after setting itself up.
    // Scheduler never returns.  It loops, doing:
    //  - choose a process to run.
    //  - swtch to start running that process.
    //  - eventually that process transfers control
    //    via swtch back to the scheduler.
    pub unsafe fn scheduler(&self) -> ! {
        extern {
            fn swtch(old: *mut Context, new: *mut Context);
        }
        let cpu = CPU_MANAGER.my_cpu_mut();

        cpu.process = null_mut();

        loop {
            // Avoid deadlock by ensuring that devices can interrupt.
            intr_on();

            let mut process_count = 0;
            for process in self.processes.iter() {
                let guard = process.lock.lock();
                let info = process.info();
                if info.state != UNUSED {
                    process_count += 1;
                }
                if info.state == RUNNABLE {
                    // Switch to chosen process.  It is the process's job
                    // to release its lock and then reacquire it
                    // before jumping back to us.
                    info.state = RUNNING;
                    cpu.process = process as *const Process;

                    swtch(&mut cpu.context, &mut process.data().context);

                    // Process is done running for now.
                    // It should have changed its p->state before coming back.
                    cpu.process = null_mut();
                }
                drop(guard);
            }
            if process_count <= 2 { // only init and sh exist
                intr_on();
                llvm_asm!("wfi"::::"volatile");
            }
        }
    }

    // Wake up all processes sleeping on chan.
    // Must be called without any p->lock.
    pub fn wake_up(&self, channel: usize) {
        for process in self.processes.iter() {
            let guard = process.lock.lock();
            let info = process.info();
            if info.state == SLEEPING && info.channel == channel {
                info.state = RUNNABLE;
            }
            drop(guard);
        }
    }

    // Wake up p if it is sleeping in wait(); used by exit().
    // Caller must hold p->lock.
    pub fn wake_up_process(&self, process: &Process) {
        assert!(process.lock.holding());
        if process.info().channel == process as *const _ as usize && process.info().state == SLEEPING {
            process.info().state = RUNNABLE;
        }
    }

    // Print a process listing to console.  For debugging.
    // Runs when user types ^P on console.
    // No lock to avoid wedging a stuck machine further.
    pub fn print_processes(&self) {
        for process in self.processes.iter() {
            let guard = process.lock.lock();
            let info = process.info();
            let data = process.data();
            if info.state != UNUSED {
                println!("{:5} {:10} {:20}", info.pid, info.state, data.name);
            }
            drop(guard);
        }
    }

    pub unsafe fn user_init(&self) {
        let process = self.alloc_process().unwrap();

        (*self.init_process.get()) = process as *const Process;

        let mut data = process.data();
        let guard = process.lock.lock();
        let info = &mut process.info();

        user_virtual_memory::init(data.page_table.as_mut().unwrap());
        data.size = PAGE_SIZE;

        (*data.trap_frame).epc = 0;
        (*data.trap_frame).sp = PAGE_SIZE as u64;

        data.name = String::from("initcode");
        data.current_dir = find_inode(&String::from("/"));

        info.state = RUNNABLE;

        drop(guard);
    }

    // Look in the process table for an UNUSED proc.
    // If found, initialize state required to run in the kernel,
    // and return with p->lock held.
    // If there are no free procs, or a memory allocation fails, return 0.
    pub fn alloc_process(&self) -> Option<&Process> {
        for process in self.processes.iter() {
            let guard = process.lock.lock();
            let info = process.info();
            if info.state == UNUSED {
                let mut data = process.data();

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
                data.context.sp = data.kernel_stack as u64;

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

    // free a proc structure and the data hanging from it,
    // including user pages.
    // p->lock must be held.
    pub fn free_precess(&self, process: &Process) {
        let data = process.data();
        let info = process.info();

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

    // Exit the current process.  Does not return.
    // An exited process remains in the zombie state
    // until its parent calls wait().
    pub fn exit(&self, exit_state: i32) {
        let process = CPU_MANAGER.my_proc();

        assert_ne!(process as *const _, self.init_process() as *const _);

        // TODO close open files

        // we might re-parent a child to init. we can't be precise about
        // waking up init, since we can't acquire its lock once we've
        // acquired any other proc lock. so wake up init whether that's
        // necessary or not. init may miss this wakeup, but that seems
        // harmless.
        let init_guard = self.init_process().lock.lock();
        self.wake_up_process(self.init_process());
        drop(init_guard);

        // grab a copy of p->parent, to ensure that we unlock the same
        // parent we locked. in case our parent gives us away to init while
        // we're waiting for the parent lock. we may then race with an
        // exiting parent, but the result will be a harmless spurious wakeup
        // to a dead or wrong process; proc structs are never re-allocated
        // as anything else.
        let proc_guard = process.lock.lock();
        let parent = process.info().parent.unwrap();
        drop(proc_guard);

        // we need the parent's lock in order to wake it up from wait().
        // the parent-then-child rule says we have to lock it first.
        let parent_guard = parent.lock.lock();
        let self_guard = process.lock.lock();

        // Give any children to init.
        self.reparent(process);

        // Parent might be sleeping in wait().
        self.wake_up_process(parent);

        process.info().exit_state = exit_state;
        process.info().state = ZOMBIE;

        drop(parent_guard);

        unsafe {
            CPU_MANAGER.my_cpu_mut().scheduled();
        }

        drop(self_guard);
        panic!("zombie exit");
    }

    // Pass p's abandoned children to init.
    // Caller must hold p->lock.
    fn reparent(&self, process: &Process) {
        for pp in self.processes.iter() {
            // this code uses pp->parent without holding pp->lock.
            // acquiring the lock first could cause a deadlock
            // if pp or a child of pp were also in exit()
            // and about to try to lock p.
            if pp.info().parent.map_or(false, |it| it as *const _ == process as *const _) {
                // pp->parent can't change between the check and the acquire()
                // because only the parent changes it, and we're the parent.
                let guard = pp.lock.lock();
                pp.info().parent = Some(self.init_process());
                // we should wake up init here, but that would require
                // initproc->lock, which would be a deadlock, since we hold
                // the lock on one of init's children (pp). this is why
                // exit() always wakes init (before acquiring any locks).
                drop(guard);
            }
        }
    }
}

// A fork child's very first scheduling by scheduler()
// will swtch to forkret.
unsafe fn fork_return() {
    static mut IS_FIRST_PROCESS: bool = true;

    // Still holding p->lock from scheduler.
    CPU_MANAGER.my_proc().lock.unlock();

    if IS_FIRST_PROCESS {
        // File system initialization must be run in the context of a
        // regular process (e.g., because it calls sleep), and thus cannot
        // be run from main().
        IS_FIRST_PROCESS = false;
        file_system_init(ROOT_DEV);
    }

    user_trap_return();
}