use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use crate::memory::{Frame, PHYSICAL_MEMORY};

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
        PageEntryFlags::from_bits_truncate(self.0)
    }

    pub fn pointed_frame(&self) -> Option<Frame> {
        if self.flags().contains(PageEntryFlags::VALID) {
            Some(Frame::from_physical_address((self.0 as usize >> 10) << 12))
        } else {
            None
        }
    }

    pub fn set(&mut self, frame: Frame, flags: PageEntryFlags) {
        self.0 = ((frame.addr() >> 12) << 10) as u64 | flags.bits();
    }

    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
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

impl<L: PageTableLevel> PageTable<L> {
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.set_unused();
        }
    }
}

impl<L: HierarchicalLevel> PageTable<L> {
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
            self[index].pointed_frame().map(|it| it.addr())
        } else {
            None
        }
    }

    pub fn next_table_or_create(&mut self, index: usize) -> Option<&mut PageTable<L::NextLevel>> {
        if self.next_table(index).is_none() {
            let frame = PHYSICAL_MEMORY.alloc();
            if frame.is_some() {
                self.entries[index].set(frame.unwrap(), PageEntryFlags::VALID);
                self.next_table_mut(index).unwrap().zero();
            }
        }
        self.next_table_mut(index)
    }
}