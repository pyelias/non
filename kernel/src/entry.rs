use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use core::mem::MaybeUninit;
use core::ptr::{addr_of, slice_from_raw_parts_mut};

use alloc::boxed::Box;
use alloc::vec;

use crate::mm::bump_alloc::BumpAllocator;
use crate::mm::page_table::PTL2;
use crate::mm::{self, frame_alloc};
use crate::multiboot::{self, BootloaderName, Rsdp};
use crate::sync::SpinLock;
use crate::task::{switch_to_task, TCB};
use crate::types::page::PAGE_SIZE;
use crate::types::page_table::{Entry, Flags, LargePageFlags, PTE_INDEX_SIZE};
use crate::types::{zeroable, FrameAddr, HasPhysAddr, HasVirtAddr};

fn print_mmap_entry(entry: multiboot::MMapEntry) {
    // you can't take a reference to a field of a packed struct, so
    // put it in a temp variable and take a reference to that
    macro_rules! copy_packed_field {
        ($x:expr) => {{
            let val = $x;
            val
        }};
    }

    println!("{:?}", entry.kind());
    println!("  addr: {:x}", copy_packed_field!({ entry.addr }.usize()));
    println!("  len: {:x}", copy_packed_field!(entry.len));
}

fn print_all_mmap_entries(entries: multiboot::MMapEntryIterator) {
    for entry in entries {
        print_mmap_entry(entry);
    }
}

#[no_mangle]
extern "sysv64" fn kernel_main(multiboot_info: i32, magic: u32) {
    crate::io::init_com1();

    if magic != 0x36D76289 {
        // check multiboot2 magic number
        println!("bad magic number, dying now");
        return;
    }

    println!("in kernel now");

    let multiboot_info = (multiboot_info as usize).phys_addr().to_virt();

    crate::int::init();
    crate::apic::init();

    println!("multiboot info at {}", multiboot_info);

    let multiboot_info = unsafe { multiboot::Info::new(multiboot_info) };

    for (tag_kind, _) in multiboot_info.tags_iter() {
        println!("tag kind: {}", tag_kind);
    }

    println!(
        "bootloader: {}",
        multiboot_info
            .get_tag::<BootloaderName>()
            .unwrap()
            .as_str()
            .unwrap()
    );

    print_all_mmap_entries(multiboot_info.get_tag().unwrap());

    let acpi_tables = multiboot_info.get_tag::<Rsdp>().unwrap().get_tables();
    // this has to stay as a PhysicalMapping for .entries to work properly
    // b/c this library is scuffed
    let madt = acpi_tables.find_table::<acpi::madt::Madt>().unwrap();
    for entry in madt.entries() {
        match entry {
            acpi::madt::MadtEntry::LocalApic(&e) => {
                println!("local apic: {:?}", e);
            }
            _ => {}
        }
    }

    unsafe {
        frame_alloc::init(multiboot_info);
    };

    let frame = frame_alloc::alloc_frame_with_order(frame_alloc::FrameOrder(9)).unwrap();
    println!("order-9 frame: {}", frame);

    let mut l3_entries = unsafe { mm::page_alloc::get_l3_entries() };
    let mut l2_entries;
    unsafe {
        static mut L2_TABLE: PTL2 = zeroable::zeroed();
        let l2_table_frame: FrameAddr = addr_of!(L2_TABLE).to_phys().as_aligned();
        l2_entries = l3_entries
            .take_first()
            .map_subtable(Entry::at_frame(l2_table_frame), &mut L2_TABLE);
    };
    let large_page_entry = Entry::at_frame(frame).set_flags(LargePageFlags::none());
    let large_page_addr = unsafe { l2_entries.take_first().map_large_page(large_page_entry) };
    println!("order-9 page: {}", large_page_addr);
    let large_page: &'static mut [MaybeUninit<u8>] = unsafe {
        &mut *slice_from_raw_parts_mut(large_page_addr.ptr(), PAGE_SIZE << PTE_INDEX_SIZE)
    };

    /*let seg = unsafe { mm::Segment::make_small_alloc(large_page) };
    let slab = unsafe { seg.alloc_slab(Layout::new::<usize>()) }.unwrap();
    let num = unsafe {
        let ptr = slab.alloc_fast().unwrap() as *mut usize;
        println!("alloced usize: {}", ptr.virt_addr());
        ptr.write(21);
        &mut *ptr
    };
    println!("val: {}", num);

    let slab = unsafe { seg.alloc_slab(Layout::new::<[u64; 128]>()) }.unwrap();
    let stack: &'static mut [u8] =
        unsafe { &mut *(slab.alloc_fast().unwrap() as *mut [u8; 1024] as *mut [u8]) };*/

    let alloc = BumpAllocator::new(large_page);
    *GLOBAL_ALLOC.lock() = Some(alloc);

    let stack: &'static mut [u64] = Box::leak(vec![0; 1024].into_boxed_slice());
    let stack: &'static mut [u8] =
        unsafe { &mut *slice_from_raw_parts_mut(stack as *mut [u64] as *mut u8, 8192) };

    println!("stack: {}", (stack as *const [u8]).virt_addr());

    let mut curr_tcb = Cell::new(TCB::new_empty());
    println!("hwt: {:08x}", hello_world_task as u64);
    let mut next_tcb = Cell::new(TCB::new_with_stack(stack, hello_world_task));
    let mut curr_tcb_reg = &mut curr_tcb;

    println!("switching now");
    unsafe { switch_to_task(&mut curr_tcb_reg, &mut next_tcb) };
    println!("switched back")
}

unsafe impl GlobalAlloc for SpinLock<Option<BumpAllocator<'static>>> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().as_mut().unwrap().alloc_layout(layout) as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // bump allocators can't deallocate
    }
}

#[global_allocator]
static GLOBAL_ALLOC: SpinLock<Option<BumpAllocator<'static>>> = SpinLock::new(None);

fn hello_world_task() {
    println!("hello from a thread");
}
