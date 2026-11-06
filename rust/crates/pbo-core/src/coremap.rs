//! Where the SMU addresses each core (invariant I3).
//!
//! Windows lists the enabled cores; the SMU is addressed by (chiplet, physical slot), eight slots per
//! chiplet, and a part with fewer cores has the rest fused off. The fuse map the processor reports says which
//! slots are enabled: the k-th enabled slot of a chiplet is the k-th core Windows lists for it. A mapping
//! that does not add up is refused, never guessed. Vectors: `spec/vectors/smu-core-map.json`.

pub const SLOTS_PER_CCD: u32 = 8;

/// Which chiplet and which of its eight physical slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    pub ccd: u32,
    pub slot: u32,
}

/// One slot per core, in core order, from the fuse map: one mask per chiplet, bit N set meaning slot N is
/// fused off. `None` when there is no fuse data, no cores, or the fuses and Windows disagree about how many
/// cores there are.
pub fn build(disabled_slots_per_ccd: &[u32], windows_core_count: usize) -> Option<Vec<Slot>> {
    if disabled_slots_per_ccd.is_empty() || windows_core_count == 0 {
        return None;
    }

    let mut map = Vec::with_capacity(windows_core_count);
    for (ccd, disabled) in disabled_slots_per_ccd.iter().enumerate() {
        for slot in 0..SLOTS_PER_CCD {
            if disabled & (1 << slot) == 0 {
                map.push(Slot {
                    ccd: ccd as u32,
                    slot,
                });
            }
        }
    }

    (map.len() == windows_core_count).then_some(map)
}

/// The fallback when no fuse map is available: assume the fused-off slots are the top ones and divide the
/// cores Windows reports evenly between the chiplets. `None` for what it cannot place.
pub fn fallback(core: i32, ccd_count: i32, windows_core_count: i32) -> Option<Slot> {
    if core < 0 || ccd_count <= 0 || windows_core_count <= 0 || windows_core_count % ccd_count != 0
    {
        return None;
    }
    let per_ccd = windows_core_count / ccd_count;
    (core < windows_core_count).then(|| Slot {
        ccd: (core / per_ccd) as u32,
        slot: (core % per_ccd) as u32,
    })
}
