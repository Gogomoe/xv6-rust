use lazy_static::lazy_static;
use spin::Mutex;

use crate::memory::{ActivePageTable, Frame, Page, page_round_down, PAGE_SIZE, PHYSICAL_MEMORY};
use crate::memory::layout::{CLINT, KERNEL_BASE, KERNEL_HEAP_SIZE, KERNEL_HEAP_START, PHY_STOP, PLIC, TRAMPOLINE, UART0, VIRTIO0};
use crate::memory::page_table::PageEntryFlags;
use crate::riscv::{sfence_vma, write_satp};

extern {
    fn etext();
    fn trampoline();
}

unsafe impl Sync for ActivePageTable {}

unsafe impl Send for ActivePageTable {}

lazy_static! {
    pub static ref KERNEL_PAGETABLE: Mutex<ActivePageTable> = {
        let mut page_table = ActivePageTable::new().unwrap();

        let rw = PageEntryFlags::READABLE | PageEntryFlags::WRITEABLE;
        let rx = PageEntryFlags::READABLE | PageEntryFlags::EXECUTABLE;

        let etext = etext as usize;
        let trampoline = trampoline as usize;

        page_table.map_pages(UART0, UART0, PAGE_SIZE, rw);
        page_table.map_pages(VIRTIO0, VIRTIO0, PAGE_SIZE, rw);
        page_table.map_pages(CLINT, CLINT, 0x10000, rw);
        page_table.map_pages(PLIC, PLIC, 0x400000, rw);
        page_table.map_pages(KERNEL_BASE, KERNEL_BASE, etext - KERNEL_BASE, rx);
        page_table.map_pages(etext, etext , PHY_STOP - etext, rw);
        page_table.map_pages(TRAMPOLINE, trampoline , PAGE_SIZE, rx);

        page_table.alloc_pages(KERNEL_HEAP_START, KERNEL_HEAP_SIZE, rw);

        Mutex::new(page_table)
    };
}

impl ActivePageTable {
    fn map_pages(&mut self, virtual_memory: usize, physical_memory: usize, size: usize, perm: PageEntryFlags) {
        let mut v_addr = page_round_down(virtual_memory);
        let mut p_addr = physical_memory;
        let v_last = page_round_down(virtual_memory + size - 1) + PAGE_SIZE;

        while v_addr < v_last {
            let result = self.map(
                Page::from_virtual_address(v_addr),
                Frame::from_physical_address(p_addr),
                perm,
            );
            assert!(result);

            v_addr += PAGE_SIZE;
            p_addr += PAGE_SIZE;
        }
    }

    fn alloc_pages(&mut self, virtual_memory: usize, size: usize, perm: PageEntryFlags) {
        let mut v_addr = page_round_down(virtual_memory);
        let v_last = page_round_down(virtual_memory + size - 1) + PAGE_SIZE;

        while v_addr < v_last {
            let frame = PHYSICAL_MEMORY.alloc().unwrap();
            let result = self.map(
                Page::from_virtual_address(v_addr),
                frame,
                perm,
            );
            assert!(result);

            v_addr += PAGE_SIZE;
        }
    }
}

pub fn kernel_page_table_init() {
    let page_table = &*KERNEL_PAGETABLE.lock();

    let etext = etext as usize;
    assert!(page_table.translate(UART0).is_some());
    assert!(page_table.translate(VIRTIO0).is_some());
    assert!(page_table.translate(CLINT).is_some());
    assert!(page_table.translate(PLIC).is_some());
    assert!(page_table.translate(KERNEL_BASE).is_some());
    assert!(page_table.translate(etext).is_some());
    assert!(page_table.translate(TRAMPOLINE).is_some());
}

pub fn hart_init() {
    const SATP_SV39: usize = 8 << 60;
    fn make_satp(page_table: &ActivePageTable) -> usize {
        SATP_SV39 | (page_table.addr() >> 12)
    }
    unsafe {
        write_satp(make_satp(&*KERNEL_PAGETABLE.lock()));
        sfence_vma();
    }
}