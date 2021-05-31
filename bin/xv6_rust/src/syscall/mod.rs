use alloc::collections::BTreeMap;
use alloc::string::String;

use lazy_static::lazy_static;

use crate::memory::copy_in_string;
use crate::process::CPU_MANAGER;
use crate::syscall::exec::sys_exec;
use crate::syscall::file::{sys_close, sys_dup, sys_mknod, sys_open, sys_chdir, sys_read, sys_write, sys_fstat, sys_mkdir};
use crate::syscall::process::{sys_exit, sys_fork, sys_sbrk, sys_wait};

pub mod exec;
pub mod file;
pub mod process;

#[derive(Clone)]
pub struct SystemCall {
    name: &'static str,
    id: usize,
    func: fn() -> u64,
}

static SYSCALL_FORK: SystemCall = SystemCall { name: "fork", id: 1, func: sys_fork };
static SYSCALL_EXIT: SystemCall = SystemCall { name: "exit", id: 2, func: sys_exit };
static SYSCALL_WAIT: SystemCall = SystemCall { name: "wait", id: 3, func: sys_wait };
static SYSCALL_READ: SystemCall = SystemCall { name: "read", id: 5, func: sys_read };
static SYSCALL_EXEC: SystemCall = SystemCall { name: "exec", id: 7, func: sys_exec };
static SYSCALL_FSTAT: SystemCall = SystemCall { name: "stat", id: 8, func: sys_fstat };
static SYSCALL_CHDIR: SystemCall = SystemCall { name: "chdir", id: 9, func: sys_chdir };
static SYSCALL_DUP: SystemCall = SystemCall { name: "dup", id: 10, func: sys_dup };
static SYSCALL_SBRK: SystemCall = SystemCall { name: "sbrk", id: 12, func: sys_sbrk };
static SYSCALL_OPEN: SystemCall = SystemCall { name: "open", id: 15, func: sys_open };
static SYSCALL_WRITE: SystemCall = SystemCall { name: "write", id: 16, func: sys_write };
static SYSCALL_MKNOD: SystemCall = SystemCall { name: "mknod", id: 17, func: sys_mknod };
static SYSCALL_MKDIR: SystemCall = SystemCall { name: "mkdir", id: 20, func: sys_mkdir };
static SYSCALL_CLOSE: SystemCall = SystemCall { name: "close", id: 21, func: sys_close };

lazy_static! {
    pub static ref SYSTEM_CALL: BTreeMap<usize, SystemCall> = {
        let mut map: BTreeMap<usize, SystemCall> = BTreeMap::new();
        let mut insert = |it: SystemCall| {
            assert!(map.get(&it.id).is_none());
            map.insert(it.id, it);
        };
        insert(SYSCALL_FORK.clone());
        insert(SYSCALL_EXIT.clone());
        insert(SYSCALL_WAIT.clone());
        insert(SYSCALL_READ.clone());
        insert(SYSCALL_EXEC.clone());
        insert(SYSCALL_FSTAT.clone());
        insert(SYSCALL_CHDIR.clone());
        insert(SYSCALL_DUP.clone());
        insert(SYSCALL_SBRK.clone());
        insert(SYSCALL_OPEN.clone());
        insert(SYSCALL_WRITE.clone());
        insert(SYSCALL_MKNOD.clone());
        insert(SYSCALL_MKDIR.clone());
        insert(SYSCALL_CLOSE.clone());
        map
    };
}

pub fn system_call_init() {
    assert!(SYSTEM_CALL.get(&SYSCALL_EXEC.id).is_some())
}

pub fn system_call() {
    let process = CPU_MANAGER.my_proc();
    let trap_frame = unsafe { process.data().trap_frame.as_mut() }.unwrap();
    let num = trap_frame.a7 as usize;

    trap_frame.a0 = match SYSTEM_CALL.get(&num) {
        Some(it) => {
            it.func.call(())
        }
        None => {
            println!("{} {}: unknown system call {}", process.info().pid, process.data().name, num);
            u64::max_value()
        }
    };
}

fn read_arg_content(pos: usize) -> u64 {
    let process = CPU_MANAGER.my_proc();
    unsafe {
        match pos {
            0 => { (*process.data().trap_frame).a0 }
            1 => { (*process.data().trap_frame).a1 }
            2 => { (*process.data().trap_frame).a2 }
            3 => { (*process.data().trap_frame).a3 }
            4 => { (*process.data().trap_frame).a4 }
            5 => { (*process.data().trap_frame).a5 }
            _ => { panic!("out of arg bound {}", pos) }
        }
    }
}

pub fn read_arg_usize(pos: usize) -> usize {
    read_arg_content(pos) as usize
}

pub fn read_arg_string(pos: usize) -> Option<String> {
    let page_table = CPU_MANAGER.my_proc().data().page_table.as_ref().unwrap();
    let user_addr = read_arg_usize(pos);

    return copy_in_string(page_table, user_addr);
}