use core::mem::MaybeUninit;
use crate::types::page::PAGE_SIZE;
use crate::types::page_table::PTE_INDEX_SIZE;
use crate::types::{self, page, page_table, Page, FrameAddr, PageAddr, HasPhysAddr, HasVirtAddr, PageTable, Flags, Entry, PageEntryFlags, PageEntry, GenericEntry, PageTableEntryFlags, PageTableEntry, TransmuteFrom, page_table::pte_indices_to_addr};
use crate::sync::SpinLock;
use super::alloc_frame;

type PageListLink = Option<PageAddr>;

// Safety: PageListLink is smaller than a page, and has alignment less than 4096
// (i don't think anything actually guarantees this, but it'd be really weird if it wasn't)
// anything is a valid value for MaybeUninit
unsafe impl TransmuteFrom<Page> for MaybeUninit<PageListLink> {}

pub struct PageTableCursor {
    curr: &'static mut [MaybeUninit<GenericEntry>],
    curr_virt: PageAddr,
}

impl PageTableCursor {
    fn new(first_table: &'static mut Page, virt: PageAddr) -> Self {
        Self {
            curr: PageTable::from_mut(first_table),
            curr_virt: virt
        }
    }

    fn remaining(&self) -> usize {
        self.curr.len()
    }

    pub fn next(&mut self, level: usize) -> Option<(&'static mut GenericEntry, PageAddr)> {
        println!("pulled from l{} cursor {:?}", level, self.curr as *mut _);
        let entry = self.curr.take_first_mut()?;
        let advance = PAGE_SIZE << (PTE_INDEX_SIZE * (level - 1));
        let virt = self.curr_virt;
        self.curr_virt = (self.curr_virt.usize() + advance).page_addr();
        Some((entry.write(GenericEntry::empty()), virt))
    }
}

struct PageAllocator {
    l3_cursor: PageTableCursor, // depending on how exactly is structure virtual address space, this cursor might need to be a different type later
    l2_cursor: PageTableCursor,
    next_l2_cursor: Option<PageTableCursor>,
    l1_cursor: PageTableCursor,
    next_l1_cursor: Option<PageTableCursor>,
}

impl PageAllocator {
    // Safety: curr_virt must point to the address the cursors will map the first page to
    unsafe fn new(cursors: [PageTableCursor; 3]) -> Self {
        let [l1_cursor, l2_cursor, l3_cursor] = cursors;
        Self {
            l3_cursor,
            l2_cursor,
            next_l2_cursor: None,
            l1_cursor,
            next_l1_cursor: None,
        }
    }

    fn alloc_l3_entry(&mut self) -> Option<(&'static mut GenericEntry, PageAddr)> {
        self.l3_cursor.next(3)
    }

    // returns ref to the table, plus address of pages mapped by the table
    fn alloc_l2_table(&mut self) -> Option<(&'static mut Page, PageAddr)> {
        let (entry, curr_virt) = self.alloc_l3_entry()?;
        let frame = alloc_frame()?;
        *entry = PageTableEntry::at_frame(frame).with_flags(PageTableEntryFlags::none().present()).to_generic();
        let new_table = self.map_next_frame(frame)?;
        // Safety: we just added the PTE for it
        let page_ref = unsafe { &mut *new_table.ptr() };
        Some((page_ref, curr_virt))
    }

    fn make_l2_cursor(&mut self) -> Option<PageTableCursor> {
        let (table, virt) = self.alloc_l2_table()?;
        Some(PageTableCursor::new(table, virt))
    }

    fn alloc_l2_entry(&mut self) -> Option<(&'static mut GenericEntry, PageAddr)> {
        if self.next_l2_cursor.is_none() {
            self.next_l2_cursor = Some(self.make_l2_cursor()?);
        }
        
        let (entry, curr_virt) = self.l2_cursor.next(2)?;

        if self.l2_cursor.remaining() == 0 {
            self.l2_cursor = self.next_l2_cursor.take()?;
        }
        Some((entry, curr_virt))
    }

