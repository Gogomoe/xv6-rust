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
    todo!()
}