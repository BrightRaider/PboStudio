//! The pure part of PboStudio: conversions, layouts and decoders.
//!
//! Nothing here touches the operating system, a file, the clock or the hardware, and the crate has no
//! dependencies (ADR-0003). Whatever the hardware layer reads arrives here as plain numbers, and every
//! decision about what those numbers mean is made here, where it can be tested without a machine.
//!
//! Provenance of the constants: the SMU message, the PM table positions and the fuse addresses are taken
//! from the vendored ZenStates-Core (GPL-3.0) and from the existing C# program, which checked the Zen 3
//! values on a Ryzen 7 5800X3D. Where a value is not verified on hardware it is marked as such and says
//! so in its documentation (ADR-0005).

#![forbid(unsafe_code)]

pub mod apic;
pub mod auto;
pub mod campaign;
pub mod coremap;
pub mod depth;
pub mod download;
pub mod duration;
pub mod engine;
pub mod estimate;
pub mod families;
pub mod interrupted;
pub mod knowledge;
pub mod logday;
pub mod logtail;
pub mod onerror;
pub mod order;
pub mod profile;
pub mod profiles;
pub mod quality;
pub mod readiness;
pub mod runstate;
pub mod search;
pub mod transient;
pub mod tuner;
pub mod version;
pub mod whea;

/// Conversions between the voltage codes the processor reports and volts.
pub mod voltage {
    /// SVI2 (Zen 2 and Zen 3): `1.55 V - code * 6.25 mV`.
    pub fn svi2(code: u32) -> f64 {
        1.55 - f64::from(code) * 0.00625
    }

    /// SVI3 (Zen 4 and newer): `0.245 V + code * 5 mV`, rounded to millivolts like the C# helper.
    /// Not verified on hardware here.
    pub fn svi3(code: u32) -> f64 {
        ((0.245 + f64::from(code) * 0.005) * 1000.0).round() / 1000.0
    }

    /// The core voltage code in the SVI2 telemetry register (bits 23:16).
    pub fn svi2_telemetry_code(raw: u32) -> u32 {
        (raw >> 16) & 0xFF
    }
}

/// The hardware P-state status register, MSR C001_0293.
pub mod pstate {
    pub const MSR: u32 = 0xC001_0293;

    /// What one core currently asks for. The frequency is the *requested* state, not the measured clock.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Status {
        pub fid: u32,
        pub dfs_id: u32,
        pub vid: u32,
        pub hw_pstate: u32,
    }

    impl Status {
        pub fn decode(eax: u32) -> Status {
            Status {
                fid: eax & 0xFF,
                dfs_id: (eax >> 8) & 0x3F,
                vid: (eax >> 14) & 0xFF,
                hw_pstate: (eax >> 22) & 0x7,
            }
        }

        /// Requested frequency in MHz: `fid * 25 / (dfs / 8)`. Zero when the divider is zero.
        pub fn requested_mhz(&self) -> f64 {
            if self.dfs_id == 0 {
                0.0
            } else {
                f64::from(self.fid) * 25.0 * 8.0 / f64::from(self.dfs_id)
            }
        }
    }
}

/// Curve Optimizer reads.
pub mod margin {
    /// The margin the SMU returns, as a signed number. Negative values come back as two's complement.
    pub fn decode(raw: u32) -> i32 {
        raw as i32
    }

    /// The low 16 bits of the set message argument: the margin as two's complement.
    pub fn psm_arg(margin: i32) -> u32 {
        (margin as u32) & 0xFFFF
    }

    /// The whole argument of the set message: the core in bits 31:20, the margin in the low 16 bits.
    pub fn set_argument(core_mask: u32, margin: i32) -> u32 {
        (core_mask & 0xFFF0_0000) | psm_arg(margin)
    }

    /// The core mask the SMU addresses a core by. From Zen 3 on (family above 17h) it is the chiplet in
    /// bits 31:28 and the slot within the chiplet in bits 22:20.
    pub fn core_mask(family: u32, ccd: u32, slot: u32) -> u32 {
        if family > 0x17 {
            (ccd << 28) | ((slot % 8) << 20)
        } else {
            // Zen and Zen 2: complex number in bit 24, four slots per complex.
            (ccd << 28) | ((slot / 4 % 2) << 24) | ((slot % 4) << 20)
        }
    }
}

