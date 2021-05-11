use crate::memory::{ActivePageTable, Page, PAGE_SIZE, PHYSICAL_MEMORY};
use crate::memory::layout::{TRAMPOLINE, TRAPFRAME};
use crate::memory::page_table::PageEntryFlags;
use crate::process::trap_frame::TrapFrame;
use core::intrinsics::{size_of, size_of_val};
use core::ptr;

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

    let wrxu = PageEntryFlags::WRITEABLE | PageEntryFlags::READABLE | PageEntryFlags::EXECUTABLE | PageEntryFlags::USER;
    page_table.map(Page::from_virtual_address(0), frame, wrxu);
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
    PHYSICAL_MEMORY.free(&page_table as *const _ as usize);
}