    // returns ref to the table, plus address of pages mapped by the table
    fn alloc_l1_table(&mut self) -> Option<(&'static mut Page, PageAddr)> {
        let (entry, curr_virt) = self.alloc_l2_entry()?;
        let frame = alloc_frame()?;
        *entry = PageTableEntry::at_frame(frame).with_flags(PageTableEntryFlags::none().present()).to_generic();
        let new_table = self.map_next_frame(frame)?;
        // Safety: we just added the PTE for it
        let page_ref = unsafe { &mut *new_table.ptr() };
        Some((page_ref, curr_virt))
    }

    fn make_l1_cursor(&mut self) -> Option<PageTableCursor> {
        let (table, virt) = self.alloc_l1_table()?;
        Some(PageTableCursor::new(table, virt))
    }


    fn alloc_l1_entry(&mut self) -> Option<(&'static mut GenericEntry, PageAddr)> {
        if self.next_l1_cursor.is_none() {
            self.next_l1_cursor = Some(self.make_l1_cursor()?);
        }
        
        let (entry, curr_virt) = self.l1_cursor.next(1)?;

        if self.l1_cursor.remaining() == 0 {
            self.l1_cursor = self.next_l1_cursor.take()?;
        }
        Some((entry, curr_virt))
    }

    fn map_frame(entry: &'static mut GenericEntry, frame: FrameAddr) {
        *entry = PageEntry::at_frame(frame).with_flags(PageEntryFlags::none().present()).to_generic();
    }

    fn map_next_frame(&mut self, frame: FrameAddr) -> Option<PageAddr> {
        let (entry, virt) = self.l1_cursor.next(1)?;
        Self::map_frame(entry, frame);
        Some(virt)
    }

    fn alloc_page_get_phys(&mut self) -> Option<(PageAddr, FrameAddr)> {
        let (entry, curr_virt) = self.alloc_l1_entry()?;
        let frame = alloc_frame()?;
        Self::map_frame(entry, frame);

        Some((curr_virt, frame))
    }

    fn alloc_page(&mut self) -> Option<PageAddr> {
        self.alloc_page_get_phys().map(|t| t.0)
    }
}

static PAGE_ALLOC: SpinLock<Option<PageAllocator>> = SpinLock::new(None);

pub fn alloc_page() -> Option<PageAddr> {
    PAGE_ALLOC.lock().as_mut().unwrap().alloc_page()
}

pub fn alloc_l1_entry() -> Option<(&'static mut GenericEntry, PageAddr)> {
    PAGE_ALLOC.lock().as_mut().unwrap().alloc_l1_entry()
}

pub fn alloc_l2_entry() -> Option<(&'static mut GenericEntry, PageAddr)> {
    PAGE_ALLOC.lock().as_mut().unwrap().alloc_l2_entry()
}

// PAGE_ALLOC needs a couple page tables to start with
// only it is allowed to touch these
static mut START_TABLES: [Page; 2] = [Page::zeroed(); 2];

pub unsafe fn init() {
    let virt = pte_indices_to_addr(511, 0, 0, 0); // 511th entry of the l4 page table, then 0th entry of the l3-l1 tables
    println!("mm virt s {}", virt);
    let l1_cursor;
    let mut l2_cursor;
    let mut l3_cursor;
    unsafe {
        l1_cursor = PageTableCursor::new(&mut START_TABLES[0], virt);
        l2_cursor = PageTableCursor::new(&mut START_TABLES[1], virt);
        let l3_cursor_table = &mut *(0x2000usize.to_virt().ptr());
        let l3_cursor_table = PageTable::from_mut(l3_cursor_table);
        l3_cursor = PageTableCursor {
            curr: &mut l3_cursor_table[..511], // last entry is the id-map
            curr_virt: virt
        };
        let l1_table_frame = (&mut START_TABLES[0]).to_phys().frame_addr();
        let l2_table_frame = (&mut START_TABLES[1]).to_phys().frame_addr();
        *l3_cursor.next(3).unwrap().0 = PageTableEntry::at_frame(l2_table_frame).with_flags(PageTableEntryFlags::none().present()).to_generic();
        *l2_cursor.next(2).unwrap().0 = PageTableEntry::at_frame(l1_table_frame).with_flags(PageTableEntryFlags::none().present()).to_generic();
    };
    let cursors = [l1_cursor, l2_cursor, l3_cursor];
    *PAGE_ALLOC.lock() = Some(PageAllocator::new(cursors));
}