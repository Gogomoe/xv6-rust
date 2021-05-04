#![allow(dead_code)]

use alloc::string::String;

use spin::Mutex;

use crate::memory::{ActivePageTable, PHYSICAL_MEMORY};
use core::ptr::null_mut;

pub struct Context {
    ra: u64,
    sp: u64,

    // callee-saved
    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
    s10: u64,
    s11: u64,
}

impl Context {
    pub fn new() -> Context {
        Self {
            ra: 0,
            sp: 0,
            s0: 0,
            s1: 0,
            s2: 0,
            s3: 0,
            s4: 0,
            s5: 0,
            s6: 0,
            s7: 0,
            s8: 0,
            s9: 0,
            s10: 0,
            s11: 0,
        }
    }

    pub fn clear(&mut self) {
        self.ra = 0;
        self.sp = 0;
        self.s0 = 0;
        self.s1 = 0;
        self.s2 = 0;
        self.s3 = 0;
        self.s4 = 0;
        self.s5 = 0;
        self.s6 = 0;
        self.s7 = 0;
        self.s8 = 0;
        self.s9 = 0;
        self.s10 = 0;
        self.s11 = 0;
    }
}

pub struct TrapFrame {
    /*   0 */ kernel_satp: u64,
    /* kernel page table */
    /*   8 */ kernel_sp: u64,
    /* top of process's kernel stack */
    /*  16 */ kernel_trap: u64,
    /* usertrap() */
    /*  24 */ epc: u64,
    /* saved user program counter */
    /*  32 */ kernel_hartid: u64,
    /* saved kernel tp */
    /*  40 */ ra: u64,
    /*  48 */ sp: u64,
    /*  56 */ gp: u64,
    /*  64 */ tp: u64,
    /*  72 */ t0: u64,
    /*  80 */ t1: u64,
    /*  88 */ t2: u64,
    /*  96 */ s0: u64,
    /* 104 */ s1: u64,
    /* 112 */ a0: u64,
    /* 120 */ a1: u64,
    /* 128 */ a2: u64,
    /* 136 */ a3: u64,
    /* 144 */ a4: u64,
    /* 152 */ a5: u64,
    /* 160 */ a6: u64,
    /* 168 */ a7: u64,
    /* 176 */ s2: u64,
    /* 184 */ s3: u64,
    /* 192 */ s4: u64,
    /* 200 */ s5: u64,
    /* 208 */ s6: u64,
    /* 216 */ s7: u64,
    /* 224 */ s8: u64,
    /* 232 */ s9: u64,
    /* 240 */ s10: u64,
    /* 248 */ s11: u64,
    /* 256 */ t3: u64,
    /* 264 */ t4: u64,
    /* 272 */ t5: u64,
    /* 280 */ t6: u64,
}

/// private data for process, no lock needs
pub struct ProcessData {
    pub kernel_stack: usize,
    pub size: usize,
    pub page_table: *mut ActivePageTable,
    pub trap_frame: *mut TrapFrame,
    pub context: Context,
    pub name: String,

    // TODO, open file, current dir
}

impl ProcessData {
    pub fn new() -> ProcessData {
        ProcessData {
            kernel_stack: 0,
            size: 0,
            page_table: null_mut(),
            trap_frame: null_mut(),
            context: Context::new(),
            name: String::from(""),
        }
    }
}

pub enum ProcessState { UNUSED, SLEEPING, RUNNABLE, RUNNING, ZOMBIE }

/// public data for process, need lock
pub struct ProcessInfo {
    pub state: ProcessState,
    pub channel: usize,
    pub pid: usize,
}

impl ProcessInfo {
    pub fn new() -> ProcessInfo {
        ProcessInfo {
            state: ProcessState::UNUSED,
            channel: 0,
            pid: 0,
        }
    }
}

pub struct Process {
    pub data: ProcessData,
    pub info: Mutex<ProcessInfo>,
}

impl Process {
    pub fn new() -> Process {
        Process {
            data: ProcessData::new(),
            info: Mutex::new(ProcessInfo::new()),
        }
    }
}