use core::mem::MaybeUninit;
use crate::types::page::PAGE_SIZE;
use crate::types::page_table::PTE_INDEX_SIZE;
use crate::types::{Page, FrameAddr, PageAddr, HasPhysAddr, HasVirtAddr, page_table::{PageTable, Entry, pte_indices_to_addr}};
use crate::sync::SpinLock;
use super::alloc_frame;
use super::page_table::{PTL1Handle, PTL2Handle, PTL3Handle, PTL1EntrySlot, PTL2EntrySlot, PTL3EntrySlot, PTL1, PTL2};

struct PageAllocator {
}

impl PageAllocator {
}

static PAGE_ALLOC: SpinLock<Option<PageAllocator>> = SpinLock::new(None);

pub fn alloc_page() -> Option<PageAddr> {
    unimplemented!()
    // PAGE_ALLOC.lock().as_mut().unwrap().alloc_page()
}

pub fn alloc_l1_entry() -> Option<(&'static mut Entry, PageAddr)> {
    unimplemented!()
    // PAGE_ALLOC.lock().as_mut().unwrap().alloc_l1_entry()
}

pub fn alloc_l2_entry() -> Option<(&'static mut Entry, PageAddr)> {
    unimplemented!()
    // PAGE_ALLOC.lock().as_mut().unwrap().alloc_l2_entry()
}

// PAGE_ALLOC needs a couple page tables to start with
// only it is allowed to touch these
static mut START_TABLES: [[Page; 2]; 3] = [[Page::zeroed(); 2]; 3];

pub unsafe fn init() {
    let virt = pte_indices_to_addr(511, 0, 0, 0); // 511th entry of the l4 page table, then 0th entry of the l3-l1 tables
    println!("mm virt s {}", virt);
    /*let l1_cursor;
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
        *l3_cursor.next(3).unwrap().0 = Entry::at_frame(l2_table_frame);
        *l2_cursor.next(2).unwrap().0 = Entry::at_frame(l1_table_frame);
    };
    let cursors = [l1_cursor, l2_cursor, l3_cursor];
    *PAGE_ALLOC.lock() = Some(PageAllocator::new(cursors));*/
}