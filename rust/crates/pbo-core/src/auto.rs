//! How hard Auto works: three intensities, and what each one costs.
//!
//! A search that tests every single step at ten minutes is thorough, and for eight cores that have to walk from
//! "nothing" down to the chip limit it is also a day or more. Most of that time is spent confirming steps that were
//! never going to fail. So Auto has three speeds, and the person sees the worst case of the one chosen before it
//! starts:
//!
//! * **Fast**: big steps (three counts at a time), a few minutes per test. The headroom kept above the last value
//!   that held covers the resolution. An afternoon at most.
//! * **Normal**: big steps, minutes enough to catch what a short test misses. The default.
//! * **Thorough**: single steps and the long tests, the way the first program did it. A night to a weekend.
//!
//! The numbers are the minutes per test: one test is one engine on one core. A step with two engines (the first
//! phase) runs both. The estimate is a worst case: every core walks its whole way, which almost none does.

use std::collections::BTreeMap;

use crate::campaign::TOTAL_PHASES;
use crate::estimate::{self, Estimate, PhaseProfile};
use crate::knowledge::Knowledge;
use crate::tuner::Mode;

/// The step of the search that is chosen beforehand for the quick and the balanced run: with the safety kept at the end it
/// is as precise as a shorter step, and it takes the fewest tests.
pub const PRESELECTED_STEP: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intensity {
    Fast,
    #[default]
    Normal,
    Thorough,
}

impl Intensity {
    pub const ALL: [Intensity; 3] = [Intensity::Fast, Intensity::Normal, Intensity::Thorough];

    /// The name in files and on the command line.
    pub fn id(self) -> &'static str {
        match self {
            Intensity::Fast => "fast",
            Intensity::Normal => "normal",
            Intensity::Thorough => "thorough",
        }
    }

    pub fn from_id(id: &str) -> Option<Intensity> {
        Intensity::ALL
            .into_iter()
            .find(|i| i.id().eq_ignore_ascii_case(id.trim()))
    }

    /// How long one test runs in `phase`, in minutes: the first phase validates what is set, the second is the search
    /// (one test per step), the third confirms what the search left.
    pub fn minutes(self, phase: i32) -> u32 {
        let (validate, search, confirm) = match self {
            Intensity::Fast => (2, 3, 3),
            Intensity::Normal => (3, 5, 6),
            Intensity::Thorough => (6, 6, 10),
        };
        match phase {
            1 => validate,
            2 => search,
            _ => confirm,
        }
    }

    /// How far one step of the search goes.
    pub fn search_mode(self) -> Mode {
        match self {
            Intensity::Thorough => Mode::Fine,
            _ => Mode::Coarse,
        }
    }

    /// The steps the window and `pbo estimate` start from: single steps for the thorough run, and otherwise the step the
    /// program has judged efficient (`PRESELECTED_STEP`). The person can change it; `search_mode` is the older, fixed
    /// pairing of intensity and step and is kept as it was.
    pub fn preselected_mode(self) -> Mode {
        match self {
            Intensity::Thorough => Mode::Fine,
            _ => Mode::Steps(PRESELECTED_STEP),
        }
    }
}

/// What each campaign phase costs at this intensity: the minutes of the intensity, the legs of the phase's profile.
pub fn profiles(intensity: Intensity) -> BTreeMap<String, PhaseProfile> {
    let mut map = estimate::campaign_profiles();
    for phase in 1..=TOTAL_PHASES {
        if let Some(p) = map.get_mut(estimate::profile_for_phase(phase).name()) {
            p.minutes_per_core = intensity.minutes(phase);
            // One pass over the cores: the search itself is what repeats.
            p.iterations = 1;
        }
    }
    map
}

/// How long the campaign still has to run at this intensity, worst case, from `from_phase` on.
#[allow(clippy::too_many_arguments)]
pub fn estimate(
    intensity: Intensity,
    cores: &[u32],
    margins: &BTreeMap<u32, i32>,
    knowledge: &Knowledge,
    chip_limit: i32,
    guardband: i32,
    from_phase: i32,
) -> Estimate {
    estimate_with(
        intensity,
        intensity.search_mode(),
        cores,
        margins,
        knowledge,
        chip_limit,
        guardband,
        from_phase,
        0,
    )
}

/// The same with a step of one's own: the minutes of the intensity, the steps of `mode`.
#[allow(clippy::too_many_arguments)]
pub fn estimate_with(
    intensity: Intensity,
    mode: Mode,
    cores: &[u32],
    margins: &BTreeMap<u32, i32>,
    knowledge: &Knowledge,
    chip_limit: i32,
    guardband: i32,
    from_phase: i32,
    start: i32,
) -> Estimate {
    estimate::estimate_mode(
        &profiles(intensity),
        cores,
        margins,
        knowledge,
        chip_limit,
        guardband,
        from_phase,
        mode,
        start,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(i: Intensity) -> f64 {
        let cores: Vec<u32> = (0..8).collect();
        let held: BTreeMap<u32, i32> = cores.iter().map(|c| (*c, -10)).collect();
        estimate(i, &cores, &held, &Knowledge::new(), -30, 3, 1).total_minutes
    }

    #[test]
    fn the_faster_the_intensity_the_shorter_the_campaign() {
        let (fast, normal, thorough) = (
            total(Intensity::Fast),
            total(Intensity::Normal),
            total(Intensity::Thorough),
        );
        assert!(fast < normal, "{fast} < {normal}");
        assert!(normal < thorough, "{normal} < {thorough}");
    }

    #[test]
    fn eight_cores_at_the_default_intensity_are_well_under_a_day() {
        assert!(total(Intensity::Normal) < 24.0 * 60.0);
        assert!(total(Intensity::Fast) < 8.0 * 60.0);
    }

    #[test]
    fn the_names_round_trip() {
        for i in Intensity::ALL {
            assert_eq!(Intensity::from_id(i.id()), Some(i));
        }
        assert_eq!(Intensity::from_id(" Fast "), Some(Intensity::Fast));
        assert_eq!(Intensity::from_id("slow"), None);
    }

    #[test]
    fn the_default_is_normal() {
        assert_eq!(Intensity::default(), Intensity::Normal);
    }

    #[test]
    fn only_thorough_steps_one_at_a_time() {
        assert_eq!(Intensity::Thorough.search_mode(), Mode::Fine);
        assert_eq!(Intensity::Fast.search_mode(), Mode::Coarse);
        assert_eq!(Intensity::Normal.search_mode(), Mode::Coarse);
    }

    #[test]
    fn a_core_held_below_its_failure_costs_the_climb_back_up() {
        use crate::knowledge::CoreKnowledge;
        let held: BTreeMap<u32, i32> = [(0, -28)].into();
        let mut k = Knowledge::new();
        k.insert(
            0,
            CoreKnowledge {
                best_passed: None,
                worst_failed: Some(-27),
            },
        );
        let e = estimate(Intensity::Normal, &[0], &held, &k, -30, 0, 2);
        assert!(
            e.per_phase[0].1 > 0.0,
            "a core that is too deep still has to be searched"
        );
    }
}
