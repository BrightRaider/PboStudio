//! The plan: what is going to run (ux.md). One plan, three depths; Auto is a way of pre-filling it, not a mode.
//!
//! What is shown is what runs: the window renders this struct and the command line is built from it, so there is no
//! state that only one of them knows.

use std::collections::BTreeMap;

use pbo_core::engine::{FftPreset, Prime95Mode};
use pbo_core::knowledge::Knowledge;
use pbo_core::tuner::{self, Mode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Prime95,
    YCruncher,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Search {
    /// Test the cores at the values they hold and change nothing.
    Off,
    /// Steps of 3.
    Coarse,
    /// Steps of 1.
    Fine,
    /// Steps of the person's own choosing (2, 4, 5 or 6: 3 and 1 are `Coarse` and `Fine`).
    Step(u32),
}

impl Search {
    pub fn mode(self) -> Option<Mode> {
        match self {
            Search::Off => None,
            Search::Coarse => Some(Mode::Coarse),
            Search::Fine => Some(Mode::Fine),
            Search::Step(n) => Some(Mode::Steps(n as i32)),
        }
    }

    /// A search that steps `counts` at a time, in its one spelling: 3 is `Coarse`, 1 is `Fine`.
    pub fn stepping(counts: u32) -> Search {
        match counts.clamp(1, tuner::MAX_STEP as u32) {
            1 => Search::Fine,
            3 => Search::Coarse,
            n => Search::Step(n),
        }
    }

    /// How far one step goes, when there is a search.
    pub fn step_size(self) -> Option<u32> {
        self.mode().map(|m| tuner::step_size(m) as u32)
    }

    /// The name in plan files and on the wire.
    pub fn id(self) -> String {
        match self {
            Search::Off => "off".to_string(),
            Search::Coarse => "coarse".to_string(),
            Search::Fine => "fine".to_string(),
            Search::Step(n) => format!("step:{n}"),
        }
    }

    pub fn from_id(id: &str) -> Option<Search> {
        match id {
            "off" => Some(Search::Off),
            "coarse" => Some(Search::Coarse),
            "fine" => Some(Search::Fine),
            other => {
                let n: u32 = other.strip_prefix("step:")?.parse().ok()?;
                (1..=tuner::MAX_STEP as u32)
                    .contains(&n)
                    .then(|| Search::stepping(n))
            }
        }
    }
}

/// One further step after the first: another engine (or the same with other settings) on the same core, with the same
/// time. A core passes only when every step passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub engine: Engine,
    pub mode: Prime95Mode,
    pub fft: FftPreset,
}

impl Step {
    /// `engine:mode:fft`, for files and the command line.
    pub fn to_text(self) -> String {
        format!(
            "{}:{}:{}",
            match self.engine {
                Engine::Prime95 => "prime95",
                Engine::YCruncher => "ycruncher",
            },
            self.mode.name().to_ascii_lowercase(),
            self.fft.to_text()
        )
    }

