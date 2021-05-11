use core::intrinsics::size_of;

use crate::memory::{Frame, page_round_down, PAGE_SIZE, PHYSICAL_MEMORY, PhysicalAddress, VirtualAddress};
use crate::memory::layout::MAX_VA;
use crate::memory::page_table::{Level1, Level2, Level3, PageEntry, PageEntryFlags, PageTable};

pub struct Page {
    number: usize,
}

impl Page {
    pub fn from_virtual_address(virtual_address: VirtualAddress) -> Page {
        assert!(virtual_address < MAX_VA);
        Page { number: virtual_address >> 12 }
    }

    pub fn addr(&self) -> usize {
        self.number << 12
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
    pub fn new() -> Option<ActivePageTable> {
        PHYSICAL_MEMORY.alloc().map(|frame| {
            let mut page_table = ActivePageTable {
                p3: frame.addr() as *mut PageTable<Level3>
            };
            page_table.p3_mut().zero();
            page_table
        })
    }

    fn p3(&self) -> &PageTable<Level3> {
        unsafe { &*self.p3 }
    }

    fn p3_mut(&mut self) -> &mut PageTable<Level3> {
        unsafe { &mut *self.p3 }
    }

    pub fn addr(&self) -> usize {
        self.p3 as usize
    }

    pub fn translate(&self, virtual_address: VirtualAddress) -> Option<PhysicalAddress> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(&Page::from_virtual_address(virtual_address)).map(|it| it + offset)
    }

    fn translate_page(&self, page: &Page) -> Option<PhysicalAddress> {
        let p2 = self.p3().next_table(page.l3_index());
        let p1 = p2.and_then(|it| it.next_table(page.l2_index()));
        return p1.and_then(|it| it[page.l1_index()].pointed_frame().map(|it| it.addr()));
    }

    pub fn map(&mut self, page: Page, frame: Frame, flags: PageEntryFlags) -> bool {
        self.p3_mut().next_table_or_create(page.l3_index())
            .and_then(|p2| p2.next_table_or_create(page.l2_index()))
            .map_or(false, |p1| {
                assert!(p1[page.l1_index()].is_unused());
                p1[page.l1_index()].set(frame, flags | PageEntryFlags::VALID);
                true
            })
    }

    pub fn unmap(&mut self, page: Page) {
        assert!(self.translate(page.addr()).is_some());

        let p1 = self.p3_mut().next_table_mut(page.l3_index())
            .and_then(|p2| p2.next_table_mut(page.l2_index()))
            .expect("unmap");

        let frame = p1[page.l1_index()].pointed_frame().unwrap();
        p1[page.l1_index()].set_unused();
        PHYSICAL_MEMORY.dealloc(frame);
    }

    pub fn map_pages(&mut self, virtual_memory: usize, physical_memory: usize, size: usize, perm: PageEntryFlags) -> bool {
        let mut v_addr = page_round_down(virtual_memory);
        let mut p_addr = physical_memory;
        let v_last = page_round_down(virtual_memory + size - 1) + PAGE_SIZE;

        while v_addr < v_last {
            let result = self.map(
                Page::from_virtual_address(v_addr),
                Frame::from_physical_address(p_addr),
                perm,
            );
            if !result {
                return false;
            }

            v_addr += PAGE_SIZE;
            p_addr += PAGE_SIZE;
        }

        true
    }

    pub fn unmap_pages(&mut self, virtual_memory: usize, size: usize) {
        assert_eq!(virtual_memory % PAGE_SIZE, 0);
        assert_eq!(size % PAGE_SIZE, 0);

        for v_addr in (virtual_memory..(virtual_memory + size)).step_by(PAGE_SIZE) {
            self.unmap_no_free(Page::from_virtual_address(v_addr));
        }
    }

    fn unmap_no_free(&mut self, page: Page) {
        assert!(self.translate(page.addr()).is_some());

        let p1 = self.p3_mut().next_table_mut(page.l3_index())
            .and_then(|p2| p2.next_table_mut(page.l2_index()))
            .expect("unmap");

        p1[page.l1_index()].set_unused();
    }
}

pub fn virtual_memory_init() {
    assert_eq!(size_of::<PageEntry>(), 8);
    assert_eq!(size_of::<PageTable<Level1>>(), PAGE_SIZE);
    assert_eq!(size_of::<PageTable<Level2>>(), PAGE_SIZE);
    assert_eq!(size_of::<PageTable<Level3>>(), PAGE_SIZE);
}