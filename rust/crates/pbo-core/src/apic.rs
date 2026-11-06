//! Which core a machine check belongs to (invariant I4).
//!
//! A machine check names the processor that raised it by APIC id. Treating the id as the logical processor's
//! number is true on the part it was checked on and an assumption everywhere else: with cores fused off the ids
//! can skip numbers, and a genuine fault on the core under test would be credited to another core and the core
//! would pass. The processor says what its ids are (CPUID leaf 0xB on each logical processor), so a measured map
//! is exact. Vectors: `spec/vectors/apic-topology.json`.

use std::collections::HashMap;

/// One physical core and the logical processors that live on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Core {
    pub index: usize,
    pub logical_processors: Vec<i32>,
}

/// The core owning an APIC id. With a measured map (`apic id -> logical processor`) the answer is exact, and an id
/// no processor owns blames no core. Without a map the id is taken as the logical processor's number.
pub fn core_for_apic_id(
    apic_id: i32,
    cores: &[Core],
    apic_to_logical: Option<&HashMap<i32, i32>>,
) -> Option<usize> {
    let logical = match apic_to_logical {
        Some(map) => *map.get(&apic_id)?,
        None => apic_id,
    };
    cores
        .iter()
        .find(|c| c.logical_processors.contains(&logical))
        .map(|c| c.index)
}
