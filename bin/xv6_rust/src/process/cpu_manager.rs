use core::cell::RefCell;
use core::ptr::null_mut;

use param_lib::MAX_CPU_NUMBER;

use crate::process::context::Context;
use crate::process::process::Process;
use crate::process::process::ProcessState::{RUNNABLE, RUNNING, SLEEPING};
use crate::riscv::{intr_get, intr_off, intr_on, read_tp};
use crate::spin_lock::SpinLockGuard;

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
    pub const fn new() -> Cpu {
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

    pub unsafe fn scheduled(&mut self) {
        extern {
            fn swtch(old: *mut Context, new: *mut Context);
        }
        let process = self.my_proc().as_ref().unwrap();

        assert!(process.lock.holding());
        assert_eq!(self.off_depth, 1);
        assert_ne!(process.info().state, RUNNING);
        assert!(!intr_get());

        let old_intr = self.interrupt_enable;
        swtch(&mut process.data().context, &mut self.context);
        self.interrupt_enable = old_intr;
    }

    pub fn yield_self(&mut self) {
        let process = unsafe { self.my_proc().as_ref() }.unwrap();
        let guard = process.lock.lock();
        assert_eq!(process.info().state, RUNNING);
        process.info().state = RUNNABLE;
        unsafe {
            self.scheduled();
        }
        drop(guard);
    }

    /// NOTICE: acquire lock after sleep
    pub fn sleep<T>(&mut self, channel: usize, guard: SpinLockGuard<T>) {
        let proc = self.my_proc();
        assert!(!proc.is_null());
        let process = unsafe { proc.as_ref().unwrap() };

        let proc_guard = process.lock.lock();
        drop(guard);

        process.info().channel = channel;
        process.info().state = SLEEPING;

        unsafe {
            self.scheduled();
        }

        process.info().channel = 0;
        drop(proc_guard);
    }
}

pub struct CpuManager {
    cpus: [RefCell<Cpu>; MAX_CPU_NUMBER],
}

unsafe impl Sync for CpuManager {}

pub static CPU_MANAGER: CpuManager = CpuManager::new();

impl CpuManager {
    const fn new() -> CpuManager {
        CpuManager {
            cpus: array![_ => RefCell::new(Cpu::new()); MAX_CPU_NUMBER]
        }
    }

    pub fn my_cpu(&self) -> &mut Cpu {
        unsafe {
            self.cpus[cpu_id()].as_ptr().as_mut().unwrap()
        }
    }

    pub fn my_proc(&self) -> &Process {
        let cpu = self.my_cpu();
        return unsafe { cpu.my_proc().as_ref() }.unwrap();
    }
}

pub fn cpu_id() -> usize {
    unsafe {
        let id = read_tp();
        return id;
    }
}