    pub fn from_text(text: &str) -> Option<Step> {
        let mut parts = text.split(':');
        let engine = match parts.next()? {
            "prime95" => Engine::Prime95,
            "ycruncher" => Engine::YCruncher,
            _ => return None,
        };
        let mode = match parts.next().unwrap_or("avx2") {
            "sse" => Prime95Mode::Sse,
            "avx" => Prime95Mode::Avx,
            "avx2" => Prime95Mode::Avx2,
            "avx512" => Prime95Mode::Avx512,
            _ => return None,
        };
        let fft = FftPreset::from_name(parts.next().unwrap_or("Smallest"))?;
        parts.next().is_none().then_some(Step { engine, mode, fft })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The cores to test. Nothing chosen means all of them, in every phase.
    pub cores: Vec<usize>,
    pub engine: Engine,
    pub seconds_per_test: u32,
    pub search: Search,
    /// Steps of headroom kept between what held and where a core is locked.
    pub guardband: i32,
    /// Where the search of a core that holds nothing yet begins, in counts below zero (10, 15, 20 are offered); 0 is the
    /// start the processor family suggests.
    pub start: u32,
    /// Load both threads of a core.
    pub smt: bool,
    /// Prime95's instruction set and FFT range (y-cruncher ignores both).
    pub mode: Prime95Mode,
    pub fft: FftPreset,
    /// Further steps per core after the first, in order.
    pub then: Vec<Step>,
    /// How many times a test of the cores at their held values goes round all the chosen cores (a search has its own
    /// repetition and ignores this).
    pub rounds: u32,
    /// The order the cores are tested in.
    pub order: pbo_core::order::CoreOrder,
    /// How the load is interrupted so the core has to boost back up.
    pub transient: pbo_core::transient::Transient,
    /// What a test of the held values does when a core fails.
    pub on_error: pbo_core::onerror::OnError,
    /// A sound at an error (in a test of the held values) and at the end of a run.
    pub notify: bool,
    /// Which algorithms y-cruncher runs.
    pub yc_set: pbo_core::engine::YcSet,
}

impl Plan {
    /// The recommendation. Short tests and a coarse search: the point is a first value most cores clear.
    pub fn recommended() -> Plan {
        Plan {
            cores: Vec::new(),
            engine: Engine::Prime95,
            seconds_per_test: 60,
            search: Search::Coarse,
            guardband: tuner::DEFAULT_GUARDBAND,
            start: 0,
            smt: false,
            mode: Prime95Mode::Avx2,
            fft: FftPreset::Smallest,
            then: Vec::new(),
            rounds: 1,
            order: pbo_core::order::CoreOrder::default(),
            transient: pbo_core::transient::Transient::default(),
            on_error: pbo_core::onerror::OnError::default(),
            notify: true,
            yc_set: pbo_core::engine::YcSet::default(),
        }
    }

    /// One step of Auto's campaign as a plan: the step's test list, on the cores it names (all chosen ones when it names
    /// none), the time per test of the intensity, one round, and the search of `base` only where the step searches. The
    /// window and `pbo auto` both build their steps here, so they run the same thing. `None` when the step names no test
    /// list that exists.
    pub fn campaign_step(
        step: &pbo_core::campaign::NextStep,
        base: &Plan,
        intensity: pbo_core::auto::Intensity,
    ) -> Option<Plan> {
        let profile = pbo_core::profiles::by_id(step.profile.name())?;
        let mut plan = Plan::from_profile(&profile, base);
        plan.seconds_per_test = intensity.minutes(step.stage.phase_number().max(1)) * 60;
        plan.rounds = 1;
        if !step.cores.is_empty() {
            plan.cores = step.cores.iter().map(|c| *c as usize).collect();
        }
        plan.search = if step.use_auto_tuner {
            base.search
        } else {
            Search::Off
        };
        Some(plan)
    }

    /// A plan from one of the ready-made test lists: its steps, its time per core and its threads. A test list tests at the
    /// values the cores hold, so the search is off; the cores the person chose are kept.
    pub fn from_profile(profile: &pbo_core::profiles::Profile, keep: &Plan) -> Plan {
        use pbo_core::profiles::Engine as E;
        let step = |l: &pbo_core::profiles::Leg| Step {
            engine: match l.engine {
                E::Prime95 => Engine::Prime95,
                E::YCruncher => Engine::YCruncher,
            },
            mode: l.mode,
            fft: l.fft,
        };
        let first = profile.legs.first().map(step);
        Plan {
            cores: keep.cores.clone(),
            engine: first.map_or(Engine::Prime95, |s| s.engine),
            seconds_per_test: profile.minutes_per_core * 60,
            search: Search::Off,
            guardband: keep.guardband,
            start: keep.start,
            smt: profile.threads > 1,
            mode: first.map_or(Prime95Mode::Avx2, |s| s.mode),
            fft: first.map_or(FftPreset::Smallest, |s| s.fft),
            then: profile.legs.iter().skip(1).map(step).collect(),
            rounds: profile.iterations.max(1),
            order: profile.order.into(),
            transient: profile.transient_mode(),
            on_error: keep.on_error,
            notify: keep.notify,
            yc_set: keep.yc_set,
        }
    }

    /// The cores the plan touches, out of `total`.
    pub fn cores_or_all(&self, total: usize) -> Vec<usize> {
        if self.cores.is_empty() {
            (0..total).collect()
        } else {
            let mut c: Vec<usize> = self.cores.iter().copied().filter(|c| *c < total).collect();
            c.sort_unstable();
            c.dedup();
            c
        }
    }

