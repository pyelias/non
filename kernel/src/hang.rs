use core::arch::asm;

pub fn hang() -> ! {
    loop {
        unsafe { asm!("cli", "hlt") };
    }
}