/// Who the processor is, from CPUID leaf 1.
pub mod cpu {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Id {
        pub family: u32,
        pub model: u32,
        pub stepping: u32,
    }

    impl Id {
        /// Decodes CPUID leaf 1, EAX, with the extended family and model added in.
        pub fn decode(eax: u32) -> Id {
            let family = ((eax & 0xF00) >> 8) + ((eax & 0xFF0_0000) >> 20);
            let base_model = (eax & 0xF0) >> 4;
            let ext_model = (eax & 0xF_0000) >> 16;
            Id {
                family,
                model: base_model + ext_model * 0x10,
                stepping: eax & 0xF,
            }
        }

        /// A name for the generation and whether this program has checked it on hardware.
        /// Only Zen 3 desktop (Vermeer, model 21h) has been checked.
        pub fn generation(&self) -> (&'static str, bool) {
            match (self.family, self.model) {
                (0x19, 0x21) => ("Zen 3 (Vermeer)", true),
                (0x19, 0x00..=0x5F) => ("Zen 3 family", false),
                (0x19, 0x60..=0x7F) => ("Zen 4 (Raphael family)", false),
                (0x19, _) => ("Zen 4 family", false),
                (0x1A, _) => ("Zen 5 family", false),
                (0x17, _) => ("Zen, Zen+ or Zen 2", false),
                _ => ("unknown", false),
            }
        }
    }
}

/// Where the live readings sit in the power monitoring table the SMU publishes into DRAM.
pub mod pm {
    /// Positions of the readings, as indices into the table of 32-bit floats.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Layout {
        pub name: &'static str,
        pub verified: bool,
        pub ppt_limit: Option<usize>,
        pub ppt_now: Option<usize>,
        pub tdc_limit: Option<usize>,
        pub tdc_now: Option<usize>,
        pub edc_limit: Option<usize>,
        pub edc_now: Option<usize>,
        pub temp_limit: Option<usize>,
        pub temp_now: Option<usize>,
    }

    /// Zen 3 (Vermeer), table versions 0x3808xx and 0x3809xx. Checked on a Ryzen 7 5800X3D.
    pub const ZEN3: Layout = Layout {
        name: "Zen 3",
        verified: true,
        ppt_limit: Some(0),
        ppt_now: Some(1),
        tdc_limit: Some(2),
        tdc_now: Some(3),
        edc_limit: Some(8),
        edc_now: Some(9),
        temp_limit: Some(4),
        temp_now: Some(5),
    };

    /// Zen 4 (Raphael), table version 0x540004. The positions come from the library's sensor definitions,
    /// not from a measurement here. The limits are not defined there, so they stay empty.
    pub const ZEN4: Layout = Layout {
        name: "Zen 4",
        verified: false,
        ppt_limit: None,
        ppt_now: Some(3),
        tdc_limit: None,
        tdc_now: Some(48),
        edc_limit: None,
        edc_now: Some(49),
        temp_limit: None,
        temp_now: Some(11),
    };

    /// The layout for a table version, or `None` when it is not known. `None` is the point: reading
    /// another generation's positions anyway is what once produced zeros on a Zen 4 part.
    pub fn layout_for(version: u32) -> Option<Layout> {
        if version & 0xFFFF_0000 == 0x0038_0000 {
            Some(ZEN3)
        } else if version == 0x0054_0004 {
            Some(ZEN4)
        } else {
            None
        }
    }

    /// Power and current: finite, positive and not absurd.
    pub fn reading(v: f32) -> Option<f32> {
        (v.is_finite() && v > 0.0 && v < 10_000.0).then_some(v)
    }

    /// A package temperature has a far narrower plausible range than any other figure.
    pub fn temperature(v: f32) -> Option<f32> {
        (v.is_finite() && (1.0..=125.0).contains(&v)).then_some(v)
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct Snapshot {
        pub ppt_limit: Option<f32>,
        pub ppt_now: Option<f32>,
        pub tdc_limit: Option<f32>,
        pub tdc_now: Option<f32>,
        pub edc_limit: Option<f32>,
        pub edc_now: Option<f32>,
        pub temp_limit: Option<f32>,
        pub temp_now: Option<f32>,
    }

    impl Snapshot {
        /// Whether the table gave a power limit at all (the C# `HasLimits`).
        pub fn has_limits(&self) -> bool {
            self.ppt_limit.is_some_and(|v| v > 0.0)
        }
    }

    impl Layout {
        /// The largest index this layout reads, so a short table can be rejected whole.
        pub fn highest_index(&self) -> usize {
            [
                self.ppt_limit,
                self.ppt_now,
                self.tdc_limit,
                self.tdc_now,
                self.edc_limit,
                self.edc_now,
                self.temp_limit,
                self.temp_now,
            ]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or(0)
        }

        /// The readings out of a raw table, or `None` when the table is too short for this layout.
        /// A value that fails its plausibility check is left empty, never shown as a number.
        pub fn read(&self, table: &[f32]) -> Option<Snapshot> {
            if table.len() <= self.highest_index() {
                return None;
            }
            let at =
                |i: Option<usize>, check: fn(f32) -> Option<f32>| i.and_then(|i| check(table[i]));
            Some(Snapshot {
                ppt_limit: at(self.ppt_limit, reading),
                ppt_now: at(self.ppt_now, reading),
                tdc_limit: at(self.tdc_limit, reading),
                tdc_now: at(self.tdc_now, reading),
                edc_limit: at(self.edc_limit, reading),
                edc_now: at(self.edc_now, reading),
                temp_limit: at(self.temp_limit, temperature),
                temp_now: at(self.temp_now, temperature),
            })
        }
    }

    /// Size in bytes of the table for a version, from the vendored library. `None` when unknown.
    pub fn table_size(version: u32) -> Option<usize> {
        Some(match version {
            0x2D0903 => 0x594,
            0x380904 => 0x5A4,
            0x380905 => 0x5D0,
            0x2D0803 => 0x894,
            0x380804 => 0x8A4,
            0x380805 => 0x8F0,
            0x540004 => 0x948,
            0x540104 => 0x950,
            0x620205 => 0x994,
            _ => return None,
        })
    }
}

