use core::ptr;

use crate::memory::{ActivePageTable, Page, page_round_up, PAGE_SIZE, PHYSICAL_MEMORY};
use crate::memory::layout::{TRAMPOLINE, TRAPFRAME};
use crate::memory::page_table::PageEntryFlags;
use crate::process::trap_frame::TrapFrame;

extern {
    fn trampoline();
}

static INIT_CODE: [u8; 52] = [
    0x17, 0x05, 0x00, 0x00, 0x13, 0x05, 0x45, 0x02,
    0x97, 0x05, 0x00, 0x00, 0x93, 0x85, 0x35, 0x02,
    0x93, 0x08, 0x70, 0x00, 0x73, 0x00, 0x00, 0x00,
    0x93, 0x08, 0x20, 0x00, 0x73, 0x00, 0x00, 0x00,
    0xef, 0xf0, 0x9f, 0xff, 0x2f, 0x69, 0x6e, 0x69,
    0x74, 0x00, 0x00, 0x24, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00
];


pub unsafe fn init(page_table: &mut ActivePageTable) {
    let frame = PHYSICAL_MEMORY.alloc().unwrap();
    ptr::write_bytes(frame.addr() as *mut u8, 0, PAGE_SIZE);
    ptr::copy(&INIT_CODE as *const [u8; 52], frame.addr() as *mut [u8; 52], 1);

    let wrxu: PageEntryFlags = PageEntryFlags::WRITEABLE | PageEntryFlags::READABLE | PageEntryFlags::EXECUTABLE | PageEntryFlags::USER;
    let result = page_table.map(Page::from_virtual_address(0), frame, wrxu);
    assert!(result.is_ok());
}

pub fn alloc_page_table(trapframe: *mut TrapFrame) -> Option<ActivePageTable> {
    let page_table = ActivePageTable::new();
    if page_table.is_none() {
        return None;
    }

    // map the trampoline code (for system call return)
    // at the highest user virtual address.
    // only the supervisor uses it, on the way
    // to/from user space, so not PTE_U.
    let mut page_table = page_table.unwrap();

    let trampoline = trampoline as usize;
    let rw = PageEntryFlags::READABLE | PageEntryFlags::WRITEABLE;
    let rx = PageEntryFlags::READABLE | PageEntryFlags::EXECUTABLE;

    let result = page_table.map_pages(TRAMPOLINE, trampoline, PAGE_SIZE, rx);
    if !result {
        free_page_table(page_table, 0);
        return None;
    }

    let result = page_table.map_pages(TRAPFRAME, trapframe as usize, PAGE_SIZE, rw);
    if !result {
        page_table.unmap_pages(TRAMPOLINE, PAGE_SIZE);
        free_page_table(page_table, 0);
        return None;
    }

    Some(page_table)
}

pub fn free_page_table(mut page_table: ActivePageTable, size: usize) {
    assert_eq!(size % PAGE_SIZE, 0);
    for v_addr in (0..size).step_by(PAGE_SIZE) {
        page_table.unmap(Page::from_virtual_address(v_addr));
    }
    page_table.free();
}

// Allocate PTEs and physical memory to grow process from oldsz to
// newsz, which need not be page aligned.  Returns new size or 0 on error.
pub fn alloc_user_virtual_memory(page_table: &mut ActivePageTable, mut old_size: usize, new_size: usize) -> Option<usize> {
    if new_size < old_size {
        return Some(old_size);
    }

    old_size = page_round_up(old_size);
    for addr in (old_size..new_size).step_by(PAGE_SIZE) {
        let frame = PHYSICAL_MEMORY.alloc();
        if frame.is_none() {
            dealloc_user_virtual_memory(page_table, addr, old_size);
            return None;
        }
        let frame = frame.unwrap();
        unsafe {
            ptr::write_bytes(frame.addr() as *mut u8, 0, PAGE_SIZE);
        }
        let wrxu: PageEntryFlags = PageEntryFlags::WRITEABLE | PageEntryFlags::READABLE | PageEntryFlags::EXECUTABLE | PageEntryFlags::USER;
        let map_result = page_table.map(Page::from_virtual_address(addr), frame, wrxu);
        if map_result.is_err() {
            PHYSICAL_MEMORY.dealloc(map_result.err().unwrap());
            dealloc_user_virtual_memory(page_table, addr, old_size);
            return None;
        }
    }

    return Some(new_size);
}

// Deallocate user pages to bring the process size from oldsz to
// newsz.  oldsz and newsz need not be page-aligned, nor does newsz
// need to be less than oldsz.  oldsz can be larger than the actual
// process size.  Returns the new process size.
pub fn dealloc_user_virtual_memory(page_table: &mut ActivePageTable, old_size: usize, new_size: usize) -> usize {
    if new_size >= old_size {
        return old_size;
    }

    for addr in (page_round_up(new_size)..page_round_up(old_size)).step_by(PAGE_SIZE) {
        page_table.unmap(Page::from_virtual_address(addr));
    }

    return new_size;
}

// mark a PTE invalid for user access.
// used by exec for the user stack guard page.
pub fn make_guard_page(page_table: &mut ActivePageTable, va: usize) {
    let page = Page::from_virtual_address(va);
    let mut flags = page_table.read_flags(&page).unwrap();
    flags &= !PageEntryFlags::USER;
    page_table.write_flags(&page, flags);
}