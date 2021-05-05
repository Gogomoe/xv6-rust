#![allow(dead_code)]

use alloc::string::String;
use core::fmt;

use spin::Mutex;

use crate::memory::{ActivePageTable, PHYSICAL_MEMORY};
use crate::process::context::Context;
use crate::process::trap_frame::TrapFrame;
use core::cell::RefCell;

/// private data for process, no lock needs
pub struct ProcessData {
    pub kernel_stack: usize,
    pub size: usize,
    pub page_table: ActivePageTable,
    pub trap_frame: *mut TrapFrame,
    pub context: Context,
    pub name: String,

    // TODO, open file, current dir
}

unsafe impl Send for ProcessData {}

impl ProcessData {
    pub fn new() -> ProcessData {
        ProcessData {
            kernel_stack: 0,
            size: 0,
            page_table: ActivePageTable::new().unwrap(),
            trap_frame: PHYSICAL_MEMORY.alloc().unwrap().addr() as *mut TrapFrame,
            context: Context::new(),
            name: String::from(""),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ProcessState { UNUSED, SLEEPING, RUNNABLE, RUNNING, ZOMBIE }

impl fmt::Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

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
    pub data: RefCell<ProcessData>,
    pub info: Mutex<ProcessInfo>,
}

unsafe impl Sync for Process {}

impl Process {
    pub fn new() -> Process {
        Process {
            data: RefCell::new(ProcessData::new()),
            info: Mutex::new(ProcessInfo::new()),
        }
    }
}