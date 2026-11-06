//! The search for one core's value, as a loop that knows nothing about hardware (invariants I1, I11, I14).
//!
//! The steps themselves are `tuner::next_step`; this is the loop around them: set a value and read it back, test it,
//! remember what happened, ask for the next step, stop when the core locks. The processor, the test and the memory are
//! behind [`Rig`], so the whole walk can be run against a simulated core and checked: a value that already failed is
//! never offered again, a test that did not run changes nothing, a value the processor does not keep becomes the limit,
//! and a cancelled search ends without a guess.

use crate::depth;
use crate::knowledge::CoreKnowledge;
use crate::tuner::{self, CoreState, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResult {
    Pass,
    Fail,
    /// The test did not run properly; it says nothing about the value.
    Inconclusive,
    Cancelled,
}

/// What the search needs from the outside.
pub trait Rig {
    /// Writes a value and returns what the processor holds afterwards.
    fn set(&mut self, core: usize, margin: i32) -> Result<i32, String>;
    /// Tests the core at the value it holds now.
    fn test(&mut self, core: usize, margin: i32) -> TestResult;
    /// Puts the result into the memory (and says it). Called for every result except `Cancelled`.
    fn record(&mut self, core: usize, margin: i32, result: TestResult);
    fn knowledge(&self, core: usize) -> CoreKnowledge;
    fn say(&mut self, text: &str);
    fn cancelled(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct Params {
    /// What the family starts out allowed to try (`families::Family::start_depth`).
    pub model_limit: i32,
    /// What every core held before the search, as evidence of what the processor accepts.
    pub held_before: Vec<i32>,
    pub deepest: i32,
    pub shallowest: i32,
    pub mode: Mode,
    /// Headroom for this core, extra for the preferred ones included.
    pub guardband: i32,
    /// Where a first attempt should reach, and what the core holds now.
    pub suggested_start: i32,
    pub held_now: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Search {
    /// The core is locked at this value (not yet written: the caller writes it and checks it).
    Locked(i32),
    Unfinished(Why),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Cancelled,
    CannotSet,
    TestsDidNotRun,
    TooManySteps,
}

const MAX_STEPS: u32 = 60;
const MAX_INCONCLUSIVE: u32 = 3;

pub fn search_core(rig: &mut dyn Rig, core: usize, p: &Params) -> Search {
    let proven = rig.knowledge(core);
    let chip_limit = depth::effective(p.model_limit, None, &p.held_before, None);
    let floor = proven.floor_for(chip_limit).max(p.deepest);

    // Start from what the core is known to hold, else from where a first attempt should reach.
    let from = proven.best_passed.unwrap_or(if p.held_now < 0 {
        p.held_now
    } else {
        p.suggested_start
    });
    let mut margin = from.clamp(floor, p.shallowest.min(0));

    let mut state = CoreState {
        last_known_good: proven.best_passed,
        ascending: false,
    };
    let mut tests = 0u32;
    let mut inconclusive = 0u32;
    let mut applied_floor: Option<i32> = None;

    loop {
        if rig.cancelled() {
            return Search::Unfinished(Why::Cancelled);
        }
        let held = match rig.set(core, margin) {
            Ok(h) => h,
            Err(e) => {
                rig.say(&format!("core {core:2}: cannot set {margin}: {e}"));
                return Search::Unfinished(Why::CannotSet);
            }
        };
        if held != margin {
            // The board kept something else: that is as deep as it goes. What it keeps is what gets tested, and it
            // becomes the limit for the rest of the search.
            rig.say(&format!(
                "core {core:2}: asked for {margin}, the processor keeps {held}"
            ));
            if !depth::was_kept(margin, Some(held)) {
                applied_floor = Some(held);
                margin = held;
            }
        }

        let result = rig.test(core, margin);
        tests += 1;
        match result {
            TestResult::Cancelled => return Search::Unfinished(Why::Cancelled),
            TestResult::Inconclusive => {
                // A test that did not run says nothing about the value. Try again; after a few tries give up on this
                // core rather than move on a guess.
                rig.record(core, margin, result);
                inconclusive += 1;
                if inconclusive >= MAX_INCONCLUSIVE {
                    return Search::Unfinished(Why::TestsDidNotRun);
                }
                continue;
            }
            TestResult::Pass | TestResult::Fail => {}
        }
        let passed = result == TestResult::Pass;
        rig.record(core, margin, result);

        let chip = depth::effective(p.model_limit, None, &p.held_before, applied_floor);
        let max_limit = rig.knowledge(core).floor_for(chip).max(p.deepest);
        let step = tuner::next_step(margin, passed, p.mode, max_limit, state, p.guardband);
        state = step.state;
        if step.locked {
            return Search::Locked(step.next_margin);
        }
        margin = step.next_margin;
        if tests > MAX_STEPS {
            rig.say(&format!("core {core:2}: too many steps; stopping here"));
            return Search::Unfinished(Why::TooManySteps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{self, Knowledge};

    /// A simulated core: it holds up at any value at or above `threshold` and fails below it. The processor keeps at
    /// most `keeps_to` deep.
    struct Sim {
        threshold: i32,
        keeps_to: i32,
        knowledge: Knowledge,
        offered: Vec<i32>,
        tested: Vec<i32>,
        inconclusive_for: u32,
        cancel_after: Option<usize>,
        said: Vec<String>,
        held: i32,
    }

    impl Sim {
        fn new(threshold: i32) -> Sim {
            Sim {
                threshold,
                keeps_to: -50,
                knowledge: Knowledge::new(),
                offered: Vec::new(),
                tested: Vec::new(),
                inconclusive_for: 0,
                cancel_after: None,
                said: Vec::new(),
                held: 0,
            }
        }
    }

    impl Rig for Sim {
        fn set(&mut self, _core: usize, margin: i32) -> Result<i32, String> {
            self.offered.push(margin);
            self.held = margin.max(self.keeps_to);
            Ok(self.held)
        }
        fn test(&mut self, _core: usize, margin: i32) -> TestResult {
            if self.cancel_after.is_some_and(|n| self.tested.len() >= n) {
                return TestResult::Cancelled;
            }
            if self.inconclusive_for > 0 {
                self.inconclusive_for -= 1;
                return TestResult::Inconclusive;
            }
            self.tested.push(margin);
            if margin >= self.threshold {
                TestResult::Pass
            } else {
                TestResult::Fail
            }
        }
        fn record(&mut self, core: usize, margin: i32, result: TestResult) {
            match result {
                TestResult::Pass => {
                    knowledge::record_pass(&mut self.knowledge, core as u32, margin)
                }
                TestResult::Fail => {
                    knowledge::record_failure(&mut self.knowledge, core as u32, margin)
                }
                _ => {}
            }
        }
        fn knowledge(&self, core: usize) -> CoreKnowledge {
            self.knowledge
                .get(&(core as u32))
                .copied()
                .unwrap_or_default()
        }
        fn say(&mut self, text: &str) {
            self.said.push(text.to_string());
        }
        fn cancelled(&self) -> bool {
            false
        }
    }

    fn params(guardband: i32) -> Params {
        Params {
            model_limit: -30,
            held_before: vec![-10],
            deepest: -30,
            shallowest: 0,
            mode: Mode::Coarse,
            guardband,
            suggested_start: -15,
            held_now: -15,
        }
    }

    #[test]
    fn a_core_that_fails_after_passing_settles_back_without_testing_the_failure_again() {
        let mut sim = Sim::new(-26);
        let r = search_core(&mut sim, 0, &params(3));
        // -15 -18 -21 -24 hold, -27 fails; the core settles on what it proved, with headroom.
        assert_eq!(sim.tested, vec![-15, -18, -21, -24, -27]);
        assert_eq!(r, Search::Locked(-21));
        assert_eq!(sim.knowledge[&0].best_passed, Some(-24));
        assert_eq!(sim.knowledge[&0].worst_failed, Some(-27));
    }

    #[test]
    fn a_core_that_holds_to_the_limit_locks_above_the_limit() {
        let mut sim = Sim::new(-100);
        let r = search_core(&mut sim, 0, &params(3));
        assert_eq!(*sim.tested.last().unwrap(), -30);
        assert_eq!(r, Search::Locked(-27));
    }

    #[test]
    fn a_core_that_fails_at_once_climbs_and_locks_at_the_first_value_that_holds() {
        let mut sim = Sim::new(-5);
        let r = search_core(&mut sim, 0, &params(3));
        assert_eq!(sim.tested, vec![-15, -12, -9, -6, -3]);
        assert_eq!(r, Search::Locked(0));
        assert_eq!(sim.knowledge[&0].worst_failed, Some(-6));
    }

    #[test]
    fn a_value_that_failed_before_is_never_offered_again() {
        let mut sim = Sim::new(-26);
        sim.knowledge.insert(
            0,
            CoreKnowledge {
                best_passed: Some(-24),
                worst_failed: Some(-27),
            },
        );
        let r = search_core(&mut sim, 0, &params(3));
        assert!(sim.offered.iter().all(|m| *m > -27), "{:?}", sim.offered);
        assert!(matches!(r, Search::Locked(_)));
    }

    #[test]
    fn whatever_the_core_can_take_no_failed_value_is_ever_offered_twice_or_below_a_failure() {
        // Every threshold and both step sizes, from a few starting points. This is invariant I1 as a property.
        for threshold in -50..=0 {
            for mode in [Mode::Coarse, Mode::Fine] {
                for start in [-5, -15, -29] {
                    let mut sim = Sim::new(threshold);
                    let mut p = params(3);
                    p.mode = mode;
                    p.held_now = start;
                    let _ = search_core(&mut sim, 0, &p);
                    let mut worst: Option<i32> = None;
                    for m in &sim.tested {
                        if let Some(w) = worst {
                            assert!(
                                *m > w,
                                "threshold {threshold}: {m} offered after a failure at {w}: {:?}",
                                sim.tested
                            );
                        }
                        if *m < threshold {
                            worst = Some(worst.map_or(*m, |w| w.max(*m)));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_test_that_did_not_run_changes_nothing_and_three_in_a_row_give_up() {
        let mut sim = Sim::new(-26);
        sim.inconclusive_for = 3;
        let r = search_core(&mut sim, 0, &params(3));
        assert_eq!(r, Search::Unfinished(Why::TestsDidNotRun));
        assert!(sim.knowledge.is_empty());
        // Two that did not run, then it goes on from the same value.
        let mut sim = Sim::new(-26);
        sim.inconclusive_for = 2;
        let r = search_core(&mut sim, 0, &params(3));
        assert!(matches!(r, Search::Locked(_)));
        assert_eq!(sim.tested[0], -15);
    }

    #[test]
    fn a_cancelled_search_ends_without_a_guess() {
        let mut sim = Sim::new(-100);
        sim.cancel_after = Some(2);
        let r = search_core(&mut sim, 0, &params(3));
        assert_eq!(r, Search::Unfinished(Why::Cancelled));
    }

    #[test]
    fn a_depth_the_processor_does_not_keep_becomes_the_limit() {
        // The family starts out allowed -50, but this board stops at -40.
        let mut sim = Sim::new(-100);
        sim.keeps_to = -40;
        let mut p = params(3);
        p.model_limit = -50;
        p.deepest = -50;
        p.held_before = vec![-10];
        let r = search_core(&mut sim, 0, &p);
        let deepest_tested = *sim.tested.iter().min().unwrap();
        assert_eq!(deepest_tested, -40, "{:?}", sim.tested);
        assert_eq!(r, Search::Locked(-37));
        assert!(sim
            .said
            .iter()
            .any(|s| s.contains("the processor keeps -40")));
    }
}
