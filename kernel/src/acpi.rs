use core::{
    mem::{size_of, transmute},
    ptr::NonNull,
};

use acpi::{rsdp::Rsdp, PhysicalMapping};

use crate::types::{HasPhysAddr, HasVirtAddr};

// use the identity map for the first 2MB
#[derive(Clone)]
pub struct IdMapper;

impl acpi::AcpiHandler for IdMapper {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        // make sure the end of the mapping fits in the id-map
        (physical_address + size - 1).to_virt();
        PhysicalMapping::new(
            physical_address,
            NonNull::new(physical_address.to_virt().ptr()).unwrap(),
            size,
            0,
            self.clone(),
        )
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // don't unmap anything
    }
}

pub fn rsdp_from_buf(buf: &[u8]) -> Rsdp {
    let mut rsdp = [0u8; size_of::<Rsdp>()];
    rsdp[..buf.len()].copy_from_slice(buf);
    unsafe { transmute(rsdp) }
}

pub unsafe fn acpi_tables_from_rsdp(rsdp: Rsdp) -> acpi::AcpiResult<acpi::AcpiTables<IdMapper>> {
    rsdp.validate()?;
    acpi::AcpiTables::from_validated_rsdp(IdMapper, rsdp)
}
