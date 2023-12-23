#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(maybe_uninit_slice)]
#![feature(alloc_layout_extra)]
#![allow(dead_code)]

use core::fmt::{write, Write};
use core::panic::PanicInfo;

#[macro_use]
mod io;

mod data_structures;
mod entry;
mod mm;
mod multiboot;
mod sync;
mod task;
mod types;

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
        _ = writeln!(
            io::SerialOut,
            "at {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        );
    }

    let payload = info.payload();
    if let Some(&msg) = payload.downcast_ref::<&'static str>() {
        _ = writeln!(io::SerialOut, "with payload {}", msg);
    }

    _ = writeln!(io::SerialOut, "Now halting");

    hang();
}
