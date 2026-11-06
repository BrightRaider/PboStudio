//! How long the whole campaign still has to run, from what this machine currently holds (invariant I13).
//!
//! "A night to a weekend, depending on the processor" is true and useless: a processor with six cores already at the
//! chip limit and two to search is an evening. The numbers are all there (what each core holds, what it is allowed to
//! reach, what each phase's profile runs), so there is no reason to guess in prose. Vectors:
//! `spec/vectors/campaign-estimate.json`.

use std::collections::BTreeMap;

use crate::campaign::{Profile, TOTAL_PHASES};
use crate::knowledge::Knowledge;
use crate::tuner::{self, Mode};

/// What a profile costs: minutes per core, how many times each core is tested, and how many legs a run has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseProfile {
    pub minutes_per_core: u32,
    pub iterations: u32,
    pub legs: u32,
}

/// The three profiles the campaign is built from, with what each costs, taken from the profile list.
pub fn campaign_profiles() -> BTreeMap<String, PhaseProfile> {
    (1..=TOTAL_PHASES)
        .filter_map(|phase| {
            let name = profile_for_phase(phase).name();
            crate::profiles::phase_profile(name).map(|p| (name.to_string(), p))
        })
        .collect()
}

/// The profile each phase runs. It has to match what `campaign::recommend` hands back, or the estimate describes a
/// different campaign from the one that will happen.
pub fn profile_for_phase(phase: i32) -> Profile {
    match phase {
        1 => Profile::RecommendedCombo,
        2 => Profile::HeavyFfts,
        _ => Profile::Overnight,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// Everything still to do, from the phase the campaign is on to the end, in minutes.
    pub total_minutes: f64,
    /// What each remaining phase costs, in order.
    pub per_phase: Vec<(i32, f64)>,
}

/// `profiles` is keyed by `Profile::name()`. A phase whose profile is not there is left out.
pub fn estimate(
    profiles: &BTreeMap<String, PhaseProfile>,
    cores: &[u32],
    margins: &BTreeMap<u32, i32>,
    knowledge: &Knowledge,
    chip_limit: i32,
    guardband: i32,
    from_phase: i32,
) -> Estimate {
    estimate_mode(
        profiles,
        cores,
        margins,
        knowledge,
        chip_limit,
        guardband,
        from_phase,
        Mode::Fine,
        0,
    )
}

/// The same, for a search that steps in `mode`: the coarse search takes fewer, larger steps and so far fewer slots.
#[allow(clippy::too_many_arguments)]
pub fn estimate_mode(
    profiles: &BTreeMap<String, PhaseProfile>,
    cores: &[u32],
    margins: &BTreeMap<u32, i32>,
    knowledge: &Knowledge,
    chip_limit: i32,
    guardband: i32,
    from_phase: i32,
    mode: Mode,
    start: i32,
) -> Estimate {
    let mut per_phase = Vec::new();
    for phase in from_phase.max(1)..=TOTAL_PHASES {
        let Some(profile) = profiles.get(profile_for_phase(phase).name()) else {
            continue;
        };
        per_phase.push((
            phase,
            phase_minutes(
                profile, phase, cores, margins, knowledge, chip_limit, guardband, mode, start,
            ),
        ));
    }
    Estimate {
        total_minutes: per_phase.iter().map(|(_, m)| m).sum(),
        per_phase,
    }
}

#[allow(clippy::too_many_arguments)]
fn phase_minutes(
    profile: &PhaseProfile,
    phase: i32,
    cores: &[u32],
    margins: &BTreeMap<u32, i32>,
    knowledge: &Knowledge,
    chip_limit: i32,
    guardband: i32,
    mode: Mode,
    start: i32,
) -> f64 {
    let legs = f64::from(profile.legs);
    let minutes = f64::from(profile.minutes_per_core);

    // Phases one and three test every core a fixed number of times.
    if phase != 2 {
        return cores.len() as f64 * minutes * f64::from(profile.iterations) * legs;
    }

    // The search only visits cores with room left, and each of those costs as many slots as the walk down to its own
    // floor and back takes. That is the difference between an evening and a night.
    let mut slots: i64 = 0;
    for core in cores {
        let held = margins.get(core).copied().unwrap_or(0);
        let k = knowledge.get(core).copied();
        let floor = k.map_or(chip_limit, |k| k.floor_for(chip_limit));
        let target = tuner::apply_guardband(floor, guardband);
        // A core that holds a value it is known to fail at is searched too: it is taken back up.
        let known_bad = k.is_some_and(|k| k.is_known_bad(held));
        if held <= target && !known_bad {
            continue;
        }
        // A core with a value on record that held cannot be walked back up from scratch: the search returns to that
        // value and locks there. Most cores a campaign is resumed on are in exactly that state.
        let can_climb = k.is_none_or(|k| k.best_passed.is_none());
        // The search does not begin at 0 for a core that holds nothing yet: it begins at the start the family suggests.
        let from = if held >= 0 && can_climb {
            start.min(0)
        } else {
            held
        };
        slots += i64::from(tuner::worst_case_slots(
            from,
            floor,
            mode,
            i32::MAX,
            can_climb,
        ));
    }
    slots as f64 * minutes * legs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::CoreKnowledge;

    #[test]
    fn a_core_that_holds_nothing_yet_is_searched_from_the_suggested_start_and_not_from_zero() {
        let profiles = profiles();
        let cores = [0u32];
        let held: BTreeMap<u32, i32> = [(0, 0)].into_iter().collect();
        let run = |start: i32| {
            estimate_mode(
                &profiles,
                &cores,
                &held,
                &Knowledge::new(),
                -30,
                3,
                2,
                Mode::Coarse,
                start,
            )
            .total_minutes
        };
        let (from_zero, from_ten, from_fifteen) = (run(0), run(-10), run(-15));
        assert!(from_ten < from_zero, "{from_ten} < {from_zero}");
        assert!(from_fifteen < from_ten, "{from_fifteen} < {from_ten}");
        // A core that already holds a value is not moved to the start: it is searched from what it holds.
        let deep: BTreeMap<u32, i32> = [(0, -12)].into_iter().collect();
        let held_deep = |start: i32| {
            estimate_mode(
                &profiles,
                &cores,
                &deep,
                &Knowledge::new(),
                -30,
                3,
                2,
                Mode::Coarse,
                start,
            )
            .total_minutes
        };
        assert_eq!(held_deep(0), held_deep(-15));
    }

    fn profiles() -> BTreeMap<String, PhaseProfile> {
        [
            (
                "RecommendedCombo",
                PhaseProfile {
                    minutes_per_core: 6,
                    iterations: 3,
                    legs: 2,
                },
            ),
            (
                "HeavyFfts",
                PhaseProfile {
                    minutes_per_core: 10,
                    iterations: 1,
                    legs: 1,
                },
            ),
            (
                "Overnight",
                PhaseProfile {
                    minutes_per_core: 60,
                    iterations: 1,
                    legs: 1,
                },
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect()
    }

    #[test]
    fn phases_one_and_three_test_every_core_a_fixed_number_of_times() {
        let e = estimate(
            &profiles(),
            &[0, 1],
            &BTreeMap::new(),
            &Knowledge::new(),
            -30,
            3,
            1,
        );
        assert_eq!(e.per_phase[0], (1, 2.0 * 6.0 * 3.0 * 2.0));
        assert_eq!(e.per_phase[2], (3, 2.0 * 60.0));
    }

    #[test]
    fn a_core_already_at_its_target_costs_the_search_nothing() {
        let held: BTreeMap<u32, i32> = [(0, -27)].into();
        let e = estimate(&profiles(), &[0], &held, &Knowledge::new(), -30, 3, 2);
        assert_eq!(e.per_phase[0], (2, 0.0));
    }

    #[test]
    fn the_estimate_starts_at_the_phase_the_campaign_is_on() {
        let e = estimate(
            &profiles(),
            &[0],
            &BTreeMap::new(),
            &Knowledge::new(),
            -30,
            3,
            3,
        );
        assert_eq!(e.per_phase.len(), 1);
        assert_eq!(e.per_phase[0].0, 3);
    }

    #[test]
    fn a_proven_core_cannot_climb_so_it_costs_less() {
        let held: BTreeMap<u32, i32> = [(0, -10)].into();
        let fresh = estimate(&profiles(), &[0], &held, &Knowledge::new(), -30, 3, 2).total_minutes;
        let mut k = Knowledge::new();
        k.insert(
            0,
            CoreKnowledge {
                best_passed: Some(-10),
                worst_failed: None,
            },
        );
        let proven = estimate(&profiles(), &[0], &held, &k, -30, 3, 2).total_minutes;
        assert!(proven <= fresh);
    }
}
