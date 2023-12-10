use crate::types::{self, HasPhysAddr, HasVirtAddr, FrameAddr, page_table::{PageTable, Entry}};
use crate::multiboot::{self, MMapEntryType};
use crate::sync::SpinLock;
use crate::mm::bump_alloc::BumpAllocator;

const MAX_PAGE_ORDER: u8 = 10;

extern "sysv64" {
    static HIGH_ID_MAP_VMA: u8;
    static KERNEL_END_VMA: u8;
}

#[no_mangle]
extern "sysv64" fn get_usable_memory(info: &multiboot::Info, low_frame: &mut FrameAddr, high_frame: &mut FrameAddr) {
    // safe because we don't care about the values of those things
    // just their addresses
    let kernel_phys_end = unsafe { core::ptr::addr_of!(KERNEL_END_VMA).usize() - core::ptr::addr_of!(HIGH_ID_MAP_VMA).usize() };
    *low_frame = kernel_phys_end.phys_addr().align::<FrameAddr>().next(1);

    let mut high_addr = 0;
    for entry in info.mmap_entries() {
        if let MMapEntryType::Available = entry.type_() {
            // { entry.addr } instead of entry.addr puts it on the stack so i can take a reference to it
            high_addr = core::cmp::max(high_addr, { entry.addr }.usize() + entry.len);
        }
    }
    *high_frame = high_addr.phys_addr().align::<FrameAddr>();
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct GroupSize(u8);

impl GroupSize {
    fn bits(self) -> usize {
        match self.0 {
            0 => 1,
            1 => 2,
            _ => 4
        }
    }

    fn avails_per_entry(self) -> usize {
        64 / self.bits()
    }

    fn idx_offset(self, idx: usize) -> (usize, usize) {
        let entry_idx = idx / self.avails_per_entry();
        let bit_offset = (idx % self.avails_per_entry()) * self.bits();
        (entry_idx, bit_offset)
    }

    fn mask(self) -> u64 {
        (1 << self.bits()) - 1
    }

    fn max_avail(self) -> Avail {
        Avail(self.0 * 2 + 1)
    }

    fn sub_size(self) -> Self {
        Self(self.0 - 1)
    }

    fn super_size(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Copy)]
pub struct FrameOrder(pub u8);

impl FrameOrder {
    fn is_single_group(self) -> bool {
        self.0 % 2 == 0
    }

    fn group_size(self) -> GroupSize {
        GroupSize((self.0 + 1) / 2)
    }

    fn free_avail(self) -> Avail {
        Avail(self.0 + 1)
    }

    fn page_shift(self) -> usize {
        self.0 as usize
    }

    fn frame_at_idx(self, idx: usize) -> FrameAddr {
        0.phys_addr().as_aligned::<FrameAddr>().next(idx << self.page_shift())
    }

    fn idx_of_frame(self, frame: FrameAddr) -> usize {
        frame.usize() >> (types::page::PAGE_SHIFT + self.page_shift())
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Avail(u8);

impl Avail {
    fn empty() -> Self {
        Self(0)
    }
    fn max() -> Self {
        Self(MAX_PAGE_ORDER + 1)
    }

    fn two(&self) -> Self {
        Self(self.0 + 1)
    }

    fn four(&self) -> Self {
        Self(self.0 + 2)
    }

    fn merge(size: GroupSize, sub_avails: [Avail; 4]) -> Self {
        use core::cmp::{min, max};
        let sub_group_max_order = size.sub_size().max_avail();

        let left_max = 
            sub_avails[0] == sub_group_max_order &&
            sub_avails[1] == sub_group_max_order;
        let right_max = 
            sub_avails[2] == sub_group_max_order &&
            sub_avails[3] == sub_group_max_order;
        if left_max && right_max {
            return min(sub_group_max_order.four(), Self::max());
        } else if left_max || right_max {
            return min(sub_group_max_order.two(), Self::max());
        }
        let max = max(max(sub_avails[0], sub_avails[1]), max(sub_avails[2], sub_avails[3]));
        max
    }
}

struct BitmapFrameAllocator {
    group_maps: [&'static mut [u64]; 16],
    max_group_size: GroupSize,
    max_size_group_count: usize
}

impl BitmapFrameAllocator {
    fn get_group_avail(&self, size: GroupSize, idx: usize) -> Avail {
        let (entry_idx, bit_offset) = size.idx_offset(idx);
        let res = ((self.group_maps[size.0 as usize][entry_idx] >> bit_offset) & size.mask()) as u8;
        Avail(res)
    }

    fn set_group_avail(&mut self, size: GroupSize, idx: usize, avail: Avail) {
        let (entry_idx, bit_offset) = size.idx_offset(idx);
        self.group_maps[size.0 as usize][entry_idx] &= !(size.mask() << bit_offset);
        self.group_maps[size.0 as usize][entry_idx] |= (avail.0 as u64) << bit_offset;
    }

    fn get_sub_group_avails(&self, size: GroupSize, idx: usize) -> [Avail; 4] {
        [0, 1, 2, 3].map(|i| { self.get_group_avail(size.sub_size(), 4 * idx + i) }) 
    }

    fn update_group_avail(&mut self, size: GroupSize, idx: usize) -> bool {
        let sub_avails = self.get_sub_group_avails(size, idx);
        let avail = Avail::merge(size, sub_avails);
        if avail != self.get_group_avail(size, idx) {
            self.set_group_avail(size, idx, avail);
            true
        } else {
            false
        }
    }

    fn update_all_parents(&mut self, mut size: GroupSize, mut idx: usize) {
        while size < self.max_group_size {
            size = size.super_size();
            idx /= 4;
            if !self.update_group_avail(size, idx) {
                // if nothing changes, we're done
                return;
            }
        }
    }

    fn free_frame(&mut self, frame: FrameAddr, order: FrameOrder) {
        if order.is_single_group() {
            let size = order.group_size();
        
            // update the avail of the page's group
            let idx = order.idx_of_frame(frame);
            self.set_group_avail(size, idx, order.free_avail());
        
            // update the avail of the groups containing it
            self.update_all_parents(size, idx);
        } else {
            let size = order.group_size().sub_size();

            let frame_idx = order.idx_of_frame(frame);
            let group_idx = frame_idx * 2;
            self.set_group_avail(size, group_idx, order.free_avail());
            self.set_group_avail(size, group_idx + 1, order.free_avail());
            self.update_all_parents(size, group_idx);
            self.update_all_parents(size, group_idx + 1);

        }
    }

    fn find_group_of_size_with_avail(&self, size: GroupSize, avail: Avail) -> Option<usize> {
        let mut curr_group_pos = (0..self.max_size_group_count).find(|&i| self.get_group_avail(self.max_group_size, i) >= avail)?;
        let mut curr_size: GroupSize = self.max_group_size;
        while curr_size > size {
            curr_size = curr_size.sub_size();
            // TODO: select the smallest avail >= req_avail to reduce fragmentation
            curr_group_pos = (0..4).map(|i| 4 * curr_group_pos + i).find(|&i| self.get_group_avail(curr_size, i) >= avail)?;
        }
        Some(curr_group_pos)
    }

    fn alloc_frame(&mut self, order: FrameOrder) -> Option<FrameAddr> {
        let size = order.group_size();
        let idx = self.find_group_of_size_with_avail(size, order.free_avail())?;
    
        if order.is_single_group() {
            self.set_group_avail(size, idx, Avail::empty());
            self.update_all_parents(size, idx);
        
            Some(order.frame_at_idx(idx))
        } else {
            let req_sub_avail = size.sub_size().max_avail();
            let sub_avails = self.get_sub_group_avails(size, idx);
            let sub_idx = if sub_avails[0] == req_sub_avail && sub_avails[1] == req_sub_avail {
                0
            } else {
                debug_assert!(sub_avails[2] == req_sub_avail && sub_avails[3] == req_sub_avail);
                1
            };
            let frame_idx = idx * 2 + sub_idx;
            self.set_group_avail(size, frame_idx, Avail::empty());
            self.set_group_avail(size, frame_idx + 1, Avail::empty());
            self.update_all_parents(size, frame_idx);
            self.update_all_parents(size, frame_idx + 1);
            Some(order.frame_at_idx(frame_idx))
        }
    }
}

static FRAME_ALLOC: SpinLock<Option<BitmapFrameAllocator>> = SpinLock::new(None);

fn setup_bitmap_frame_allocator(map_alloc: &mut BumpAllocator<'static>, frame_count: usize) {
    let mut group_size = 0;
    let mut group_count = frame_count;

    // like [None; 16], but that doesn't work since &mut isn't Copy
    let mut group_maps: [&'static mut [u64]; 16] = [(); 16].map(|_| &mut [][..]);
    let mut max_group_size = 0;
    let mut max_size_group_count = 0;
    while group_count >= 4 && group_size < 16 {
        let bits_per_group = match group_size {
            0 => 1,
            1 => 2,
            _ => 4
        };

        let entry_count = (bits_per_group * group_count + 63) / 64;

        group_maps[group_size] = map_alloc.alloc_slice_default(entry_count);

        println!("{} groups of size {}", group_count, group_size);
        println!("using {} entries", entry_count);

        max_group_size = group_size;
        max_size_group_count = group_count;
        group_size += 1;
        group_count = (group_count + 3) / 4;
    }

    let frame_alloc_ = BitmapFrameAllocator {
        group_maps,
        max_group_size: GroupSize(max_group_size as u8),
        max_size_group_count
    };
    *FRAME_ALLOC.lock() = Some(frame_alloc_);
}

pub fn alloc_frame_with_order(order: FrameOrder) -> Option<FrameAddr> {
    FRAME_ALLOC.lock().as_mut().unwrap().alloc_frame(order)
}

pub fn alloc_frame() -> Option<FrameAddr> {
    alloc_frame_with_order(FrameOrder(0))
}
pub fn free_frame_with_order(frame: FrameAddr, order: FrameOrder) {
    FRAME_ALLOC.lock().as_mut().unwrap().free_frame(frame, order);
}

pub fn free_frame(frame: FrameAddr) {
    free_frame_with_order(frame, FrameOrder(0));
}


pub unsafe fn init(multiboot_info: &multiboot::Info) {
    // clear low-address identity mapping set up during boot
    // it's probably fine to just leave it but i dont want to
    let ptl4: &mut PageTable = unsafe { &mut *0x1000usize.to_virt().ptr() };
    ptl4[0].write(Entry::empty());
    let ptl3: &mut PageTable = unsafe { &mut *0x2000usize.to_virt().ptr() };
    ptl3[0].write(Entry::empty());
    
    extern "sysv64" {
        fn flush_tlb();
    }
    unsafe { flush_tlb() };

    let mut low_frame = 0usize.phys_addr().as_aligned::<FrameAddr>();
    let mut high_frame = low_frame;
    get_usable_memory(multiboot_info, &mut low_frame, &mut high_frame);
    println!("low_frame:  {}", low_frame);
    println!("high_frame: {}", high_frame);

    let frame_count = high_frame.usize() >> types::page::PAGE_SHIFT;
    
    // for allocator debugging, make the memory really small i guess
    // let high_frame = (low_frame.usize() + 0x100000).frame_addr);

    let bitmap_start_ptr = low_frame.to_virt().ptr();
    let mem_length = high_frame.usize() - low_frame.usize();
    let mut map_alloc = unsafe { BumpAllocator::new(bitmap_start_ptr, mem_length) };
    setup_bitmap_frame_allocator(&mut map_alloc, frame_count);
    let bitmap_end_ptr = map_alloc.done_ptr();
    let low_frame = bitmap_end_ptr.to_phys().align::<FrameAddr>().next(1);

    println!("low_frame:  {}", low_frame);
    println!("high_frame: {}", high_frame);
    let frame_count = (high_frame.usize() - low_frame.usize()) >> types::page::PAGE_SHIFT;
    println!("total free frames: {}", frame_count);
    let mut curr_frame = low_frame;
    for _ in 0..frame_count {
        free_frame(curr_frame);
        curr_frame = curr_frame.next(1);
    }
}