    /// How many parts of the plan differ from the recommendation (the "adjusted" count of depth 2).
    pub fn adjusted_parts(&self) -> usize {
        let r = Plan::recommended();
        [
            self.cores != r.cores,
            self.engine != r.engine,
            self.seconds_per_test != r.seconds_per_test,
            self.search != r.search,
            self.guardband != r.guardband,
            self.start != r.start,
            self.smt != r.smt,
            self.mode != r.mode,
            self.fft != r.fft,
            self.then != r.then,
            self.rounds != r.rounds,
            self.order != r.order,
            self.transient != r.transient,
            self.on_error != r.on_error,
            self.notify != r.notify,
            self.yc_set != r.yc_set,
        ]
        .iter()
        .filter(|d| **d)
        .count()
    }

    /// The command line that does exactly this plan.
    pub fn command_line(&self) -> String {
        let mut parts = vec![match self.search {
            Search::Off => "pbo run".to_string(),
            _ => "pbo tune".to_string(),
        }];
        parts.push(if self.seconds_per_test == 0 {
            "--seconds auto".to_string()
        } else {
            format!("--seconds {}", self.seconds_per_test)
        });
        if !self.cores.is_empty() {
            let list: Vec<String> = self.cores.iter().map(|c| c.to_string()).collect();
            parts.push(format!("--cores {}", list.join(",")));
        }
        parts.push(format!(
            "--engine {}",
            match self.engine {
                Engine::Prime95 => "prime95",
                Engine::YCruncher => "ycruncher",
            }
        ));
        if self.smt {
            parts.push("--smt".into());
        }
        if self.mode != Plan::recommended().mode {
            parts.push(format!("--mode {}", self.mode.name().to_ascii_lowercase()));
        }
        if self.fft != Plan::recommended().fft {
            parts.push(format!("--fft {}", self.fft.to_text()));
        }
        if !self.then.is_empty() {
            let list: Vec<String> = self.then.iter().map(|s| s.to_text()).collect();
            parts.push(format!("--then {}", list.join("+")));
        }
        if self.rounds != 1 {
            parts.push(format!("--rounds {}", self.rounds));
        }
        if self.order != pbo_core::order::CoreOrder::default() {
            parts.push(format!("--order {}", self.order.id()));
        }
        if self.transient != pbo_core::transient::Transient::default() {
            parts.push(format!("--transient {}", self.transient.to_text()));
        }
        if self.on_error != pbo_core::onerror::OnError::default() {
            parts.push(format!("--on-error {}", self.on_error.id()));
        }
        if !self.notify {
            parts.push("--quiet".into());
        }
        if self.yc_set != pbo_core::engine::YcSet::default() {
            parts.push(format!("--yc-set {}", self.yc_set.id()));
        }
        match self.search {
            Search::Fine => parts.push("--fine".into()),
            Search::Step(n) => parts.push(format!("--step {n}")),
            _ => {}
        }
        if self.search != Search::Off && self.guardband != tuner::DEFAULT_GUARDBAND {
            parts.push(format!("--guardband {}", self.guardband));
        }
        if self.search != Search::Off && self.start > 0 {
            parts.push(format!("--start {}", self.start));
        }
        parts.join(" ")
    }

