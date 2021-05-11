use core::ptr;

pub use kernel_virtual_memory::KERNEL_PAGETABLE;
pub use physical_memory::Frame;
pub use physical_memory::PHYSICAL_MEMORY;
pub use virtual_memory::ActivePageTable;
pub use virtual_memory::Page;

use crate::process::CPU_MANAGER;

pub mod layout;
pub mod physical_memory;
pub mod virtual_memory;
pub mod page_table;
pub mod kernel_virtual_memory;
pub mod kernel_heap;
pub mod user_virtual_memory;

pub const PAGE_SIZE: usize = 4096;

pub type PhysicalAddress = usize;
pub type VirtualAddress = usize;

pub fn page_round_up(addr: usize) -> usize {
    (addr + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1))
}

pub fn page_round_down(addr: usize) -> usize {
    addr & !(PAGE_SIZE - 1)
}

// Copy to either a user address, or kernel address,
// depending on usr_dst.
// Returns 0 on success, -1 on error.
pub fn either_copy_out(user_dst: bool, dst: usize, src: usize, len: usize) -> bool {
    let proc = CPU_MANAGER.my_proc();
    let proc = unsafe { proc.as_ref().unwrap() };
    return if user_dst {
        let data = (*proc).data.borrow();
        let pt = data.page_table.as_ref().unwrap();
        unsafe {
            copy_out(pt, dst, src, len)
        }
    } else {
        unsafe {
            ptr::copy(src as *mut u8, dst as *mut u8, len);
        }
        true
    };
}

// Copy from either a user address, or kernel address,
// depending on usr_src.
// Returns 0 on success, -1 on error.
pub fn either_copy_in(user_src: bool, dst: usize, src: usize, len: usize) -> bool {
    let proc = CPU_MANAGER.my_proc();
    let proc = unsafe { proc.as_ref().unwrap() };
    return if user_src {
        let data = (*proc).data.borrow();
        let pt = data.page_table.as_ref().unwrap();
        unsafe {
            copy_in(pt, dst, src, len)
        }
    } else {
        unsafe {
            ptr::copy(src as *mut u8, dst as *mut u8, len);
        }
        true
    };
}

// Copy from kernel to user.
// Copy len bytes from src to virtual address dstva in a given page table.
// Return 0 on success, -1 on error.
pub unsafe fn copy_out(pt: &ActivePageTable, mut dst_va: usize, mut src: usize, mut len: usize) -> bool {
    while len > 0 {
        let va0 = page_round_down(dst_va);
        let pa0 = pt.translate(va0);
        if pa0.is_none() {
            return false;
        }
        let pa0 = pa0.unwrap();

        let mut n = PAGE_SIZE - (dst_va - va0);
        if n > len {
            n = len;
        }

        ptr::copy(src as *const u8, (pa0 + dst_va - va0) as *mut u8, n);

        len -= n;
        src += n;
        dst_va = va0 + PAGE_SIZE;
    }
    true
}

// Copy from user to kernel.
// Copy len bytes to dst from virtual address srcva in a given page table.
// Return 0 on success, -1 on error.
pub unsafe fn copy_in(pt: &ActivePageTable, mut dst: usize, mut src_va: usize, mut len: usize) -> bool {
    while len > 0 {
        let va0 = page_round_down(src_va);
        let pa0 = pt.translate(va0);
        if pa0.is_none() {
            return false;
        }
        let pa0 = pa0.unwrap();

        let mut n = PAGE_SIZE - (src_va - va0);
        if n > len {
            n = len;
        }

        ptr::copy((pa0 + src_va - va0) as *const u8, dst as *mut u8, n);

        len -= n;
        dst += n;
        src_va = va0 + PAGE_SIZE;
    }
    true
}