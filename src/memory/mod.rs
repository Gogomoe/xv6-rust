pub use kernel_virtual_memory::KERNEL_PAGETABLE;
pub use physical_memory::Frame;
pub use physical_memory::PHYSICAL_MEMORY;
pub use virtual_memory::ActivePageTable;
pub use virtual_memory::Page;

pub mod layout;
pub mod physical_memory;
pub mod virtual_memory;
pub mod page_table;
pub mod kernel_virtual_memory;
pub mod kernel_heap;

pub const PAGE_SIZE: usize = 4096;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub fn page_round_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1))
}

pub fn page_round_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}