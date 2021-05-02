use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use crate::memory::PAGE_SIZE;

const ENTRY_COUNT: usize = 512;

bitflags! {
    pub struct PageEntryFlags: u64 {
        const VALID      = 1 << 0;
        const READABLE   = 1 << 1;
        const WRITEABLE  = 1 << 2;
        const EXECUTABLE = 1 << 3;
        const USER       = 1 << 4;
        const GLOBAL     = 1 << 5;
        const ACCESSED   = 1 << 6;
        const DIRTY      = 1 << 7;
    }
}

pub struct PageEntry(u64);

impl PageEntry {
    pub fn flags(&self) -> PageEntryFlags {
        PageEntryFlags::from_bits(self.0).unwrap()
    }

    pub fn pointed_addr(&self) -> Option<usize> {
        if self.flags().contains(PageEntryFlags::VALID) {
            Some((self.0 as usize >> 12) << 10)
        } else {
            None
        }
    }

    pub fn set(&mut self, addr: usize, flags: PageEntryFlags) {
        assert_eq!(addr % PAGE_SIZE, 0);
        self.0 = ((addr >> 12) << 10) as u64 | flags.bits();
    }
}


pub trait PageTableLevel {}

pub enum Level3 {}

pub enum Level2 {}

pub enum Level1 {}

impl PageTableLevel for Level3 {}

impl PageTableLevel for Level2 {}

impl PageTableLevel for Level1 {}

pub trait HierarchicalLevel: PageTableLevel {
    type NextLevel: PageTableLevel;
}

impl HierarchicalLevel for Level3 {
    type NextLevel = Level2;
}

impl HierarchicalLevel for Level2 {
    type NextLevel = Level1;
}

pub struct PageTable<L: PageTableLevel> {
    entries: [PageEntry; ENTRY_COUNT],
    level: PhantomData<L>,
}

impl<L: PageTableLevel> Index<usize> for PageTable<L> {
    type Output = PageEntry;

    fn index(&self, index: usize) -> &PageEntry {
        &self.entries[index]
    }
}

impl<L: PageTableLevel> IndexMut<usize> for PageTable<L> {
    fn index_mut(&mut self, index: usize) -> &mut PageEntry {
        &mut self.entries[index]
    }
}

impl<L> PageTable<L> where L: HierarchicalLevel
{
    pub fn next_table(&self, index: usize) -> Option<&PageTable<L::NextLevel>> {
        self.next_table_address(index)
            .map(|addr| unsafe { &*(addr as *const _) })
    }

    pub fn next_table_mut(&mut self, index: usize) -> Option<&mut PageTable<L::NextLevel>> {
        self.next_table_address(index)
            .map(|addr| unsafe { &mut *(addr as *mut _) })
    }

    fn next_table_address(&self, index: usize) -> Option<usize> {
        let flags = self[index].flags();
        if flags.contains(PageEntryFlags::VALID) {
            self[index].pointed_addr()
        } else {
            None
        }
    }
}