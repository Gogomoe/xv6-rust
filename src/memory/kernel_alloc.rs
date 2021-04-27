use core::ptr::null_mut;

use spin::Mutex;

use super::*;
use super::layout::*;

extern {
    fn end();
}

#[repr(C)]
struct FreePage {
    next: *mut FreePage
}

#[repr(C)]
struct FreeMemory {
    head: *mut FreePage
}

unsafe impl Send for FreeMemory {}

static KERNEL_MEMORY: Mutex<FreeMemory> = Mutex::new(FreeMemory { head: null_mut() });

pub fn kernel_init() {
    let phy_end = end as usize;
    println!("physical memory: {:#x} - {:#x}", phy_end, PHY_STOP);
    free_range(phy_end, PHY_STOP);
}

pub fn free_range(pa_start: usize, pa_end: usize) {
    let mut addr = page_round_up(pa_start);
    while addr + PAGE_SIZE <= pa_end {
        kernel_free(addr);
        addr += PAGE_SIZE;
    }
}

pub fn kernel_free(pa: usize) {
    let phy_end = end as usize;
    assert!(pa % PAGE_SIZE == 0 && pa >= phy_end && pa < PHY_STOP);

    // TODO fill junk

    unsafe {
        let page = pa as *mut FreePage;
        let mut lock = KERNEL_MEMORY.lock();
        let mut kernel = &mut *lock;

        let next_page = kernel.head;
        (*page).next = next_page;
        kernel.head = page
    }
}