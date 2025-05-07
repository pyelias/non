#![no_std]
#![no_main]
#![feature(maybe_uninit_slice)]
#![feature(alloc_layout_extra)]
#![allow(dead_code)]

extern crate alloc;

use core::fmt::Write;
use core::panic::PanicInfo;

#[macro_use]
mod io;

mod acpi;
mod apic;
mod asm;
mod data_structures;
mod entry;
mod hang;
mod int;
mod mm;
mod multiboot;
mod sync;
mod task;
mod types;
mod util;

#[panic_handler]
unsafe fn panic(info: &PanicInfo) -> ! {
    // using "_ =" ignores the Result returned by writeln
    // there's not much that can be done about an error here
    _ = writeln!(io::SerialOut, "Rust panicked");

    _ = write!(io::SerialOut, "{}", info.message());
    _ = writeln!(io::SerialOut, ""); // newline

    if let Some(loc) = info.location() {
        _ = writeln!(
            io::SerialOut,
            "at {}:{}:{}",
            loc.file(),
            loc.line(),
            loc.column()
        );
    }

    _ = writeln!(io::SerialOut, "Now halting");

    hang::hang();
}
