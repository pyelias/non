use crate::multiboot;
use crate::types::{HasPhysAddr};

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
    println!("  addr: {:x}", copy_packed_field!(entry.addr.usize()));
    println!("  len: {:x}", copy_packed_field!(entry.len));
}

#[no_mangle]
extern "sysv64" fn print_all_mmap_elements(info: &multiboot::Info) {
    for entry in info.mmap_entries() {
        print_mmap_element(entry);
    }
}