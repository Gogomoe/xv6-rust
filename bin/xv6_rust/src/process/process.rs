#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::fmt;

use bitflags::_core::ptr::null_mut;

use crate::file_system::file::File;
use crate::file_system::inode::INode;
use crate::memory::ActivePageTable;
use crate::process::context::Context;
use crate::process::trap_frame::TrapFrame;
use crate::spin_lock::SpinLock;

/// private data for process, no lock needs
pub struct ProcessData {
    pub kernel_stack: usize,
    pub size: usize,
    pub page_table: Option<ActivePageTable>,
    pub trap_frame: *mut TrapFrame,
    pub context: Context,
    pub current_dir: Option<&'static INode>,
    pub name: String,
    pub open_file: Vec<&'static File>,
}

unsafe impl Send for ProcessData {}

impl ProcessData {
    pub const fn new() -> ProcessData {
        ProcessData {
            kernel_stack: 0,
            size: 0,
            page_table: None,
            trap_frame: null_mut(),
            context: Context::new(),
            current_dir: None,
            name: String::new(),
            open_file: Vec::new(),
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
    pub killed: bool,
    pub exit_state: i32,
    pub parent: Option<&'static Process>,
}

impl ProcessInfo {
    pub const fn new() -> ProcessInfo {
        ProcessInfo {
            state: ProcessState::UNUSED,
            channel: 0,
            pid: 0,
            killed: false,
            exit_state: 0,
            parent: None,
        }
    }
}

pub struct Process {
    pub lock: SpinLock<()>,
    data: UnsafeCell<ProcessData>,
    info: UnsafeCell<ProcessInfo>,
}

unsafe impl Sync for Process {}

impl Process {
    pub const fn new() -> Process {
        Process {
            lock: SpinLock::new((), "process"),
            data: UnsafeCell::new(ProcessData::new()),
            info: UnsafeCell::new(ProcessInfo::new()),
        }
    }

    pub fn data(&self) -> &mut ProcessData {
        unsafe { self.data.get().as_mut() }.unwrap()
    }

    pub fn info(&self) -> &mut ProcessInfo {
        unsafe { self.info.get().as_mut() }.unwrap()
    }
}