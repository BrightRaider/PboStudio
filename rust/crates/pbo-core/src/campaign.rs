//! What to run next, and the three-phase campaign built on it (invariant I13).
//!
//! `recommend` has one correct answer at any moment: set up the driver, then an engine, then make the first
//! measuring run, then search the cores that still have room, then confirm. A core has room when it holds a value
//! shallower than where the search would lock it (its floor plus headroom); leaving the headroom out was a loop, in
//! which a core the search had just finished still counted as open. `decide` says whether the campaign runs the
//! next step, is done, has stalled (the same step over the same values twice) or needs setup. Vectors:
//! `spec/vectors/campaign.json`.

use std::collections::BTreeMap;

use crate::knowledge::{CoreKnowledge, Knowledge};
use crate::tuner::apply_guardband;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    InstallDriver,
    InstallEngine,
    Discover,
    Narrow,
    Confirm,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::InstallDriver => "InstallDriver",
            Stage::InstallEngine => "InstallEngine",
            Stage::Discover => "Discover",
            Stage::Narrow => "Narrow",
            Stage::Confirm => "Confirm",
        }
    }

    pub fn from_name(name: &str) -> Option<Stage> {
        [
            Stage::InstallDriver,
            Stage::InstallEngine,
            Stage::Discover,
            Stage::Narrow,
            Stage::Confirm,
        ]
        .into_iter()
        .find(|s| s.name() == name)
    }

    /// 1 to 3 for the three phases of a campaign, 0 for the setup stages.
    pub fn phase_number(self) -> i32 {
        match self {
            Stage::Discover => 1,
            Stage::Narrow => 2,
            Stage::Confirm => 3,
            _ => 0,
        }
    }
}

pub const TOTAL_PHASES: i32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    OpenSetup,
    ConfigureRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    HeavyFfts,
    RecommendedCombo,
    Overnight,
}

impl Profile {
    pub fn name(self) -> &'static str {
        match self {
            Profile::HeavyFfts => "HeavyFfts",
            Profile::RecommendedCombo => "RecommendedCombo",
            Profile::Overnight => "Overnight",
        }
    }
}

/// The name of the profile a phase runs (for display).
pub fn profile_name_for_phase(phase: i32) -> &'static str {
    crate::estimate::profile_for_phase(phase).name()
}

/// The single recommendation the interface leads with. An empty `cores` means all of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextStep {
    pub profile: Profile,
    pub stage: Stage,
    pub use_auto_tuner: bool,
    pub cores: Vec<u32>,
    pub action: Action,
    pub show_profile: bool,
}

impl NextStep {
    fn setup(stage: Stage) -> NextStep {
        NextStep {
            profile: Profile::HeavyFfts,
            stage,
            use_auto_tuner: false,
            cores: Vec::new(),
            action: Action::OpenSetup,
            show_profile: false,
        }
    }

    fn run(profile: Profile, stage: Stage, use_auto_tuner: bool, cores: Vec<u32>) -> NextStep {
        NextStep {
            profile,
            stage,
            use_auto_tuner,
            cores,
            action: Action::ConfigureRun,
            show_profile: true,
        }
    }
}

/// What to run now. `margins` holds what each core holds at this moment; a core without an entry holds zero.
pub fn recommend(
    cores: &[u32],
    margins: &BTreeMap<u32, i32>,
    knowledge: &Knowledge,
    chip_limit: i32,
    driver_ready: bool,
    engine_ready: bool,
    guardband: i32,
) -> NextStep {
    // Setup comes first, and the driver leads because it is the only step that also needs a restart.
    if !driver_ready {
        return NextStep::setup(Stage::InstallDriver);
    }
    if !engine_ready {
        return NextStep::setup(Stage::InstallEngine);
    }

    let open: Vec<u32> = cores
        .iter()
        .copied()
        .filter(|core| {
            let held = margins.get(core).copied().unwrap_or(0);
            let floor = knowledge
                .get(core)
                .map_or(chip_limit, |k| k.floor_for(chip_limit));
            // A core is open when it has room to go deeper, and also when it holds a value it is known to fail at:
            // the search works in both directions, so a BIOS value that is too aggressive is taken back up.
            let known_bad = knowledge.get(core).is_some_and(|k| k.is_known_bad(held));
            held > apply_guardband(floor, guardband) || known_bad
        })
        .collect();

    // Nothing is measured until a core has held somewhere, or has failed at the very value it holds now (a BIOS value
    // that is too aggressive is a measurement too, and the search then takes it back up).
    let nothing_known = knowledge.is_empty()
        || cores.iter().all(|c| {
            let held = margins.get(c).copied().unwrap_or(0);
            knowledge
                .get(c)
                .is_none_or(|k| k.best_passed.is_none() && !k.is_known_bad(held))
        });

    if nothing_known {
        // The first run is a plain validation of whatever is set, alternating both engines.
        NextStep::run(
            Profile::RecommendedCombo,
            Stage::Discover,
            false,
            Vec::new(),
        )
    } else if !open.is_empty() {
        // The search, on the open cores only, and one engine: a core locked in the first phase would never see
        // the second.
        NextStep::run(Profile::HeavyFfts, Stage::Narrow, true, open)
    } else {
        NextStep::run(Profile::Overnight, Stage::Confirm, false, Vec::new())
    }
}

/// The value a failed core is put back to: above the failure, with headroom, never positive.
pub fn back_off_to(knowledge: &CoreKnowledge, chip_limit: i32, guardband: i32) -> i32 {
    (knowledge.floor_for(chip_limit) + guardband.max(0)).min(0)
}

/// What a step looked like when it ran: its stage and the values of its cores. Two runs with the same signature
/// changed nothing between them.
pub fn signature(stage: Stage, cores: &[u32], margins: &BTreeMap<u32, i32>) -> String {
    let all: Vec<u32>;
    let cores = if cores.is_empty() {
        all = margins.keys().copied().collect();
        &all[..]
    } else {
        cores
    };
    let values: Vec<String> = cores
        .iter()
        .map(|c| format!("{c}:{}", margins.get(c).copied().unwrap_or(0)))
        .collect();
    format!("{}|{}", stage.name(), values.join(","))
}

/// Where the campaign stands, as far as deciding goes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub last_stage: Option<Stage>,
    pub runs_done: u32,
    pub last_signature: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    NeedsSetup,
    Run,
    Done,
    Stalled,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::NeedsSetup => "NeedsSetup",
            Verdict::Run => "Run",
            Verdict::Done => "Done",
            Verdict::Stalled => "Stalled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub verdict: Verdict,
    /// The phase to run, 1 to 3, when the verdict is `Run`; otherwise 0.
    pub phase_number: i32,
}

pub fn decide(
    state: &State,
    step: &NextStep,
    margins: &BTreeMap<u32, i32>,
    last_run_clean: bool,
) -> Decision {
    let plain = |verdict| Decision {
        verdict,
        phase_number: 0,
    };

    if step.action == Action::OpenSetup {
        return plain(Verdict::NeedsSetup);
    }

    // The confirmation run is the last thing a campaign does, and passing it is the point of the other two.
    if state.last_stage == Some(Stage::Confirm) && last_run_clean && state.runs_done > 0 {
        return plain(Verdict::Done);
    }

    // A step that would run over exactly the values the last one ran over has nothing new to find.
    if state.runs_done > 0 && signature(step.stage, &step.cores, margins) == state.last_signature {
        return plain(Verdict::Stalled);
    }

    Decision {
        verdict: Verdict::Run,
        phase_number: step.stage.phase_number(),
    }
}