    /// The longest the plan can take, in seconds: every test of every core, for the worst case of the search. A
    /// number that is calculated, never guessed.
    pub fn worst_case_seconds(
        &self,
        total_cores: usize,
        held: &BTreeMap<u32, i32>,
        knowledge: &Knowledge,
        chip_limit: i32,
    ) -> u64 {
        // Every step of a test runs for the full time, and a test of the held values repeats for every round.
        let legs = 1 + self.then.len() as u64;
        let rounds = if self.search == Search::Off {
            u64::from(self.rounds.max(1))
        } else {
            1
        };
        // "Auto" runs until the work list has been through once, and at most this long.
        let seconds = if self.seconds_per_test == 0 {
            pbo_core::engine::AUTO_RUNTIME_CAP_SECONDS
        } else {
            u64::from(self.seconds_per_test)
        };
        let per_test = seconds * legs * rounds;
        let mut slots: u64 = 0;
        let chosen = self.cores_or_all(total_cores);
        // Handing the load from every core to every other visits each core many times; only the first visit searches.
        let visits = if self.order == pbo_core::order::CoreOrder::Pairs && chosen.len() > 1 {
            (chosen.len() * (chosen.len() - 1) * 2) as u64
        } else {
            chosen.len() as u64
        };
        let extra_visits = visits - chosen.len() as u64;
        for core in chosen {
            slots += match self.search.mode() {
                None => 1,
                Some(mode) => {
                    let k = knowledge.get(&(core as u32)).copied().unwrap_or_default();
                    let floor = k.floor_for(chip_limit);
                    let margin = held.get(&(core as u32)).copied().unwrap_or(0);
                    tuner::worst_case_slots(margin, floor, mode, i32::MAX, k.best_passed.is_none())
                        .max(1) as u64
                }
            };
        }
        (slots + extra_visits) * per_test
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pbo_core::knowledge::CoreKnowledge;

    #[test]
    fn a_test_of_the_held_values_costs_every_step_for_every_round() {
        let step = Step {
            engine: Engine::YCruncher,
            mode: Prime95Mode::Avx2,
            fft: FftPreset::Smallest,
        };
        let plan = Plan {
            cores: vec![0, 1],
            search: Search::Off,
            seconds_per_test: 100,
            then: vec![step],
            rounds: 3,
            ..Plan::recommended()
        };
        // Two cores, two steps each, three rounds, 100 s a step.
        let secs = plan.worst_case_seconds(8, &BTreeMap::new(), &Knowledge::new(), -30);
        assert_eq!(secs, 2 * 2 * 3 * 100);
    }

    #[test]
    fn a_search_repeats_by_its_own_steps_and_ignores_rounds() {
        let plan = Plan {
            cores: vec![0],
            search: Search::Coarse,
            seconds_per_test: 60,
            rounds: 5,
            ..Plan::recommended()
        };
        let again = Plan {
            rounds: 1,
            ..plan.clone()
        };
        let held: BTreeMap<u32, i32> = [(0, -9)].into();
        assert_eq!(
            plan.worst_case_seconds(8, &held, &Knowledge::new(), -30),
            again.worst_case_seconds(8, &held, &Knowledge::new(), -30)
        );
    }

    #[test]
    fn the_rest_of_the_options_survive_the_command_line_and_the_plan_file() {
        use pbo_core::engine::{FftPreset, YcSet};
        use pbo_core::onerror::OnError;
        let plan = Plan {
            seconds_per_test: 0,
            fft: FftPreset::Custom(36, 248),
            on_error: OnError::Stop,
            notify: false,
            yc_set: YcSet::FastDiscovery,
            order: pbo_core::order::CoreOrder::Pairs,
            ..Plan::recommended()
        };
        let line = plan.command_line();
        for part in [
            "--seconds auto",
            "--fft 36-248",
            "--on-error stop",
            "--quiet",
            "--yc-set fast-discovery",
            "--order pairs",
        ] {
            assert!(line.contains(part), "{part} missing in {line}");
        }
        let back = Plan::from_toml(&plan.to_toml()).unwrap();
        assert_eq!(back, plan);
        // The defaults say nothing.
        let quiet = Plan::recommended().command_line();
        for part in ["--on-error", "--quiet", "--yc-set", "--fft"] {
            assert!(!quiet.contains(part), "{part} in {quiet}");
        }
        let mut p = Plan::recommended();
        assert!(p.set("seconds", "auto").is_ok());
        assert_eq!(p.seconds_per_test, 0);
        assert!(p.set("seconds", "5").is_err());
        assert!(p.set("fft", "248-36").is_err());
        assert!(p.set("notify", "maybe").is_err());
        assert!(p.set("on_error", "ignore").is_err());
        assert!(p.set("ycruncher", "everything").is_err());
    }

    #[test]
    fn a_step_can_have_a_range_of_its_own() {
        let step = Step::from_text("prime95:avx2:100-2000").unwrap();
        assert_eq!(step.fft, pbo_core::engine::FftPreset::Custom(100, 2000));
        assert_eq!(step.to_text(), "prime95:avx2:100-2000");
    }

    #[test]
    fn auto_counts_at_its_cap_and_pairs_visit_every_core_many_times() {
        let auto = Plan {
            cores: vec![0, 1],
            search: Search::Off,
            seconds_per_test: 0,
            ..Plan::recommended()
        };
        assert_eq!(
            auto.worst_case_seconds(8, &BTreeMap::new(), &Knowledge::new(), -30),
            2 * pbo_core::engine::AUTO_RUNTIME_CAP_SECONDS
        );
        let pairs = Plan {
            cores: vec![0, 1, 2],
            search: Search::Off,
            seconds_per_test: 100,
            order: pbo_core::order::CoreOrder::Pairs,
            ..Plan::recommended()
        };
        // Three cores: 3 * 2 pairs, two cores each.
        assert_eq!(
            pairs.worst_case_seconds(8, &BTreeMap::new(), &Knowledge::new(), -30),
            12 * 100
        );
    }

    #[test]
    fn order_and_load_changes_survive_the_command_line_and_the_plan_file() {
        use pbo_core::order::CoreOrder;
        use pbo_core::transient::Transient;
        for (order, transient) in [
            (CoreOrder::Alternate, Transient::Bursts),
            (CoreOrder::Random, Transient::Off),
            (
                CoreOrder::Sequential,
                Transient::Periodic {
                    every_s: 5,
                    pause_s: 1,
                },
            ),
        ] {
            let plan = Plan {
                order,
                transient,
                ..Plan::recommended()
            };
            let line = plan.command_line();
            assert_eq!(
                line.contains("--order"),
                order != CoreOrder::default(),
                "{line}"
            );
            assert_eq!(
                line.contains("--transient"),
                transient != Transient::default(),
                "{line}"
            );
            let back = Plan::from_toml(&plan.to_toml()).unwrap();
            assert_eq!((back.order, back.transient), (order, transient));
        }
        let mut p = Plan::recommended();
        assert!(p.set("order", "sideways").is_err());
        assert!(p.set("transient", "sometimes").is_err());
        assert_eq!(p.order, CoreOrder::default());
    }

    #[test]
    fn a_ready_made_list_brings_its_order_and_how_it_interrupts_the_load() {
        use pbo_core::order::CoreOrder;
        use pbo_core::transient::Transient;
        let all = pbo_core::profiles::all();
        let keep = Plan::recommended();
        let breaking =
            Plan::from_profile(all.iter().find(|p| p.id == "BreakingPoint").unwrap(), &keep);
        assert_eq!(breaking.order, CoreOrder::Sequential);
        assert_eq!(
            breaking.transient,
            Transient::Periodic {
                every_s: 5,
                pause_s: 1
            }
        );
        let micro = Plan::from_profile(all.iter().find(|p| p.id == "MicroBurst").unwrap(), &keep);
        assert_eq!(micro.transient, Transient::Bursts);
        let combo = Plan::from_profile(
            all.iter().find(|p| p.id == "RecommendedCombo").unwrap(),
            &keep,
        );
        assert_eq!(combo.order, CoreOrder::Alternate);
    }

    #[test]
    fn rounds_survive_the_command_line_and_the_plan_file() {
        let plan = Plan {
            rounds: 4,
            ..Plan::recommended()
        };
        assert!(plan.command_line().contains("--rounds 4"));
        assert_eq!(Plan::from_toml(&plan.to_toml()).unwrap().rounds, 4);
        assert!(!Plan::recommended().command_line().contains("--rounds"));
        let mut p = Plan::recommended();
        assert!(p.set("rounds", "0").is_err());
        assert!(p.set("rounds", "51").is_err());
        assert_eq!(p.rounds, 1);
    }

    #[test]
    fn nothing_chosen_means_all_cores() {
        assert_eq!(Plan::recommended().cores_or_all(4), vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_choice_is_kept_and_cleaned() {
        let p = Plan {
            cores: vec![3, 1, 1, 9],
            ..Plan::recommended()
        };
        assert_eq!(p.cores_or_all(4), vec![1, 3]);
    }

    #[test]
    fn the_recommendation_is_not_adjusted() {
        assert_eq!(Plan::recommended().adjusted_parts(), 0);
        let p = Plan {
            seconds_per_test: 120,
            smt: true,
            ..Plan::recommended()
        };
        assert_eq!(p.adjusted_parts(), 2);
    }

    #[test]
    fn the_command_line_says_what_the_plan_says() {
        assert_eq!(
            Plan::recommended().command_line(),
            "pbo tune --seconds 60 --engine prime95"
        );
        let p = Plan {
            cores: vec![2, 5],
            engine: Engine::YCruncher,
            search: Search::Off,
            seconds_per_test: 30,
            smt: true,
            ..Plan::recommended()
        };
        assert_eq!(
            p.command_line(),
            "pbo run --seconds 30 --cores 2,5 --engine ycruncher --smt"
        );
    }

    #[test]
    fn a_step_of_ones_own_is_written_and_read_back_and_three_and_one_keep_their_names() {
        assert_eq!(Search::stepping(3), Search::Coarse);
        assert_eq!(Search::stepping(1), Search::Fine);
        assert_eq!(Search::stepping(2), Search::Step(2));
        assert_eq!(Search::stepping(0), Search::Fine);
        assert_eq!(Search::stepping(99), Search::Step(6));
        for s in [
            Search::Off,
            Search::Coarse,
            Search::Fine,
            Search::Step(2),
            Search::Step(4),
            Search::Step(6),
        ] {
            assert_eq!(Search::from_id(&s.id()), Some(s));
        }
        assert_eq!(Search::from_id("step:3"), Some(Search::Coarse));
        assert_eq!(Search::from_id("step:7"), None);
        assert_eq!(Search::from_id("step:x"), None);
        assert_eq!(Search::from_id("sideways"), None);
        assert_eq!(Search::Step(5).step_size(), Some(5));
        assert_eq!(Search::Off.step_size(), None);
        let plan = Plan {
            search: Search::Step(2),
            ..Plan::recommended()
        };
        assert!(plan.command_line().contains("--step 2"));
        assert_eq!(
            Plan::from_toml(&plan.to_toml()).unwrap().search,
            Search::Step(2)
        );
    }

    #[test]
    fn the_start_of_the_search_is_written_read_back_and_only_named_when_it_is_not_the_familys() {
        let plain = Plan::recommended();
        assert_eq!(plain.start, 0);
        assert!(!plain.command_line().contains("--start"));
        let plan = Plan {
            start: 20,
            ..Plan::recommended()
        };
        assert!(plan.command_line().contains("--start 20"));
        assert_eq!(Plan::from_toml(&plan.to_toml()).unwrap().start, 20);
        assert_ne!(plan, plain);
        assert!(Plan::from_toml(
            "start = 40
"
        )
        .is_err());
    }

    #[test]
    fn without_a_search_each_core_is_one_test() {
        let p = Plan {
            search: Search::Off,
            ..Plan::recommended()
        };
        assert_eq!(
            p.worst_case_seconds(8, &BTreeMap::new(), &Knowledge::new(), -30),
            8 * 60
        );
    }

    #[test]
    fn a_search_costs_more_than_a_test_and_a_proven_core_less() {
        let p = Plan::recommended();
        let held: BTreeMap<u32, i32> = [(0, -10)].into();
        let fresh = p.worst_case_seconds(1, &held, &Knowledge::new(), -30);
        let mut k = Knowledge::new();
        k.insert(
            0,
            CoreKnowledge {
                best_passed: Some(-10),
                worst_failed: Some(-31),
            },
        );
        let proven = p.worst_case_seconds(1, &held, &k, -30);
        assert!(fresh > 60);
        assert!(proven <= fresh);
    }
    #[test]
    fn a_campaign_step_is_its_test_list_with_the_time_of_the_intensity_and_the_search_only_where_it_searches(
    ) {
        use pbo_core::auto::Intensity;
        use pbo_core::campaign::{Action, NextStep, Profile, Stage};
        let mut base = Plan::recommended();
        base.cores = vec![0, 1, 4];
        base.search = Search::stepping(5);
        let narrow = NextStep {
            profile: Profile::HeavyFfts,
            stage: Stage::Narrow,
            use_auto_tuner: true,
            cores: vec![4],
            action: Action::ConfigureRun,
            show_profile: false,
        };
        let plan = Plan::campaign_step(&narrow, &base, Intensity::Fast).unwrap();
        assert_eq!(plan.cores, vec![4]);
        assert_eq!(plan.search, Search::stepping(5));
        assert_eq!(plan.rounds, 1);
        assert_eq!(plan.seconds_per_test, Intensity::Fast.minutes(2) * 60);
        let discover = NextStep {
            profile: Profile::RecommendedCombo,
            stage: Stage::Discover,
            use_auto_tuner: false,
            cores: Vec::new(),
            ..narrow
        };
        let plan = Plan::campaign_step(&discover, &base, Intensity::Fast).unwrap();
        assert_eq!(plan.cores, vec![0, 1, 4]);
        assert_eq!(plan.search, Search::Off);
        assert_eq!(plan.seconds_per_test, Intensity::Fast.minutes(1) * 60);
    }
}
