use super::alloc_frame;
use super::page_table::{PTL1Entries, PTL1EntrySlot, PTL2Entries, PTL3Entries, PTL1, PTL2};
use crate::sync::SpinLock;
use crate::types::{
    page_table::{pte_indices_to_addr, Entry},
    zeroable, FrameAddr, HasPhysAddr, HasVirtAddr, PageAddr,
};

struct PageAllocator {
    l3: PTL3Entries<'static>,
    curr_l2: PTL2Entries<'static>,
    curr_l1: PTL1Entries<'static>,
    allocation_start: PageAddr,
}

impl PageAllocator {
    unsafe fn new(
        mut l3: PTL3Entries<'static>,
        init_l2: &mut PTL2,
        init_l2_phys: FrameAddr,
        init_l1: &mut PTL1,
        init_l1_phys: FrameAddr,
    ) -> Self {
        let mut init_l2 = unsafe {
            l3.take_first()
                .map_subtable(Entry::at_frame(init_l2_phys), init_l2.as_aligned())
        };
        let init_l1 = unsafe {
            init_l2
                .take_first()
                .map_subtable(Entry::at_frame(init_l1_phys), init_l1.as_aligned())
        };
        Self {
            l3: l3,
            allocation_start: init_l1.addr().as_aligned(),
            curr_l2: init_l2,
            curr_l1: init_l1,
        }
    }

    fn map_page(&mut self) -> (PageAddr, FrameAddr) {
        let frame = alloc_frame().unwrap();
        let l1_entry = self.curr_l1.take_first();
        let entry_value = Entry::at_frame(frame);
        (unsafe { l1_entry.map_page(entry_value) }, frame)
    }

    fn alloc_l1_entry(&mut self) -> PTL1EntrySlot<'static> {
        self.maybe_replenish();
        self.curr_l1.take_first()
    }

    fn alloc_page(&mut self) -> PageAddr {
        self.maybe_replenish();
        self.map_page().0
    }

    fn maybe_replenish(&mut self) {
        if self.curr_l2.len() == 0 && self.curr_l1.len() == 2 {
            self.map_new_l2();
        }
        if self.curr_l1.len() == 1 {
            self.map_new_l1();
        }
    }

    fn map_new_l2(&mut self) {
        let l3_entry = self.l3.take_first();
        let (ptl2_virt, ptl2_phys) = self.map_page();
        let new_l2 = unsafe { l3_entry.map_subtable(Entry::at_frame(ptl2_phys), ptl2_virt) };
        self.curr_l2 = new_l2;
    }

    fn map_new_l1(&mut self) {
        let l2_entry = self.curr_l2.take_first();
        let (ptl1_virt, ptl1_phys) = self.map_page();
        let new_l1 = unsafe { l2_entry.map_subtable(Entry::at_frame(ptl1_phys), ptl1_virt) };
        self.curr_l1 = new_l1;
    }
}

static PAGE_ALLOC: SpinLock<Option<PageAllocator>> = SpinLock::new(None);

pub fn alloc_l1_entry() -> Option<PTL1EntrySlot<'static>> {
    Some(PAGE_ALLOC.lock().as_mut().unwrap().alloc_l1_entry())
}

pub fn alloc_page() -> Option<PageAddr> {
    Some(PAGE_ALLOC.lock().as_mut().unwrap().alloc_page())
}

// PAGE_ALLOC needs a couple page tables to start with
// only it is allowed to touch these
static mut INIT_PTL2: PTL2 = zeroable::zeroed();
static mut INIT_PTL1: PTL1 = zeroable::zeroed();

pub unsafe fn init() {
    let allocator_start_addr = pte_indices_to_addr(511, 0, 0, 0); // 511th entry of the l4 page table, then 0th entry of the l3-l1 tables
    println!("mm virt s {}", allocator_start_addr);

    let l3_entries: &'static mut [usize; 511] = unsafe { &mut *(0x2000usize.to_virt()).ptr() };
    let l3_entries = PTL3Entries::from_entries_addr(l3_entries, allocator_start_addr.as_aligned());

    let l2_table_frame_addr = (&INIT_PTL2).to_phys().as_aligned();
    let l1_table_frame_addr = (&INIT_PTL1).to_phys().as_aligned();

    let page_alloc = PageAllocator::new(
        l3_entries,
        &mut INIT_PTL2,
        l2_table_frame_addr,
        &mut INIT_PTL1,
        l1_table_frame_addr,
    );

    *PAGE_ALLOC.lock() = Some(page_alloc);
}
