use alloc::collections::BTreeMap;

use lazy_static::lazy_static;

use crate::process::CPU_MANAGER;
use crate::syscall::file::sys_exec;

pub mod file;

#[derive(Clone)]
pub struct SystemCall {
    name: &'static str,
    id: usize,
    func: fn() -> u64,
}

static SYSCALL_EXEC: SystemCall = SystemCall { name: "exec", id: 7, func: sys_exec };

lazy_static! {
    pub static ref SYSTEM_CALL: BTreeMap<usize, SystemCall> = {
        let mut map: BTreeMap<usize, SystemCall> = BTreeMap::new();
        let mut insert = |it: SystemCall| {
            assert!(map.get(&it.id).is_none());
            map.insert(it.id, it);
        };
        insert(SYSCALL_EXEC.clone());
        map
    };
}

pub fn system_call_init() {
    assert!(SYSTEM_CALL.get(&SYSCALL_EXEC.id).is_some())
}

pub fn system_call() {
    let process = CPU_MANAGER.my_proc().unwrap();
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
    }
}