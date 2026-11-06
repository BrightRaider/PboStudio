//! What this program knows about each processor family, and how much of it has been checked on hardware.
//!
//! The rule behind the `verified` flag is the one the whole rebuild follows: a value that was read from the library
//! but never seen working on a real chip is shown as such, and anything that writes to the processor asks for the
//! read-back check before it is trusted. Only the Zen 3 desktop part (Vermeer) has been checked so far.

use crate::cpu::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    ZenOne,
    Zen2,
    Zen3,
    Zen4,
    Zen5,
    Unknown,
}

/// Where the core voltage can be read from, as far as it is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vcore {
    /// SVI2 telemetry planes over SMN (core, SoC), as on Zen 3. The per-core VID MSR works alongside.
    Svi2 { core_plane: u32, soc_plane: u32 },
    /// SVI3 parts (Zen 4 and newer): the per-core VID MSR is decoded with the SVI3 formula; the telemetry plane
    /// addresses are not known to this program.
    Svi3PerCoreOnly,
    /// Nothing is known to read.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    pub name: &'static str,
    pub generation: Generation,
    /// Whether this program has run on such a chip and the readings and writes held up.
    pub verified: bool,
    /// Curve Optimizer exists from Zen 3 on.
    pub curve_optimizer: bool,
    /// How deep the search may start (the standard on Zen 3, the widest anyone has claimed on newer parts); the
    /// processor corrects it through the read-back.
    pub start_depth: i32,
    pub vcore: Vcore,
}

/// The family for a processor id.
pub fn lookup(id: Id) -> Family {
    use Generation::*;
    match (id.family, id.model) {
        (0x19, 0x21) => Family {
            name: "Zen 3 (Vermeer)",
            generation: Zen3,
            verified: true,
            curve_optimizer: true,
            start_depth: -30,
            vcore: Vcore::Svi2 {
                core_plane: 0x5A010,
                soc_plane: 0x5A00C,
            },
        },
        (0x19, 0x00..=0x5F) => Family {
            name: "Zen 3 family",
            generation: Zen3,
            verified: false,
            curve_optimizer: true,
            start_depth: -30,
            vcore: Vcore::None,
        },
        (0x19, _) => Family {
            name: "Zen 4 family",
            generation: Zen4,
            verified: false,
            curve_optimizer: true,
            start_depth: -50,
            vcore: Vcore::Svi3PerCoreOnly,
        },
        (0x1A, _) => Family {
            name: "Zen 5 family",
            generation: Zen5,
            verified: false,
            curve_optimizer: true,
            start_depth: -50,
            vcore: Vcore::Svi3PerCoreOnly,
        },
        (0x17, _) => Family {
            name: "Zen, Zen+ or Zen 2",
            generation: if id.model >= 0x30 { Zen2 } else { ZenOne },
            verified: false,
            curve_optimizer: false,
            start_depth: 0,
            vcore: Vcore::None,
        },
        _ => Family {
            name: "unknown",
            generation: Unknown,
            verified: false,
            curve_optimizer: false,
            start_depth: 0,
            vcore: Vcore::None,
        },
    }
}

/// The SMU message that reads a core's Curve Optimizer margin, per generation. The library gives Zen 3 the value it
/// inherits from Zen 2 (checked on a Ryzen 7 5800X3D) and Zen 4 and Zen 5 desktop their own; the latter two are the
/// library's numbers and nobody has checked them on hardware here. Reading with the wrong generation's message returns
/// nonsense, which is why the write path tests itself on an unchecked family before anything is written. Vectors:
/// `spec/vectors/smu-messages.json`.
pub fn get_margin_message(generation: Generation) -> Option<u32> {
    match generation {
        Generation::Zen3 => Some(0x7C),
        Generation::Zen4 | Generation::Zen5 => Some(0xD5),
        _ => None,
    }
}

/// The SMU message that sets a core's margin through the MP1 mailbox (Zen 3, Zen 4 and Zen 5 desktop).
pub fn set_margin_message_mp1(generation: Generation) -> Option<u32> {
    match generation {
        Generation::Zen3 | Generation::Zen4 | Generation::Zen5 => Some(0x35),
        _ => None,
    }
}

/// How far a first attempt should reach before the search refines it: short of the chip limit, aiming at a value
/// most cores clear. The X3D parts start deeper than the others (-20 against -15): the 1.x program started them
/// shallower (at -10) with no reason on record, and they are the parts that are said to go deep. The window offers
/// 10, 15 and 20, and this is the one it has chosen beforehand.
pub fn suggested_start(family: &Family, x3d: bool) -> i32 {
    if !family.curve_optimizer {
        0
    } else if x3d {
        -20
    } else {
        -15
    }
}

/// Whether a write to this family is allowed without the person confirming the self-test first: only a checked
/// family. Everything else must pass `co-selftest` (write the current value back, read it, compare) in that session.
pub fn needs_selftest(family: &Family) -> bool {
    !family.verified
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(family: u32, model: u32) -> Id {
        Id {
            family,
            model,
            stepping: 0,
        }
    }

    #[test]
    fn each_generation_reads_with_its_own_message() {
        assert_eq!(get_margin_message(Generation::Zen3), Some(0x7C));
        assert_eq!(get_margin_message(Generation::Zen4), Some(0xD5));
        assert_eq!(get_margin_message(Generation::Zen5), Some(0xD5));
        assert_eq!(get_margin_message(Generation::Zen2), None);
        assert_eq!(set_margin_message_mp1(Generation::Zen4), Some(0x35));
        assert_eq!(set_margin_message_mp1(Generation::ZenOne), None);
    }

    #[test]
    fn only_vermeer_is_verified() {
        assert!(lookup(id(0x19, 0x21)).verified);
        assert!(!lookup(id(0x19, 0x61)).verified);
        assert!(!lookup(id(0x1A, 0x44)).verified);
    }

    #[test]
    fn zen2_and_older_have_no_curve_optimizer() {
        assert!(!lookup(id(0x17, 0x71)).curve_optimizer);
        assert_eq!(lookup(id(0x17, 0x71)).generation, Generation::Zen2);
        assert_eq!(lookup(id(0x17, 0x08)).generation, Generation::ZenOne);
    }

    #[test]
    fn newer_parts_start_deep_and_older_standard() {
        assert_eq!(lookup(id(0x19, 0x21)).start_depth, -30);
        assert_eq!(lookup(id(0x19, 0x61)).start_depth, -50);
    }

    #[test]
    fn unchecked_families_need_the_selftest() {
        assert!(needs_selftest(&lookup(id(0x19, 0x61))));
        assert!(!needs_selftest(&lookup(id(0x19, 0x21))));
    }

    #[test]
    fn x3d_starts_deeper() {
        let f = lookup(id(0x19, 0x21));
        assert_eq!(suggested_start(&f, true), -20);
        assert_eq!(suggested_start(&f, false), -15);
        assert_eq!(suggested_start(&lookup(id(0x17, 0x71)), false), 0);
    }
}
