//! `pbo auto`: the campaign of the window's Auto, on the command line, and what carries it on after a crash.
//!
//! The window leads the person through three phases (check what the values hold, search the cores that have room, prove
//! the result under every load) and asks the campaign for the next step each time. This does the same without a
//! window: it asks `campaign::recommend` what to run, runs it with `run` or `tune`, and asks again, until the campaign
//! says it is done, has stalled or needs the driver or a test program. It writes down how it was started, so that
//! `pbo resume --continue` can take it up again after the machine has crashed and come back: after a countdown the
//! person can cancel, and never more than a few times in a row, so that a core that keeps taking the machine down
//! cannot cause an endless loop.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pbo_app::plan::{Engine, Plan, Search};
use pbo_core::auto::Intensity;
use pbo_core::campaign::{self, Verdict};
use pbo_core::profiles;
use pbo_core::tuner;
use pbo_os as os;

use crate::commands::{self, Options};
use crate::store::Store;
use crate::stress::EngineKind;

/// How many phases one `pbo auto` runs at most: the campaign has three, and a step that changes nothing ends it earlier.
const MAX_RUNS: u32 = 12;
/// How many automatic continuations in a row after crashes before it stops and waits for the person.
pub const MAX_RESUMES_IN_A_ROW: u32 = 3;
/// After this long without a crash the count of continuations starts again.
const RESUME_WINDOW_SECONDS: i64 = 12 * 3600;
/// The default countdown before a continuation, in seconds.
pub const DEFAULT_COUNTDOWN_SECONDS: u64 = 60;

fn file(store: &Store, name: &str) -> PathBuf {
    store.root.join(name)
}

/// `pbo auto [--intensity fast|normal|thorough] [options of run and tune]`.
pub fn auto(args: &[String]) -> i32 {
    let mut intensity = Intensity::Fast;
    let mut rest: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--intensity" {
            match it.next().and_then(|v| Intensity::from_id(v)) {
                Some(i) => intensity = i,
                None => {
                    eprintln!("--intensity takes fast, normal or thorough");
                    return 2;
                }
            }
        } else {
            rest.push(a.clone());
        }
    }
    let o = match commands::parse(&rest) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let store = Store::beside_program();
    // How it was started, for the continuation after a crash.
    let _ = std::fs::write(file(&store, "auto_last.txt"), args.join("\n"));
    let named_step = rest.iter().any(|a| a == "--fine" || a == "--step");
    let code = drive(&o, intensity, named_step);
    // The file that says how it was started is there only while it runs: a crash leaves it behind, and that is what lets
    // `pbo resume --continue` tell a run that was cut off from one that ended. The same goes for the count of
    // continuations in a row: a run that ends by itself, however it ends, was not taken down.
    let _ = std::fs::remove_file(file(&store, "auto_resume.txt"));
    let _ = std::fs::remove_file(file(&store, "auto_last.txt"));
    code
}

/// The plan every step of the campaign is made from: the person's options, and the steps of the intensity unless they
/// named their own.
fn base_plan(o: &Options, intensity: Intensity, named_step: bool) -> Plan {
    let mut plan = Plan::recommended();
    plan.cores = o.cores.clone().unwrap_or_default();
    plan.engine = match o.engine {
        EngineKind::Prime95 => Engine::Prime95,
        EngineKind::YCruncher => Engine::YCruncher,
    };
    plan.smt = o.smt;
    plan.guardband = o.guardband;
    plan.start = o.start.unwrap_or(0);
    plan.transient = o.transient;
    plan.on_error = o.on_error;
    plan.order = o.order;
    plan.notify = o.notify;
    plan.yc_set = o.yc_set;
    let mode = if named_step {
        o.mode
    } else {
        intensity.preselected_mode()
    };
    plan.search = Search::stepping(tuner::step_size(mode) as u32);
    plan
}

