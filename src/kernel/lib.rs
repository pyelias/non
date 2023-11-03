#![no_std]
#![feature(pointer_byte_offsets)]
#![feature(const_option)]
#![feature(panic_info_message)]
#![feature(slice_take)]
#![feature(const_maybe_uninit_zeroed)]
#![feature(return_position_impl_trait_in_trait)]
#![feature(maybe_uninit_slice)]
#![feature(alloc_layout_extra)]

use core::panic::PanicInfo;
use core::fmt::{write, Write};

#[macro_use]
mod io;
mod types;
mod multiboot;
mod entry;
mod mm;
mod sync;
mod cpu;
mod task;

#[panic_handler]
unsafe fn panic(info: &PanicInfo) -> ! {
    extern "sysv64" {
        fn hang() -> !;
    }
    // using "_ =" ignores the Result returned by writeln
    // there's not much that can be done about an error here
    _ = writeln!(io::SerialOut, "Rust panicked");

    if let Some(msg) = info.message() {
        _ = write(&mut io::SerialOut, *msg);
        _ = writeln!(io::SerialOut, ""); // newline
    }

    if let Some(loc) = info.location() {
        _ = writeln!(io::SerialOut, "at {}:{}:{}", loc.file(), loc.line(), loc.column());
    }

    let payload = info.payload();
    if let Some(&msg) = payload.downcast_ref::<&'static str>() {
        _ = writeln!(io::SerialOut, "with payload {}", msg);
    }

    _ = writeln!(io::SerialOut, "Now halting");

    hang();
}