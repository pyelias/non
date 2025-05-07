use crate::asm::outb;
use core::fmt;

const COM1: u16 = 0x3F8;

pub fn init_com1() {
    unsafe {
        // from osdev wiki
        outb(COM1 + 1, 0x00); // Disable all interrupts
        outb(COM1 + 3, 0x80); // Enable DLAB (set baud rate divisor)
        outb(COM1 + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        outb(COM1 + 1, 0x00); //                  (hi byte)
        outb(COM1 + 3, 0x03); // 8 bits, no parity, one stop bit
        outb(COM1 + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
        outb(COM1 + 4, 0x0B); // IRQs enabled, RTS/DSR set
        outb(COM1 + 4, 0x0F); // Set in normal operation mode
    }
}

#[inline(always)]
fn putchar(char: u8) {
    if char == b'\n' {
        unsafe { outb(COM1, b'\r') };
    }
    unsafe { outb(COM1, char) };
}

pub struct SerialOut;

impl fmt::Write for SerialOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // hope COM1 is ready
        for c in s.bytes() {
            putchar(c);
        }
        Ok(())
    }
}

macro_rules! println {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        writeln!(crate::io::SerialOut, $($arg)*).unwrap()
    }}
}
