use core::alloc::Layout;
use core::cell::Cell;
use core::ffi::CStr;
use core::mem::MaybeUninit;

use crate::multiboot;
use crate::types::{HasPhysAddr, HasVirtAddr, page_table::Entry};
use crate::mm::{self, frame_alloc, page_alloc, alloc_frame_with_order, FrameOrder, alloc_page};
use crate::task::{TCB, switch_to_task};

fn print_mmap_element(entry: &multiboot::MMapEntry) {
    // you can't take a reference to a field of a packed struct, so
    // put it in a temp variable and take a reference to that
    macro_rules! copy_packed_field {
        ($x:expr) => {{
                let val = $x;
                val
        }}
    }

    let entry = *entry;
    println!("{:?}", entry.type_());
    println!("  addr: {:x}", copy_packed_field!({ entry.addr }.usize()));
    println!("  len: {:x}", copy_packed_field!(entry.len));
}

#[no_mangle]
extern "sysv64" fn print_all_mmap_elements(info: &multiboot::Info) {
    for entry in info.mmap_entries() {
        print_mmap_element(entry);
    }
}

#[no_mangle]
extern "sysv64" fn kernel_main_rust(info: &multiboot::Info) {
    println!("in rust now");
    println!("{:032b}", info.flags);
    println!("{:08x}", info.mmap_addr);
    let boot_loader_name = (info.boot_loader_name as usize).to_virt().ptr();
    let boot_loader_name = unsafe { CStr::from_ptr(boot_loader_name) };
    let boot_loader_name = boot_loader_name.to_str().unwrap();
    println!("{}", boot_loader_name);

    for entry in info.mmap_entries() {
        print_mmap_element(entry);
    }

    unsafe {
        frame_alloc::init(info);
        page_alloc::init();
    };

    println!("mm set up");

    let page = alloc_page().unwrap();
    println!("page: {}", page);
    unsafe {
        *page.ptr::<u8>() = 21; 
        println!("best girl: {}", *page.ptr::<u8>());
    };

    let frame = alloc_frame_with_order(FrameOrder(9)).unwrap();
    println!("order-9 frame: {}", frame);
    let (l2_entry, big_page_addr) = mm::alloc_l2_entry().unwrap();
    println!("l2 entry vaddr: {}", big_page_addr);
    *l2_entry = Entry::at_frame(frame);
    let big_page = unsafe { &mut *big_page_addr.ptr::<[MaybeUninit<u8>; 1<<21]>() };
    let seg = unsafe { mm::Segment::make_small_alloc(&mut big_page[..]) };
    /*let slab = unsafe { seg.alloc_slab(Layout::new::<usize>()) }.unwrap();
    let num = unsafe {
        let ptr = slab.alloc_fast().unwrap() as *mut usize;
        println!("alloced usize: {}", ptr.virt_addr());
        ptr.write(21);
        &mut *ptr
    };
    println!("val: {}", num);
    core::mem::drop(num);
    core::mem::drop(slab);
    core::mem::drop(seg);
    free_frame_with_order(frame, FrameOrder(9));*/

    let slab = unsafe { seg.alloc_slab(Layout::new::<[u64; 128]>()) }.unwrap();
    let stack: &'static mut [u8] = unsafe { &mut *(slab.alloc_fast().unwrap() as *mut [u8; 1024] as *mut [u8]) };


    let mut curr_tcb = Cell::new(TCB::new_empty());
    println!("hwt: {:08x}", hello_world_task as u64);
    let mut next_tcb = Cell::new(TCB::new_with_stack(stack, hello_world_task));
    let mut curr_tcb_reg = &mut curr_tcb;

    println!("switching now");
    unsafe { switch_to_task(&mut curr_tcb_reg, &mut next_tcb) };
}

fn hello_world_task() {
    println!("hello from a thread");
}