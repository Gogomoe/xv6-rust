use alloc::string::String;
use alloc::vec::Vec;
use core::ptr;

use cstr_core::{c_char, CStr};

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

const SATP_SV39: usize = 8 << 60;

#[inline]
pub fn make_satp(page_table: &ActivePageTable) -> usize {
    SATP_SV39 | (page_table.addr() >> 12)
}

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
    return if user_dst {
        let data = proc.data();
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
    return if user_src {
        let data = proc.data();
        let pt = data.page_table.as_ref().unwrap();
        copy_in(pt, dst, src, len)
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
pub fn copy_in(pt: &ActivePageTable, mut dst: usize, mut src_va: usize, mut len: usize) -> bool {
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

        unsafe {
            ptr::copy((pa0 + src_va - va0) as *const u8, dst as *mut u8, n);
        }

        len -= n;
        dst += n;
        src_va = va0 + PAGE_SIZE;
    }
    true
}

// Copy a null-terminated string from user to kernel.
// Copy bytes to dst from virtual address srcva in a given page table,
// until a '\0', or max.
// Return 0 on success, -1 on error.
pub fn copy_in_string(pt: &ActivePageTable, mut va: usize) -> Option<String> {
    let mut pa = match pt.translate(va) {
        Some(x) => x,
        None => return None,
    };
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let byte = unsafe { *(pa as *const u8) }.clone();
        bytes.push(byte);
        if byte == b'\0' {
            break;
        }
        va += 1;
        pa += 1;
        if va % PAGE_SIZE == 0 {
            pa = match pt.translate(va) {
                Some(x) => x,
                None => return None
            }
        }
    }

    unsafe {
        let result = CStr::from_ptr(bytes.as_mut_ptr() as *mut c_char).to_string_lossy().into_owned();

        Some(result)
    }
}