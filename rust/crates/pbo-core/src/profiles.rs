//! The ready-made test lists of 1.x, as data (ux.md: they become templates that pre-fill the plan).
//!
//! Only the numbers and the shape are here; names and explanations are language and live with the texts. A profile is
//! a list of legs (an engine with its mode and FFT range), a time per core, a number of passes, and how the load is
//! interrupted. The numbers are pinned against the C# list by `spec/vectors/test-profiles.json`.

use crate::engine::{FftPreset, Prime95Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Prime95,
    YCruncher,
}

impl Engine {
    pub fn name(self) -> &'static str {
        match self {
            Engine::Prime95 => "Prime95",
            Engine::YCruncher => "YCruncher",
        }
    }
}

/// One leg of a run. For y-cruncher the mode and the FFT range are carried but not used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leg {
    pub engine: Engine,
    pub mode: Prime95Mode,
    pub fft: FftPreset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// All the tests on one core, then the next core.
    Sequential,
    /// Alternating between the cores.
    Alternate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transient {
    /// A pause every so many seconds, so the core drops to idle and boosts back up.
    Periodic,
    /// Load switched on and off in fractions of a second, the way a game engine does.
    MicroBurst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: &'static str,
    pub minutes_per_core: u32,
    pub iterations: u32,
    pub threads: u32,
    pub order: Order,
    pub suspend_every_seconds: u32,
    pub suspend_for_seconds: u32,
    pub transient: Transient,
    /// One of the five that make up the path the program itself recommends.
    pub essential: bool,
    pub legs: Vec<Leg>,
}

fn p95(mode: Prime95Mode, fft: FftPreset) -> Leg {
    Leg {
        engine: Engine::Prime95,
        mode,
        fft,
    }
}

fn yc(mode: Prime95Mode, fft: FftPreset) -> Leg {
    Leg {
        engine: Engine::YCruncher,
        mode,
        fft,
    }
}

#[allow(clippy::too_many_arguments)]
fn profile(
    id: &'static str,
    minutes: u32,
    iterations: u32,
    threads: u32,
    order: Order,
    legs: Vec<Leg>,
    essential: bool,
) -> Profile {
    Profile {
        id,
        minutes_per_core: minutes,
        iterations,
        threads,
        order,
        suspend_every_seconds: 30,
        suspend_for_seconds: 1,
        transient: Transient::Periodic,
        essential,
        legs,
    }
}

/// All profiles, in the order the list shows them.
pub fn all() -> Vec<Profile> {
    use FftPreset::*;
    use Order::*;
    use Prime95Mode::*;

    let mut breaking = profile(
        "BreakingPoint",
        10,
        1,
        2,
        Sequential,
        vec![p95(Avx2, Smallest)],
        false,
    );
    breaking.suspend_every_seconds = 5;
    breaking.suspend_for_seconds = 1;

    let mut micro = profile(
        "MicroBurst",
        10,
        2,
        1,
        Sequential,
        vec![p95(Sse, HeavyShort)],
        true,
    );
    micro.transient = Transient::MicroBurst;

    vec![
        profile(
            "RecommendedCombo",
            6,
            3,
            1,
            Alternate,
            vec![p95(Sse, Huge), yc(Avx2, Smallest)],
            true,
        ),
        profile("HeavyFfts", 6, 3, 1, Alternate, vec![p95(Sse, Heavy)], true),
        profile(
            "FullPrime95",
            6,
            3,
            1,
            Alternate,
            vec![p95(Sse, Huge), p95(Avx2, Smallest)],
            false,
        ),
        profile("QuickCheck", 3, 1, 1, Alternate, vec![p95(Sse, Huge)], true),
        breaking,
        profile(
            "HeavyLoad",
            10,
            1,
            2,
            Sequential,
            vec![p95(Avx2, Smallest)],
            false,
        ),
        profile(
            "YCruncherCo",
            6,
            3,
            1,
            Alternate,
            vec![yc(Sse, Smallest)],
            false,
        ),
        profile(
            "YCruncherAll",
            10,
            2,
            1,
            Alternate,
            vec![yc(Sse, Smallest)],
            false,
        ),
        profile("Step1Sse", 6, 3, 1, Alternate, vec![p95(Sse, Huge)], false),
        profile(
            "Step2Avx2",
            6,
            3,
            1,
            Alternate,
            vec![p95(Avx2, Smallest)],
            false,
        ),
        profile(
            "SmallFfts",
            6,
            2,
            1,
            Alternate,
            vec![p95(Sse, HeavyShort)],
            false,
        ),
        profile(
            "Overnight",
            10,
            3,
            1,
            Alternate,
            vec![
                p95(Sse, All),
                p95(Avx, All),
                p95(Avx2, All),
                yc(Avx2, Smallest),
            ],
            true,
        ),
        profile(
            "MemoryFabric",
            15,
            1,
            2,
            Sequential,
            vec![p95(Avx2, Large)],
            false,
        ),
        micro,
    ]
}

pub fn by_id(id: &str) -> Option<Profile> {
    all().into_iter().find(|p| p.id == id)
}

/// What a profile costs, in the form the campaign estimate takes.
pub fn phase_profile(id: &str) -> Option<crate::estimate::PhaseProfile> {
    by_id(id).map(|p| crate::estimate::PhaseProfile {
        minutes_per_core: p.minutes_per_core,
        iterations: p.iterations,
        legs: p.legs.len() as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_fourteen_and_five_are_essential() {
        let all = all();
        assert_eq!(all.len(), 14);
        assert_eq!(all.iter().filter(|p| p.essential).count(), 5);
    }

    #[test]
    fn the_three_campaign_profiles_exist() {
        for id in ["RecommendedCombo", "HeavyFfts", "Overnight"] {
            assert!(phase_profile(id).is_some(), "{id}");
        }
    }

    #[test]
    fn every_profile_has_a_leg_and_an_unique_id() {
        let all = all();
        let mut ids: Vec<&str> = all.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all.len());
        assert!(all.iter().all(|p| !p.legs.is_empty()));
    }
}
