use core::intrinsics::size_of;

use crate::memory::{PAGE_SIZE, PageEntry, PageTable, PhysicalAddress, VirtualAddress};
use crate::memory::layout::MAX_VA;
use crate::memory::page_table::{Level1, Level2, Level3};

pub struct Page {
    number: usize,
}

impl Page {
    pub fn from_virtual_address(virtual_address: VirtualAddress) -> Page {
        assert_eq!(virtual_address < MAX_VA);
        Page { number: virtual_address >> 12 }
    }

    pub fn l3_index(&self) -> usize {
        (self.number >> (2 * 9)) & 0x1ff
    }

    pub fn l2_index(&self) -> usize {
        (self.number >> (1 * 9)) & 0x1ff
    }

    pub fn l1_index(&self) -> usize {
        (self.number >> (0 * 9)) & 0x1ff
    }
}

pub struct ActivePageTable {
    p3: *mut PageTable<Level3>,
}

impl ActivePageTable {
    fn p3(&self) -> &PageTable<Level3> {
        unsafe { &*self.p3 }
    }

    fn p3_mut(&mut self) -> &mut PageTable<Level3> {
        unsafe { &mut *self.p3 }
    }

    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        self.translate_page(&Page::from_virtual_address(virtual_address))
    }

    fn translate_page(&self, page: &Page) -> Option<PhysicalAddress> {
        let p2 = self.p3().next_table(page.l3_index());
        let p1 = p2.and_then(|it| it.next_table(page.l2_index()));
        return p1.and_then(|it| it[page.l1_index()].pointed_addr());
    }
}

pub fn virtual_memory_init() {
    unsafe {
        assert_eq!(size_of::<PageEntry>(), 8);
        assert_eq!(size_of::<PageTable<Level1>>(), PAGE_SIZE);
        assert_eq!(size_of::<PageTable<Level2>>(), PAGE_SIZE);
        assert_eq!(size_of::<PageTable<Level3>>(), PAGE_SIZE);
    }
}