fn drive(o: &Options, intensity: Intensity, named_step: bool) -> i32 {
    let base = base_plan(o, intensity, named_step);
    let mut state = campaign::State::default();
    let mut last_clean = true;
    println!(
        "Auto, {} (steps of {}, safety {}).",
        intensity.id(),
        base.search.step_size().unwrap_or(3),
        base.guardband
    );
    for _ in 0..MAX_RUNS {
        if os::cancelled() {
            println!("Auto cancelled.");
            return 130;
        }
        // What the campaign looks at: what each core holds, what is on record, and whether anything is missing.
        let (cores, held, knowledge, chip_limit) = {
            let machine = match commands::open_machine() {
                Ok(m) => m,
                Err(c) => return c,
            };
            let selected = match commands::chosen_cores(&machine, &o.cores) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{e}");
                    return 2;
                }
            };
            let held_now = machine.read_margins().unwrap_or_default();
            let held: BTreeMap<u32, i32> = held_now
                .iter()
                .enumerate()
                .map(|(c, v)| (c as u32, *v))
                .collect();
            let store = Store::beside_program();
            let (knowledge, _) = store.load_knowledge();
            let chip_limit =
                pbo_core::depth::effective(machine.family.start_depth, None, &held_now, None);
            (
                selected.into_iter().map(|c| c as u32).collect::<Vec<u32>>(),
                held,
                knowledge,
                chip_limit,
            )
        };
        let step = campaign::recommend(
            &cores,
            &held,
            &knowledge,
            chip_limit,
            true,
            crate::stress::find_prime95().is_some(),
            o.guardband,
        );
        let decision = campaign::decide(&state, &step, &held, last_clean);
        match decision.verdict {
            Verdict::NeedsSetup => {
                eprintln!(
                    "Auto needs the driver and a test program first: `pbo setup pawnio`, `pbo setup all`."
                );
                return 1;
            }
            Verdict::Done => {
                println!("Auto is done: every core has held under every load.");
                return 0;
            }
            Verdict::Stalled => {
                println!(
                    "Auto stopped: the next step would run over the same values as the last one, so it would find nothing new."
                );
                return 0;
            }
            Verdict::Run => {}
        }
        let phase = step.stage.phase_number().max(1);
        let Some(profile) = profiles::by_id(step.profile.name()) else {
            eprintln!("no test list '{}'", step.profile.name());
            return 1;
        };
        let mut plan = Plan::from_profile(&profile, &base);
        plan.seconds_per_test = intensity.minutes(phase) * 60;
        plan.rounds = 1;
        if !step.cores.is_empty() {
            plan.cores = step.cores.iter().map(|c| *c as usize).collect();
        }
        plan.search = if step.use_auto_tuner {
            base.search
        } else {
            Search::Off
        };
        println!(
            "Auto, phase {phase} of {}: {} on cores {:?}.",
            campaign::TOTAL_PHASES,
            step.profile.name(),
            if plan.cores.is_empty() {
                cores.iter().map(|c| *c as usize).collect()
            } else {
                plan.cores.clone()
            }
        );
        let mut options = Options::from_plan(&plan);
        options.yc_tests = o.yc_tests.clone();
        options.yc_memory_mb = o.yc_memory_mb;
        options.pause = o.pause;
        let code = if plan.search != Search::Off {
            commands::tune_with(&options, None)
        } else {
            commands::run_with(&options, None)
        };
        state.last_stage = Some(step.stage);
        state.runs_done += 1;
        state.last_signature = campaign::signature(step.stage, &step.cores, &held);
        last_clean = code == 0;
        if os::cancelled() {
            println!("Auto cancelled.");
            return 130;
        }
    }
    println!("Auto stopped after {MAX_RUNS} runs.");
    0
}

/// Cancels a continuation that is counting down (`pbo resume --cancel`, or the window).
pub fn cancel_continuation() -> i32 {
    let store = Store::beside_program();
    match std::fs::write(file(&store, "auto_cancel"), "cancel") {
        Ok(()) => {
            println!("A continuation that is counting down will not start.");
            0
        }
        Err(e) => {
            eprintln!("could not write the cancel file: {e}");
            1
        }
    }
}

