use alloc::collections::BTreeMap;
use alloc::string::String;

use lazy_static::lazy_static;

use crate::memory::copy_in_string;
use crate::process::CPU_MANAGER;
use crate::syscall::exec::sys_exec;

pub mod exec;

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
    }
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