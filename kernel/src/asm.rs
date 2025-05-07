use core::arch::asm;

#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let mut res: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") res
    );
    res
}

#[inline(always)]
pub unsafe fn outb(port: u16, val: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") val
    );
}

#[inline(always)]
pub unsafe fn read_msr(msr: u32) -> u64 {
    let (hi, lo): (u32, u32);
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("edx") hi,
        out("eax") lo,
    );
    (hi as u64) << 32 | (lo as u64)
}

#[inline(always)]
pub unsafe fn write_msr(msr: u32, val: u64) {
    let hi = (val >> 32) as u32;
    let lo = val as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("edx") hi,
        in("eax") lo,
    );
}