/// The count of continuations in a row and when the first of them was, as `count unix_seconds`.
fn read_resumes(store: &Store) -> (u32, i64) {
    let text = std::fs::read_to_string(file(store, "auto_resume.txt")).unwrap_or_default();
    let mut parts = text.split_whitespace();
    let count = parts.next().and_then(|c| c.parse().ok()).unwrap_or(0);
    let since = parts.next().and_then(|c| c.parse().ok()).unwrap_or(0);
    (count, since)
}

/// After a crash, when `pbo resume --continue` has put the proven values back: carries the campaign on, after a countdown.
/// Returns whether it went on.
pub fn continue_after_crash(countdown_seconds: u64) -> bool {
    let store = Store::beside_program();
    let Ok(text) = std::fs::read_to_string(file(&store, "auto_last.txt")) else {
        println!("There was no Auto run to carry on.");
        return false;
    };
    let args: Vec<String> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();

    // Never more than a few in a row: a core that keeps taking the machine down is for the person to look at.
    let now = crate::store::now_unix();
    let (count, since) = read_resumes(&store);
    let (count, since) = if count == 0 || now - since > RESUME_WINDOW_SECONDS {
        (0, now)
    } else {
        (count, since)
    };
    if count >= MAX_RESUMES_IN_A_ROW {
        println!(
            "Auto has already been carried on {count} times after crashes. It is not going on by itself again: look at `pbo whea` and `pbo knowledge`, then start it by hand."
        );
        return false;
    }

    // The countdown, which the person can end with a file (`pbo resume --cancel`) or by closing this program.
    let cancel = file(&store, "auto_cancel");
    let _ = std::fs::remove_file(&cancel);
    println!(
        "Auto goes on in {countdown_seconds} s (carried on {count} of {MAX_RESUMES_IN_A_ROW} times so far). `pbo resume --cancel` stops it."
    );
    for left in (1..=countdown_seconds).rev() {
        if cancel.exists() {
            let _ = std::fs::remove_file(&cancel);
            println!("Cancelled: Auto does not go on.");
            return false;
        }
        if left % 15 == 0 {
            println!("  {left} s");
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    let _ = std::fs::write(
        file(&store, "auto_resume.txt"),
        format!("{} {since}", count + 1),
    );
    auto(&args) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(extra: &[&str]) -> Options {
        let args: Vec<String> = extra.iter().map(|s| s.to_string()).collect();
        commands::parse(&args).unwrap()
    }

    #[test]
    fn the_steps_are_those_of_the_intensity_unless_the_person_named_their_own() {
        let o = options(&[]);
        assert_eq!(
            base_plan(&o, Intensity::Fast, false).search,
            Search::Step(5)
        );
        assert_eq!(
            base_plan(&o, Intensity::Thorough, false).search,
            Search::Fine
        );
        let o = options(&["--step", "2"]);
        assert_eq!(base_plan(&o, Intensity::Fast, true).search, Search::Step(2));
        let o = options(&["--fine"]);
        assert_eq!(base_plan(&o, Intensity::Normal, true).search, Search::Fine);
    }

    #[test]
    fn the_safety_the_start_and_the_cores_go_into_every_step_of_the_campaign() {
        let o = options(&[
            "--guardband",
            "4",
            "--start",
            "20",
            "--cores",
            "1,4",
            "--quiet",
        ]);
        let plan = base_plan(&o, Intensity::Fast, false);
        assert_eq!(plan.guardband, 4);
        assert_eq!(plan.start, 20);
        assert_eq!(plan.cores, vec![1, 4]);
        assert!(!plan.notify);
    }

    #[test]
    fn the_count_of_continuations_is_read_from_its_file_and_a_missing_file_is_none() {
        let dir = std::env::temp_dir().join(format!("pbo-autorun-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store { root: dir.clone() };
        assert_eq!(read_resumes(&store), (0, 0));
        std::fs::write(dir.join("auto_resume.txt"), "2 1793872800").unwrap();
        assert_eq!(read_resumes(&store), (2, 1_793_872_800));
        std::fs::write(dir.join("auto_resume.txt"), "garbage").unwrap();
        assert_eq!(read_resumes(&store), (0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
