use core::arch::asm;
use core::iter::Peekable;
use core::ops::Range;
use core::ptr::{addr_of, addr_of_mut};

use crate::mm::bump_alloc::BumpAllocator;
use crate::multiboot::{self, MMapEntryKind};
use crate::sync::SpinLock;
use crate::types::PhysAddr;
use crate::types::{
    self,
    page_table::{Entry, PageTable},
    FrameAddr, HasPhysAddr, HasVirtAddr,
};
use crate::util::align::Alignment;

const MAX_PAGE_ORDER: u8 = 10;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct GroupSize(u8);

impl GroupSize {
    fn bits(self) -> usize {
        match self.0 {
            0 => 1,
            1 => 2,
            _ => 4,
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
        0.phys_addr()
            .as_aligned::<FrameAddr>()
            .next(idx << self.page_shift())
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
        use core::cmp::{max, min};
        let sub_group_max_order = size.sub_size().max_avail();

        let left_max = sub_avails[0] == sub_group_max_order && sub_avails[1] == sub_group_max_order;
        let right_max =
            sub_avails[2] == sub_group_max_order && sub_avails[3] == sub_group_max_order;
        if left_max && right_max {
            return min(sub_group_max_order.four(), Self::max());
        } else if left_max || right_max {
            return min(sub_group_max_order.two(), Self::max());
        }
        let max = max(
            max(sub_avails[0], sub_avails[1]),
            max(sub_avails[2], sub_avails[3]),
        );
        max
    }
}

struct BitmapFrameAllocator {
    group_maps: [&'static mut [u64]; 16],
    max_group_size: GroupSize,
    max_size_group_count: usize,
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
        [0, 1, 2, 3].map(|i| self.get_group_avail(size.sub_size(), 4 * idx + i))
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
        let mut curr_group_pos = (0..self.max_size_group_count)
            .find(|&i| self.get_group_avail(self.max_group_size, i) >= avail)?;
        let mut curr_size: GroupSize = self.max_group_size;
        while curr_size > size {
            curr_size = curr_size.sub_size();
            // TODO: select the smallest avail >= req_avail to reduce fragmentation
            curr_group_pos = (0..4)
                .map(|i| 4 * curr_group_pos + i)
                .find(|&i| self.get_group_avail(curr_size, i) >= avail)?;
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

    fn free_frame_range(&mut self, range: Range<usize>) {
        let (skip_bits_after_start, mid, extra_bits_after_end) =
            Alignment::new(64).split_range(range);
        let start = mid.start / 64;
        let end = mid.end / 64;

        self.group_maps[0][start] |= u64::MAX << skip_bits_after_start;
        for i in start + 1..end {
            self.group_maps[0][i] = u64::MAX;
        }
        self.group_maps[0][end] |= (1 << extra_bits_after_end) - 1;
    }

    fn use_frame_range(&mut self, range: Range<usize>) {
        let (skip_bits_after_start, mid, extra_bits_after_end) =
            Alignment::new(64).split_range(range);
        let start = mid.start / 64;
        let end = mid.end / 64;

        self.group_maps[0][start] &= !(u64::MAX << skip_bits_after_start);
        for i in start + 1..end {
            self.group_maps[0][i] = 0;
        }
        self.group_maps[0][end] &= !((1 << extra_bits_after_end) - 1);
    }

    fn update_all(&mut self) {
        for size in 1..=self.max_group_size.0 {
            for idx in 0..self.group_maps[size as usize].len() {
                self.update_group_avail(GroupSize(size), idx);
            }
        }
    }
}

static FRAME_ALLOC: SpinLock<Option<BitmapFrameAllocator>> = SpinLock::new(None);

#[derive(Clone, Copy)]
struct GroupSizesIter {
    group_count: usize,
    group_size: usize,
}

impl GroupSizesIter {
    fn new(frame_count: usize) -> Self {
        Self {
            group_count: frame_count,
            group_size: 0,
        }
    }
}

impl Iterator for GroupSizesIter {
    type Item = (usize, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.group_count < 4 || self.group_size == 16 {
            return None;
        }

        let bits_per_group = match self.group_size {
            0 => 1,
            1 => 2,
            _ => 4,
        };
        let entry_count = (bits_per_group * self.group_count + 63) / 64;
        let res = (self.group_size, self.group_count, entry_count);

        self.group_size += 1;
        self.group_count = (self.group_count + 3) / 4;

        Some(res)
    }
}

fn get_bitmap_size(frame_count: usize) -> usize {
    let groups = GroupSizesIter::new(frame_count);
    groups.map(|(_, _, entry_count)| entry_count).sum()
}

fn setup_bitmap_frame_allocator(
    map_alloc: &mut BumpAllocator<'static>,
    frame_count: usize,
) -> BitmapFrameAllocator {
    // like [None; 16], but that doesn't work since &mut isn't Copy
    let mut group_maps: [&'static mut [u64]; 16] = [(); 16].map(|_| &mut [][..]);
    let mut max_group_size = 0;
    let mut max_size_group_count = 0;

    let groups = GroupSizesIter::new(frame_count);
    for (group_size, group_count, entry_count) in groups {
        // initialize with 0, which means not free
        group_maps[group_size] = map_alloc.alloc_slice_default(entry_count);

        println!("{} groups of size {}", group_count, group_size);
        println!("using {} entries", entry_count);

        max_group_size = group_size;
        max_size_group_count = group_count;
    }

    BitmapFrameAllocator {
        group_maps,
        max_group_size: GroupSize(max_group_size as u8),
        max_size_group_count,
    }
}

pub fn alloc_frame_with_order(order: FrameOrder) -> Option<FrameAddr> {
    FRAME_ALLOC.lock().as_mut().unwrap().alloc_frame(order)
}

pub fn alloc_frame() -> Option<FrameAddr> {
    alloc_frame_with_order(FrameOrder(0))
}
pub fn free_frame_with_order(frame: FrameAddr, order: FrameOrder) {
    FRAME_ALLOC
        .lock()
        .as_mut()
        .unwrap()
        .free_frame(frame, order);
}

pub fn free_frame(frame: FrameAddr) {
    free_frame_with_order(frame, FrameOrder(0));
}

extern "sysv64" {
    // these are linker variables; their addresses matter, but they have no values
    static HIGH_ID_MAP_VMA: u8;
    static KERNEL_END_VMA: u8;
}

#[derive(Clone)]
struct RangesDifference<A, I: Iterator<Item = Range<A>>, E: Iterator<Item = Range<A>>> {
    plus: Peekable<I>,
    minus: Peekable<E>,
}

impl<A: Clone + PartialOrd, I: Iterator<Item = Range<A>>, E: Iterator<Item = Range<A>>> Iterator
    for RangesDifference<A, I, E>
{
    type Item = Range<A>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(minus) = self.minus.peek().cloned() else {
                return self.plus.next();
            };

            let plus = self.plus.peek()?.clone();

            if minus.end <= plus.start {
                self.minus.next();
                continue;
            }

            if minus.start >= plus.end {
                self.plus.next();
                continue;
            }

            // minus and plus are guaranteed to overlap

            // remove the current minus from the current plus
            if minus.end < plus.end {
                self.plus.peek_mut().unwrap().start = minus.end;
                self.minus.next();
            } else {
                self.plus.next();
            }

            // if part of plus isn't subtracted, return that part
            if plus.start < minus.start {
                return Some(plus.start..minus.start);
            }
        }
    }
}

fn get_usable_memory(
    multiboot_info: multiboot::Info,
) -> impl Iterator<Item = Range<PhysAddr>> + Clone {
    let mmap: multiboot::MMapEntryIterator = multiboot_info.get_tag().unwrap();

    let free_mem = mmap.filter_map(|e| {
        if let MMapEntryKind::Available = e.kind() {
            Some(e.addr..(e.addr.usize() + e.len).phys_addr())
        } else {
            None
        }
    });

    let kernel_end_phys = addr_of!(KERNEL_END_VMA).to_phys();
    let mut used_mem = [
        0usize.phys_addr()..kernel_end_phys, // first 1MB, then kernel binary immediately after
        multiboot_info.mem_range(),          // the multiboot info
    ];
    used_mem.sort_unstable_by_key(|r| r.start);

    RangesDifference {
        plus: free_mem.peekable(),
        minus: used_mem.into_iter().peekable(), // first 1b is used
    }
}

fn get_frame_count(mem: impl Iterator<Item = Range<PhysAddr>>) -> usize {
    mem.map(|r| r.end.align_up::<FrameAddr>())
        .max()
        .unwrap()
        .index()
}

#[inline]
unsafe fn flush_tlb() {
    asm!("mov rax, cr3", "mov cr3, rax", out("rax") _);
}

extern "sysv64" {
    static mut starting_page_tables: [PageTable; 3];
}

pub unsafe fn init(multiboot_info: multiboot::Info) {
    // clear low-address identity mapping set up during boot
    // it's probably fine to just leave it but i dont want to
    let ptl4: &mut PageTable = unsafe { &mut *addr_of_mut!(starting_page_tables[0]) };
    ptl4[0].write(Entry::empty());
    let ptl3: &mut PageTable = unsafe { &mut *addr_of_mut!(starting_page_tables[1]) };
    ptl3[0].write(Entry::empty());

    unsafe { flush_tlb() };

    let usable_memory = get_usable_memory(multiboot_info);
    for range in usable_memory.clone() {
        println!("open: {:?}", range);
    }

    let frame_count = get_frame_count(usable_memory.clone());
    let bitmap_size = get_bitmap_size(frame_count);
    let mut bitmap_addr = None;
    for range in usable_memory.clone() {
        let start = Alignment::of::<u64>().align_up(range.start.usize());
        if range.end.usize() - start > bitmap_size {
            bitmap_addr = Some(start);
            break;
        }
    }
    let bitmap_addr = bitmap_addr.unwrap();

    let bitmap_size_bytes = bitmap_size * core::mem::size_of::<u64>();
    let mut bitmap_bump_alloc =
        BumpAllocator::new_raw(bitmap_addr.phys_addr().to_virt().ptr(), bitmap_size_bytes);
    let mut frame_alloc = setup_bitmap_frame_allocator(&mut bitmap_bump_alloc, frame_count);

    for range in usable_memory.clone() {
        frame_alloc.free_frame_range(
            range.start.align_up::<FrameAddr>().index()..range.end.align::<FrameAddr>().index(),
        );
    }
    frame_alloc.use_frame_range(
        bitmap_addr.phys_addr().align_up::<FrameAddr>().index()
            ..(bitmap_addr + bitmap_size_bytes)
                .phys_addr()
                .align::<FrameAddr>()
                .index(),
    );
    frame_alloc.update_all();

    *FRAME_ALLOC.lock() = Some(frame_alloc);
}
