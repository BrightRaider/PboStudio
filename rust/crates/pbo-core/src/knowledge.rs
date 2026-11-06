//! What the tuner remembers about each core (invariant I1).
//!
//! The most aggressive value that held (`best_passed`) and the least aggressive value that failed
//! (`worst_failed`). The floor the search may go down to is just above the worst failure, never deeper than the
//! chip limit, so a value that already crashed a core is never offered again. Vectors:
//! `spec/vectors/core-knowledge.json`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoreKnowledge {
    pub best_passed: Option<i32>,
    pub worst_failed: Option<i32>,
}

impl CoreKnowledge {
    /// The deepest value the search may go to for this core: the chip limit, or just above a failure.
    pub fn floor_for(&self, chip_limit: i32) -> i32 {
        match self.worst_failed {
            Some(bad) => chip_limit.max(bad + 1),
            None => chip_limit,
        }
    }

    /// Whether `margin` is a value this core has failed at, or anything deeper.
    pub fn is_known_bad(&self, margin: i32) -> bool {
        self.worst_failed.is_some_and(|bad| margin <= bad)
    }
}

/// Knowledge per core, in core order.
pub type Knowledge = BTreeMap<u32, CoreKnowledge>;

/// Records that `core` held up at `margin`. Only a more aggressive success is news.
pub fn record_pass(knowledge: &mut Knowledge, core: u32, margin: i32) {
    let existing = knowledge.get(&core).copied().unwrap_or_default();
    if existing.best_passed.is_some_and(|best| best <= margin) {
        return;
    }
    knowledge.insert(
        core,
        CoreKnowledge {
            best_passed: Some(margin),
            ..existing
        },
    );
}

/// Records that `core` failed at `margin`. A less aggressive failure widens the bad region, so it always wins
/// over an older one, and a value that now fails cannot still count as the best pass.
pub fn record_failure(knowledge: &mut Knowledge, core: u32, margin: i32) {
    let existing = knowledge.get(&core).copied().unwrap_or_default();
    if existing.worst_failed.is_some_and(|bad| bad >= margin) {
        return;
    }
    let best_passed = existing.best_passed.filter(|p| *p > margin);
    knowledge.insert(
        core,
        CoreKnowledge {
            worst_failed: Some(margin),
            best_passed,
        },
    );
}

/// How many cores carry a value on record that failed.
pub fn cores_with_known_bad(knowledge: &Knowledge) -> usize {
    knowledge
        .values()
        .filter(|k| k.worst_failed.is_some())
        .count()
}
