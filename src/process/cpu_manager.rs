use alloc::vec::Vec;
use core::ptr::null_mut;

use lazy_static::lazy_static;
use spin::Mutex;

use crate::param::MAX_CPU_NUMBER;
use crate::process::context::Context;
use crate::process::process::Process;
use crate::riscv::read_tp;

pub struct Cpu {
    pub process: *const Process,
    pub context: Context,
}

unsafe impl Send for Cpu {}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            process: null_mut(),
            context: Context::new(),
        }
    }
}

pub struct CpuManager {
    cpus: Vec<Mutex<Cpu>>,
}

lazy_static! {
    pub static ref CPU_MANAGER: CpuManager = {
        let mut cpus = Vec::new();
        for _ in 0..MAX_CPU_NUMBER  {
            cpus.push(Mutex::new(Cpu::new()));
        }

        let manager = CpuManager {
            cpus,
        };

        manager
    };
}

impl CpuManager {
    pub fn my_cpu(&self) -> &Mutex<Cpu> {
        &self.cpus[cpu_id()]
    }
}

pub fn cpu_id() -> usize {
    unsafe {
        let id = read_tp();
        return id;
    }
}
