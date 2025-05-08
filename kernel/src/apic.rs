use core::sync::atomic::{AtomicU32, Ordering};

use raw_cpuid::CpuId;

use crate::{
    asm,
    types::{HasPhysAddr, HasVirtAddr, PageAddr},
};

const APIC_PAGE_ADDR: usize = 0xFEE00000;
const APIC_BASE_MSR: u32 = 0x1B;

fn ensure_x2apic() {
    let feature_info = CpuId::new()
        .get_feature_info()
        .expect("can't get feature info from cpuid");

    // assert!(feature_info.has_x2apic(), "x2apic not supported");
    if !feature_info.has_x2apic() {
        println!("x2apic not supported");
    }
}

fn check_apic_msr_enabled() {
    const ENABLED_FLAG: u64 = 1 << 11;

    let val = unsafe { asm::read_msr(APIC_BASE_MSR) };
    // it can be enabled, but i'm expecting it to already be on
    assert!(val & ENABLED_FLAG != 0, "APIC is disabled in MSR");
}

struct APICPage(PageAddr);

impl APICPage {
    fn get_reg_ptr(&self, offset: usize) -> *const u8 {
        assert!(offset < 0x40);
        unsafe { self.0.ptr::<u8>().add(offset * 16) }
    }

    fn get_reg32(&self, offset: usize) -> &AtomicU32 {
        unsafe { &*(self.get_reg_ptr(offset) as *const AtomicU32) }
    }

    fn svr(&self) -> &AtomicU32 {
        self.get_reg32(0xF)
    }
}

pub fn init() {
    ensure_x2apic();
    check_apic_msr_enabled();

    let apic = APICPage(APIC_PAGE_ADDR.to_virt().as_aligned());
    println!("svr val: {:16x}", apic.svr().load(Ordering::Relaxed));
}