/// Chiplet and core layout from the fuses.
pub mod topology {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FuseAddresses {
        /// Register with the chiplets that are present.
        pub ccds_present: u32,
        /// Register with the chiplets that are down.
        pub ccds_down: u32,
        /// Base of the per-chiplet core disable map. Chiplet `n` adds `n << 25`.
        pub core_disable: u32,
        /// Complexes per chiplet: two before Zen 3, one from Zen 3 on.
        pub ccx_per_ccd: u32,
    }

    /// The fuse addresses for a processor, as the library derives them. Verified on Zen 3 (Vermeer);
    /// the Raphael and family 1Ah branches are the library's and unchecked here.
    pub fn fuse_addresses(family: u32, model: u32) -> FuseAddresses {
        let mut fuse1 = 0x5D218;
        let mut fuse2 = 0x5D21C;
        let mut offset = 0x238;
        let mut ccx_per_ccd = 2;

        if family == 0x19 {
            offset = 0x598;
            ccx_per_ccd = 1;
            // Raphael and Dragon Range share family 19h model 61h.
            if model == 0x61 {
                offset = 0x4D0;
                fuse1 += 0x1A4;
                fuse2 += 0x1A4;
            }
        } else if family == 0x17 && model != 0x71 && model != 0x31 {
            fuse1 += 0x40;
            fuse2 += 0x40;
        } else if family == 0x1A {
            ccx_per_ccd = 1;
            fuse1 += 0x1A4;
            fuse2 += 0x1A4;
        }

        let core_disable = if family == 0x1A {
            0x304A_03DC
        } else {
            0x3008_1800 + offset
        };
        FuseAddresses {
            ccds_present: fuse1,
            ccds_down: fuse2,
            core_disable,
            ccx_per_ccd,
        }
    }

