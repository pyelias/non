use raw_cpuid::CpuId;

fn ensure_x2apic() {
    let feature_info = CpuId::new()
        .get_feature_info()
        .expect("can't get feature info from cpuid");

    // assert!(feature_info.has_x2apic(), "x2apic not supported");
    if !feature_info.has_x2apic() {
        println!("x2apic not supported");
    }
}

pub fn init() {
    ensure_x2apic();
}
