use alloc::vec::Vec;
use core::cell::RefCell;
use core::ptr::null_mut;

use lazy_static::lazy_static;
use spin::MutexGuard;

use crate::param::MAX_CPU_NUMBER;
use crate::process::context::Context;
use crate::process::process::{Process, ProcessInfo};
use crate::process::process::ProcessState::{RUNNABLE, RUNNING, SLEEPING};
use crate::riscv::{intr_get, intr_off, intr_on, read_tp};

pub struct Cpu {
    pub process: *const Process,
    pub context: Context,

    /// Depth of push_off() nesting
    off_depth: usize,
    /// Were interrupts enabled before push_off()?
    interrupt_enable: bool,
}

unsafe impl Send for Cpu {}

unsafe impl Sync for Cpu {}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            process: null_mut(),
            context: Context::new(),
            off_depth: 0,
            interrupt_enable: false,
        }
    }

    pub fn my_proc(&mut self) -> *const Process {
        self.push_off();
        let process = self.process;
        self.pop_off();
        return process;
    }

    // push_off/pop_off are like intr_off()/intr_on() except that they are matched:
    // it takes two pop_off()s to undo two push_off()s.  Also, if interrupts
    // are initially off, then push_off, pop_off leaves them off.
    pub fn push_off(&mut self) {
        unsafe {
            let old_intr = intr_get();

            intr_off();
            if self.off_depth == 0 {
                self.interrupt_enable = old_intr;
            }
            self.off_depth += 1;
        }
    }

    pub fn pop_off(&mut self) {
        unsafe {
            assert!(!intr_get());
            assert!(self.off_depth >= 1);
            self.off_depth -= 1;
            if self.off_depth == 0 && self.interrupt_enable {
                intr_on();
            }
        }
    }

    pub unsafe fn scheduled(&mut self, guard: &MutexGuard<ProcessInfo>) {
        extern {
            fn swtch(old: *mut Context, new: *mut Context);
        }
        let proc = self.my_proc();
        let info = &**guard;
        assert!(!proc.is_null());
        assert_ne!(info.state, RUNNING);
        assert!(!intr_get());

        let proc = proc.as_ref().unwrap();
        let old_intr = self.interrupt_enable;
        swtch(&mut proc.data.borrow_mut().context, &mut self.context);
        self.interrupt_enable = old_intr;
    }

    pub fn yield_self(&mut self) {
        let proc = self.my_proc();
        let proc = unsafe { proc.as_ref().unwrap() };
        let mut guard = proc.info.lock();
        assert_eq!(guard.state, RUNNING);
        guard.state = RUNNABLE;
        unsafe {
            self.scheduled(&guard);
        }
    }

    /// NOTICE: acquire lock after sleep
    pub fn sleep<T>(&mut self, channel: usize, guard: MutexGuard<T>) {
        let proc = self.my_proc();
        assert!(!proc.is_null());
        let proc = unsafe { proc.as_ref().unwrap() };

        let mut info_lock = proc.info.lock();
        drop(guard);

        info_lock.channel = channel;
        info_lock.state = SLEEPING;

        unsafe {
            self.scheduled(&info_lock);
        }

        info_lock.channel = 0;
        drop(info_lock);
    }
}

pub struct CpuManager {
    cpus: Vec<RefCell<Cpu>>,
}

unsafe impl Sync for CpuManager {}

lazy_static! {
    pub static ref CPU_MANAGER: CpuManager = {
        let mut cpus = Vec::new();
        for _ in 0..MAX_CPU_NUMBER  {
            cpus.push(RefCell::new(Cpu::new()));
        }

        let manager = CpuManager {
            cpus,
        };

        manager
    };
}

impl CpuManager {
    pub fn my_cpu(&self) -> &Cpu {
        unsafe {
            self.cpus[cpu_id()].as_ptr().as_ref().unwrap()
        }
    }

    pub fn my_cpu_mut(&self) -> &mut Cpu {
        unsafe {
            self.cpus[cpu_id()].as_ptr().as_mut().unwrap()
        }
    }

    pub fn my_proc(&self) -> *const Process {
        let cpu = self.my_cpu_mut();
        return cpu.my_proc();
    }
}

pub fn cpu_id() -> usize {
    unsafe {
        let id = read_tp();
        return id;
    }
}
