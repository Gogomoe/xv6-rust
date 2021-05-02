pub mod layout;
pub mod physical_memory;

pub use physical_memory::PHYSICAL_MEMORY;

pub const PAGE_SIZE: usize = 4096;

pub fn page_round_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1))
}

pub fn page_round_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

pub unsafe fn memset(addr: usize, data: u8, size: usize) {
    let mut p = addr as *mut u8;
    for _ in 0..size {
        *p = data;
        p = p.offset(1);
    }
}