    /// Bits `hi..=lo` of `value`.
    pub fn bits(value: u32, hi: u32, lo: u32) -> u32 {
        (value >> lo) & ((1u32 << (hi - lo + 1)) - 1)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Chiplets {
        pub enable_map: u32,
        pub disable_map: u32,
        pub count: u32,
    }

    pub fn chiplets(ccds_present: u32, ccds_down: u32) -> Chiplets {
        let enable_map = bits(ccds_present, 23, 22);
        let disable_map = bits(ccds_present, 31, 30) | (bits(ccds_down, 5, 0) << 2);
        let enabled = enable_map.count_ones();
        Chiplets {
            enable_map,
            disable_map,
            count: if enabled > 0 { enabled } else { 1 },
        }
    }

    /// Enabled cores in one chiplet from its core disable map (a set bit is a disabled core).
    pub fn enabled_cores(core_disable_map: u32) -> u32 {
        8 - (core_disable_map & 0xFF).count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svi2_codes_give_the_volts_measured_on_the_5800x3d() {
        // Five samples read from the SVI2 telemetry plane: codes 53, 56, 56, 62, 62.
        let volts: Vec<f64> = [53, 56, 56, 62, 62]
            .iter()
            .map(|c| (voltage::svi2(*c) * 1000.0).round() / 1000.0)
            .collect();
        assert_eq!(volts, vec![1.219, 1.2, 1.2, 1.163, 1.163]);
    }

    #[test]
    fn svi3_codes_are_rounded_to_millivolts() {
        assert_eq!(voltage::svi3(100), 0.745);
        assert_eq!(voltage::svi3(0), 0.245);
    }

    #[test]
    fn the_telemetry_register_carries_the_code_in_bits_23_to_16() {
        assert_eq!(voltage::svi2_telemetry_code(0x0135_0013), 53);
        assert_eq!(voltage::svi2_telemetry_code(0x013E_000A), 62);
    }

    #[test]
    fn the_pstate_register_decodes_its_fields() {
        // fid 0x8E, dfs 8, vid 54, state 0.
        let eax = 0x8E | (8 << 8) | (54 << 14);
        let s = pstate::Status::decode(eax);
        assert_eq!((s.fid, s.dfs_id, s.vid, s.hw_pstate), (0x8E, 8, 54, 0));
        assert!((s.requested_mhz() - 3550.0).abs() < 0.001);
        assert_eq!(pstate::Status::decode(0).requested_mhz(), 0.0);
    }

    #[test]
    fn a_margin_comes_back_signed() {
        assert_eq!(margin::decode(0xFFFF_FFE2), -30);
        assert_eq!(margin::decode(0), 0);
        assert_eq!(margin::decode(5), 5);
    }

    #[test]
    fn the_core_mask_puts_chiplet_and_slot_where_the_smu_wants_them() {
        assert_eq!(margin::core_mask(0x19, 0, 3), 0x0030_0000);
        assert_eq!(margin::core_mask(0x19, 1, 7), 0x1070_0000);
        // Slot 9 wraps: the SMU only knows eight slots per chiplet.
        assert_eq!(margin::core_mask(0x19, 0, 9), 0x0010_0000);
        // Zen 2 addresses four slots in each of two complexes.
        assert_eq!(margin::core_mask(0x17, 0, 5), (1 << 24) | (1 << 20));
    }

    #[test]
    fn cpuid_leaf_1_names_the_5800x3d_as_vermeer() {
        // Family 19h, model 21h, stepping 2.
        let id = cpu::Id::decode(0x00A2_0F12);
        assert_eq!((id.family, id.model, id.stepping), (0x19, 0x21, 2));
        assert_eq!(id.generation(), ("Zen 3 (Vermeer)", true));
    }

    #[test]
    fn only_zen3_desktop_counts_as_verified() {
        let raphael = cpu::Id {
            family: 0x19,
            model: 0x61,
            stepping: 2,
        };
        assert_eq!(raphael.generation(), ("Zen 4 (Raphael family)", false));
        let zen5 = cpu::Id {
            family: 0x1A,
            model: 0x44,
            stepping: 0,
        };
        assert!(!zen5.generation().1);
    }

    #[test]
    fn a_known_table_version_has_a_layout_and_an_unknown_one_does_not() {
        assert_eq!(pm::layout_for(0x0038_0805).map(|l| l.name), Some("Zen 3"));
        assert_eq!(pm::layout_for(0x0054_0004).map(|l| l.name), Some("Zen 4"));
        assert!(pm::layout_for(0x0062_0205).is_none());
        assert!(pm::layout_for(0).is_none());
    }

    #[test]
    fn the_zen3_layout_reads_what_the_bios_shows() {
        let mut table = vec![0.0f32; 16];
        table[0] = 142.0;
        table[1] = 54.0;
        table[2] = 95.0;
        table[3] = 21.0;
        table[4] = 90.0;
        table[5] = 64.0;
        table[8] = 130.0;
        table[9] = 122.0;
        let s = pm::ZEN3.read(&table).unwrap();
        assert_eq!((s.ppt_limit, s.ppt_now), (Some(142.0), Some(54.0)));
        assert_eq!((s.tdc_limit, s.tdc_now), (Some(95.0), Some(21.0)));
        assert_eq!((s.edc_limit, s.edc_now), (Some(130.0), Some(122.0)));
        assert_eq!((s.temp_limit, s.temp_now), (Some(90.0), Some(64.0)));
    }

    #[test]
    fn a_misread_table_shows_nothing_instead_of_a_wrong_number() {
        let mut table = vec![0.0f32; 16];
        table[5] = 300.0; // not a temperature
        table[1] = f32::NAN; // not a power
        let s = pm::ZEN3.read(&table).unwrap();
        assert_eq!(s.temp_now, None);
        assert_eq!(s.ppt_now, None);
    }

    #[test]
    fn a_table_that_is_too_short_is_rejected_whole() {
        assert!(pm::ZEN3.read(&[1.0; 5]).is_none());
        assert!(pm::ZEN4.read(&[1.0; 49]).is_none());
        assert!(pm::ZEN4.read(&[1.0; 50]).is_some());
    }

    #[test]
    fn table_sizes_come_from_the_library() {
        assert_eq!(pm::table_size(0x380805), Some(0x8F0));
        assert_eq!(pm::table_size(0x540004), Some(0x948));
        assert_eq!(pm::table_size(0x1234), None);
    }

    #[test]
    fn fuse_addresses_follow_the_family() {
        let vermeer = topology::fuse_addresses(0x19, 0x21);
        assert_eq!(vermeer.ccds_present, 0x5D218);
        assert_eq!(vermeer.core_disable, 0x3008_1D98);
        assert_eq!(vermeer.ccx_per_ccd, 1);

        let raphael = topology::fuse_addresses(0x19, 0x61);
        assert_eq!(raphael.ccds_present, 0x5D3BC);
        assert_eq!(raphael.core_disable, 0x3008_1CD0);

        let zen2 = topology::fuse_addresses(0x17, 0x71);
        assert_eq!(zen2.ccds_present, 0x5D218);
        assert_eq!(zen2.ccx_per_ccd, 2);

        assert_eq!(
            topology::fuse_addresses(0x1A, 0x44).core_disable,
            0x304A_03DC
        );
    }

    #[test]
    fn chiplets_come_from_the_two_fuse_registers() {
        // One chiplet present (bit 22), nothing down.
        let c = topology::chiplets(1 << 22, 0);
        assert_eq!((c.enable_map, c.count), (0b01, 1));
        // Two chiplets present.
        let c = topology::chiplets(0b11 << 22, 0);
        assert_eq!((c.enable_map, c.count), (0b11, 2));
        // No chiplet bits at all still counts as one.
        assert_eq!(topology::chiplets(0, 0).count, 1);
    }

    #[test]
    fn enabled_cores_counts_the_slots_that_are_not_fused_off() {
        assert_eq!(topology::enabled_cores(0b1100_0000), 6);
        assert_eq!(topology::enabled_cores(0), 8);
    }
}
