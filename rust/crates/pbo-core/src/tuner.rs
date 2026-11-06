//! The step logic of the per-core search (invariants I1 and I11).
//!
//! A pass moves one step deeper until the limit, where the core locks. A failure with a proven value settles back
//! onto it without testing it again. A failure with nothing proven climbs towards zero, and the first value that
//! holds on the way back up is where the core locks. The guardband is headroom kept between what a core survived
//! and where it is locked: the search measures the boundary under a stress test, and everyday use is not that
//! load. It never pushes into a positive offset. The advice texts are language and live elsewhere
//! ([ADR-0011]). Vectors: `spec/vectors/auto-tuner.json`.

/// How far one pass steps: coarse for fast discovery of a core's limit, fine for precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Coarse,
    Fine,
    /// A step of the person's own choosing, in counts (1 to `MAX_STEP`).
    Steps(i32),
}

/// The largest step the person can ask for.
pub const MAX_STEP: i32 = 6;

pub const DEFAULT_GUARDBAND: i32 = 3;

/// Extra headroom for the cores the CPPC ranking puts highest: they boost furthest and Windows puts its
/// background work on them.
pub const PREFERRED_CORE_EXTRA_GUARDBAND: i32 = 2;

pub fn step_size(mode: Mode) -> i32 {
    match mode {
        Mode::Coarse => 3,
        Mode::Fine => 1,
        Mode::Steps(n) => n.clamp(1, MAX_STEP),
    }
}

/// What the search has learned about one core so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoreState {
    /// The most aggressive value that held.
    pub last_known_good: Option<i32>,
    /// Climbing back up after a failure.
    pub ascending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// What to try or apply next. When `locked`, the value the core is left at.
    pub next_margin: i32,
    pub locked: bool,
    /// Whether the core held up at the value that was just tested.
    pub passed: bool,
    pub state: CoreState,
}

/// Where a core is locked, given what it actually survived. Never positive; a negative guardband counts as zero.
pub fn apply_guardband(last_known_good: i32, guardband: i32) -> i32 {
    (last_known_good + guardband.max(0)).min(0)
}

/// The next move after testing `margin`.
pub fn next_step(
    margin: i32,
    passed: bool,
    mode: Mode,
    max_limit: i32,
    state: CoreState,
    guardband: i32,
) -> Step {
    let step = step_size(mode);
    if passed {
        on_passed(margin, max_limit, state, step, guardband)
    } else {
        on_failed(margin, state, step, guardband)
    }
}

fn on_passed(margin: i32, max_limit: i32, state: CoreState, step: i32, guardband: i32) -> Step {
    let learned = CoreState {
        last_known_good: Some(margin),
        ..state
    };

    // Coming back up after a failure: this is the first value that holds, and everything below it is known bad.
    if state.ascending {
        return Step {
            next_margin: apply_guardband(margin, guardband),
            locked: true,
            passed: true,
            state: learned,
        };
    }

    if margin <= max_limit {
        return Step {
            next_margin: apply_guardband(margin, guardband),
            locked: true,
            passed: true,
            state: learned,
        };
    }

    Step {
        next_margin: (margin - step).max(max_limit),
        locked: false,
        passed: true,
        state: learned,
    }
}

fn on_failed(margin: i32, state: CoreState, step: i32, guardband: i32) -> Step {
    // A value this core already survived is proven, so there is no reason to test it again.
    if let Some(good) = state.last_known_good {
        return Step {
            next_margin: apply_guardband(good, guardband),
            locked: true,
            passed: false,
            state,
        };
    }

    let ascending = CoreState {
        ascending: true,
        ..state
    };

    // Nothing has held yet. Failing without an undervolt means the cause is not the curve.
    if margin >= 0 {
        return Step {
            next_margin: 0,
            locked: true,
            passed: false,
            state: ascending,
        };
    }

    Step {
        next_margin: (margin + step).min(0),
        locked: false,
        passed: false,
        state: ascending,
    }
}

/// Worst-case number of test slots one core can consume, for the runtime estimate. The two directions are
/// alternatives, not a sum: a core that descends only ever climbs back to a value it already passed, and a core
/// climbs from the start only if its very first test fails. `can_climb` is false once the core has a value on
/// record that held, because a failure then returns straight to it.
pub fn worst_case_slots(
    margin: i32,
    max_limit: i32,
    mode: Mode,
    max_passes: i32,
    can_climb: bool,
) -> i32 {
    let step = step_size(mode);

    let descent = if margin > max_limit {
        div_ceil(margin - max_limit, step) + 1
    } else {
        1
    };
    let ascent = if can_climb {
        div_ceil((-margin).max(0), step) + 1
    } else {
        0
    };

    1.max(max_passes.min(descent.max(ascent)))
}

/// Integer division rounded up, for non-negative numerators.
fn div_ceil(numerator: i32, denominator: i32) -> i32 {
    (numerator + denominator - 1) / denominator
}
