pub use process_manager::PROCESS_MANAGER;
use crate::riscv::read_tp;

pub mod process;
pub mod process_manager;

pub fn cpu_id() -> usize {
    unsafe {
        let id = read_tp();
        return id;
    }
}