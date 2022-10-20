use core::fmt;

pub struct SerialOut;

impl fmt::Write for SerialOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        extern "sysv64" {
            fn putchar(c: u8);
        }
        for c in s.bytes() {
            unsafe { putchar(c) };
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