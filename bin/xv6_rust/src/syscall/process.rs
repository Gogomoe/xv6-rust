use core::intrinsics::size_of;

use crate::memory::either_copy_out;
use crate::process::PROCESS_MANAGER;
use crate::syscall::read_arg_usize;

pub fn sys_exit() -> u64 {
    let exit_code = read_arg_usize(0);
    PROCESS_MANAGER.exit(exit_code as i32);
    return 0; // not reached here
}

pub fn sys_fork() -> u64 {
    PROCESS_MANAGER.fork().map_or(u64::max_value(), |it| it as u64)
}

pub fn sys_wait() -> u64 {
    match PROCESS_MANAGER.wait_child() {
        None => { u64::max_value() }
        Some((pid, exit_state)) => {
            let addr = read_arg_usize(0);
            either_copy_out(true, addr, &exit_state as *const _ as usize, size_of::<i32>());
            pid as u64
        }
    }
}