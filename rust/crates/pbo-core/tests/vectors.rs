//! Runs the language-neutral vectors in `spec/vectors` against the Rust implementation (ADR-0002).
//!
//! Every case of every file is run. An area that this crate does not implement yet is listed as pending, and
//! the list of pending areas is pinned below: a new vector file cannot be added without deciding, here, that
//! Rust does not do it yet. When the list is empty, gate G2 is met for the vectors.

use serde_json::{json, Value};
use std::path::PathBuf;

/// Areas whose vectors Rust does not run yet. Shrinks to nothing by gate G2.
const PENDING: &[&str] = &[];

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../spec/vectors")
}

fn int(v: &Value, name: &str) -> i32 {
    v[name]
        .as_i64()
        .unwrap_or_else(|| panic!("input '{name}' is not an integer: {v}")) as i32
}

fn int_or_none(v: &Value, name: &str) -> Option<i32> {
    match v.get(name) {
        None | Some(Value::Null) => None,
        Some(x) => Some(
            x.as_i64()
                .unwrap_or_else(|| panic!("input '{name}' is not an integer: {v}"))
                as i32,
        ),
    }
}

fn ints(v: &Value, name: &str) -> Vec<i32> {
    v[name]
        .as_array()
        .unwrap_or_else(|| panic!("input '{name}' is not a list: {v}"))
        .iter()
        .map(|x| x.as_i64().expect("integer") as i32)
        .collect()
}

fn pm_layout(input: &Value) -> pbo_core::pm::Layout {
    match input["layout"].as_str().unwrap() {
        "Zen 3" => pbo_core::pm::ZEN3,
        "Zen 4" => pbo_core::pm::ZEN4,
        other => panic!("unknown layout {other}"),
    }
}

/// A float from a vector: a number, or the strings "NaN" and "Infinity".
fn float(v: &Value) -> f32 {
    match v {
        Value::String(s) if s == "NaN" => f32::NAN,
        Value::String(s) if s == "Infinity" => f32::INFINITY,
        other => other.as_f64().expect("number") as f32,
    }
}

fn pm_read(input: &Value) -> Value {
    let layout = pm_layout(input);
    let mut table = vec![0.0f32; input["length"].as_u64().unwrap() as usize];
    for (index, value) in input["values"].as_object().unwrap() {
        table[index.parse::<usize>().unwrap()] = float(value);
    }
    let Some(s) = layout.read(&table) else {
        return Value::Null;
    };

    let mut all = serde_json::Map::new();
    all.insert("pptLimit".into(), json!(s.ppt_limit));
    all.insert("pptNow".into(), json!(s.ppt_now));
    all.insert("tdcLimit".into(), json!(s.tdc_limit));
    all.insert("tdcNow".into(), json!(s.tdc_now));
    all.insert("edcLimit".into(), json!(s.edc_limit));
    all.insert("edcNow".into(), json!(s.edc_now));
    all.insert("tempLimit".into(), json!(s.temp_limit));
    all.insert("tempNow".into(), json!(s.temp_now));
    all.insert("hasLimits".into(), json!(s.has_limits()));

    match input.get("pick").and_then(Value::as_array) {
        Some(keys) => {
            let keep: Vec<&str> = keys.iter().filter_map(Value::as_str).collect();
            Value::Object(
                all.into_iter()
                    .filter(|(k, _)| keep.contains(&k.as_str()))
                    .collect(),
            )
        }
        None => Value::Object(all),
    }
}

fn parse_knowledge(v: &Value) -> pbo_core::knowledge::Knowledge {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(core, k)| {
            (
                core.parse::<u32>().unwrap(),
                pbo_core::knowledge::CoreKnowledge {
                    best_passed: int_or_none(k, "bestPassed"),
                    worst_failed: int_or_none(k, "worstFailed"),
                },
            )
        })
        .collect()
}

fn dump_knowledge(k: &pbo_core::knowledge::Knowledge) -> Value {
    Value::Object(
        k.iter()
            .map(|(core, k)| {
                (
                    core.to_string(),
                    json!({ "bestPassed": k.best_passed, "worstFailed": k.worst_failed }),
                )
            })
            .collect(),
    )
}

fn parse_margins(v: &Value) -> std::collections::BTreeMap<u32, i32> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(core, m)| (core.parse::<u32>().unwrap(), m.as_i64().unwrap() as i32))
        .collect()
}

fn core_list(v: &Value) -> Vec<u32> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_u64().unwrap() as u32)
        .collect()
}

fn tuner_mode(input: &Value) -> pbo_core::tuner::Mode {
    match input["mode"].as_str().unwrap() {
        "coarse" => pbo_core::tuner::Mode::Coarse,
        "fine" => pbo_core::tuner::Mode::Fine,
        other => match other
            .strip_prefix("steps:")
            .and_then(|n| n.parse::<i32>().ok())
        {
            Some(n) => pbo_core::tuner::Mode::Steps(n),
            None => panic!("unknown mode {other}"),
        },
    }
}

/// Plays the steps of a log-tail case against an in-memory "file".
fn log_tail_poll(input: &Value) -> Value {
    let mut tail = pbo_core::logtail::LogTail::default();
    let mut file: Option<Vec<u8>> = Some(Vec::new());
    let mut results = Vec::new();

    for step in input["steps"].as_array().unwrap() {
        if let Some(text) = step.get("append").and_then(Value::as_str) {
            file.get_or_insert_with(Vec::new)
                .extend_from_slice(text.as_bytes());
        } else if let Some(text) = step.get("set").and_then(Value::as_str) {
            file = Some(text.as_bytes().to_vec());
        } else if step.get("remove").is_some() {
            file = None;
        } else if step.get("poll").is_some() {
            let lines = tail.poll(file.as_deref());
            results.push(json!({ "lines": lines, "offset": tail.offset }));
        } else {
            panic!("unknown step {step}");
        }
    }
    Value::Array(results)
}

fn slot_pair(slot: pbo_core::coremap::Slot) -> Value {
    json!([slot.ccd, slot.slot])
}

/// Runs one case. `None` means this function is not implemented here.
fn run(area: &str, function: &str, input: &Value) -> Option<Value> {
    use pbo_core::{coremap, depth, engine, logday, pm};
    Some(match (area, function) {
        ("pm-table-layout", "layoutFor") => {
            match pm::layout_for(input["version"].as_u64().unwrap() as u32) {
                Some(l) => json!({ "name": l.name, "verified": l.verified }),
                None => Value::Null,
            }
        }
        ("pm-table-layout", "highestIndex") => json!(pm_layout(input).highest_index()),
        ("pm-table-layout", "read") => pm_read(input),
        ("smu-core-map", "build") => {
            let disabled: Option<Vec<u32>> = input["disabled"]
                .as_array()
                .map(|a| a.iter().map(|x| x.as_u64().unwrap() as u32).collect());
            match coremap::build(
                disabled.as_deref().unwrap_or(&[]),
                input["windowsCores"].as_i64().unwrap().max(0) as usize,
            ) {
                Some(map) => Value::Array(map.into_iter().map(slot_pair).collect()),
                None => Value::Null,
            }
        }
        ("duration-format", "hours") => json!(pbo_core::duration::hours(
            input["seconds"].as_i64().unwrap()
        )),
        ("duration-format", "hoursSeconds") => json!(pbo_core::duration::hours_seconds(
            input["seconds"].as_i64().unwrap()
        )),
        ("auto-tuner" | "search-steps", "stepSize") => {
            json!(pbo_core::tuner::step_size(tuner_mode(input)))
        }
        ("auto-tuner", "applyGuardband") => json!(pbo_core::tuner::apply_guardband(
            int(input, "lastKnownGood"),
            int(input, "guardband")
        )),
        ("auto-tuner" | "search-steps", "nextStep") => {
            let s = &input["state"];
            let state = pbo_core::tuner::CoreState {
                last_known_good: int_or_none(s, "lastKnownGood"),
                ascending: s["ascending"].as_bool().unwrap(),
            };
            let r = pbo_core::tuner::next_step(
                int(input, "margin"),
                input["passed"].as_bool().unwrap(),
                tuner_mode(input),
                int(input, "maxLimit"),
                state,
                int(input, "guardband"),
            );
            json!({
                "nextMargin": r.next_margin,
                "locked": r.locked,
                "passed": r.passed,
                "state": { "lastKnownGood": r.state.last_known_good, "ascending": r.state.ascending }
            })
        }
        ("auto-tuner" | "search-steps", "worstCaseSlots") => {
            json!(pbo_core::tuner::worst_case_slots(
                int(input, "margin"),
                int(input, "maxLimit"),
                tuner_mode(input),
                int(input, "maxPasses"),
                input["canClimb"].as_bool().unwrap()
            ))
        }
        ("core-knowledge", "floorFor") => {
            let k = pbo_core::knowledge::CoreKnowledge {
                best_passed: int_or_none(input, "bestPassed"),
                worst_failed: int_or_none(input, "worstFailed"),
            };
            json!(k.floor_for(int(input, "chipLimit")))
        }
        ("core-knowledge", "isKnownBad") => {
            let k = pbo_core::knowledge::CoreKnowledge {
                best_passed: None,
                worst_failed: int_or_none(input, "worstFailed"),
            };
            json!(k.is_known_bad(int(input, "margin")))
        }
        ("core-knowledge", "record") => {
            use pbo_core::knowledge::{record_failure, record_pass};
            let mut k = pbo_core::knowledge::Knowledge::new();
            for r in input["results"].as_array().unwrap() {
                if let Some(p) = r.get("pass") {
                    record_pass(
                        &mut k,
                        p[0].as_u64().unwrap() as u32,
                        p[1].as_i64().unwrap() as i32,
                    );
                } else if let Some(f) = r.get("fail") {
                    record_failure(
                        &mut k,
                        f[0].as_u64().unwrap() as u32,
                        f[1].as_i64().unwrap() as i32,
                    );
                } else {
                    panic!("unknown result {r}");
                }
            }
            dump_knowledge(&k)
        }
        ("core-knowledge", "coresWithKnownBad") => json!(
            pbo_core::knowledge::cores_with_known_bad(&parse_knowledge(&input["knowledge"]))
        ),
        ("campaign", "recommend") => {
            let s = pbo_core::campaign::recommend(
                &core_list(&input["cores"]),
                &parse_margins(&input["margins"]),
                &parse_knowledge(&input["knowledge"]),
                int(input, "chipLimit"),
                input["driverReady"].as_bool().unwrap(),
                input["engineReady"].as_bool().unwrap(),
                int(input, "guardband"),
            );
            json!({
                "stage": s.stage.name(),
                "profile": s.profile.name(),
                "useAutoTuner": s.use_auto_tuner,
                "cores": s.cores,
                "action": if s.action == pbo_core::campaign::Action::OpenSetup { "OpenSetup" } else { "ConfigureRun" },
                "showProfile": s.show_profile
            })
        }
        ("campaign", "phaseNumberOf") => json!(pbo_core::campaign::Stage::from_name(
            input["stage"].as_str().unwrap()
        )
        .unwrap()
        .phase_number()),
        ("campaign", "signature") => json!(pbo_core::campaign::signature(
            pbo_core::campaign::Stage::from_name(input["stage"].as_str().unwrap()).unwrap(),
            &core_list(&input["cores"]),
            &parse_margins(&input["margins"])
        )),
        ("campaign", "backOffTo") => {
            let k = pbo_core::knowledge::CoreKnowledge {
                best_passed: int_or_none(input, "bestPassed"),
                worst_failed: int_or_none(input, "worstFailed"),
            };
            json!(pbo_core::campaign::back_off_to(
                &k,
                int(input, "chipLimit"),
                int(input, "guardband")
            ))
        }
        ("campaign", "decide") => {
            use pbo_core::campaign::{decide, Action, NextStep, Profile, Stage, State};
            let s = &input["state"];
            let state = State {
                last_stage: Stage::from_name(s["lastStage"].as_str().unwrap()),
                runs_done: s["runsDone"].as_u64().unwrap() as u32,
                last_signature: s["lastSignature"].as_str().unwrap().to_string(),
            };
            let st = &input["step"];
            let step = NextStep {
                profile: Profile::HeavyFfts,
                stage: Stage::from_name(st["stage"].as_str().unwrap()).unwrap(),
                use_auto_tuner: false,
                cores: core_list(&st["cores"]),
                action: if st["action"] == "OpenSetup" {
                    Action::OpenSetup
                } else {
                    Action::ConfigureRun
                },
                show_profile: true,
            };
            let d = decide(
                &state,
                &step,
                &parse_margins(&input["margins"]),
                input["lastRunClean"].as_bool().unwrap(),
            );
            json!({ "verdict": d.verdict.name(), "phaseNumber": d.phase_number })
        }
        ("log-tail", "poll") => log_tail_poll(input),
        ("download-policy", "isAllowed") => {
            let hosts: Vec<&str> = input["allowedHosts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| h.as_str().unwrap())
                .collect();
            json!(pbo_core::download::is_allowed(
                input["url"].as_str().unwrap(),
                &hosts
            ))
        }
        ("download-policy", "registrableRoot") => json!(pbo_core::download::registrable_root(
            input["host"].as_str().unwrap()
        )),
        ("download-policy", "pinnedInstallerSha256") => {
            json!(pbo_core::download::PAWNIO_INSTALLER_SHA256)
        }
        ("interrupted-run", "classify") => {
            let written = input["hasState"].as_bool().unwrap().then_some(1_000_000i64);
            let started = 1_000_000 + input["bootOffsetSeconds"].as_i64().unwrap();
            match pbo_core::interrupted::system_restarted(written, started) {
                Some(restarted) => json!({ "systemRestarted": restarted }),
                None => Value::Null,
            }
        }
        ("apic-topology", "coreForApicId") => {
            use pbo_core::apic::{core_for_apic_id, Core};
            let count = input["cores"]["count"].as_i64().unwrap() as i32;
            let t = input["cores"]["threadsPerCore"].as_i64().unwrap() as i32;
            let cores: Vec<Core> = (0..count)
                .map(|n| Core {
                    index: n as usize,
                    logical_processors: (n * t..n * t + t).collect(),
                })
                .collect();
            let map: Option<std::collections::HashMap<i32, i32>> = match &input["map"] {
                Value::Null => None,
                m if m.get("identity").is_some() => {
                    let n = m["identity"].as_i64().unwrap() as i32;
                    Some((0..n).map(|x| (x, x)).collect())
                }
                m => Some(
                    m["pairs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| (p[0].as_i64().unwrap() as i32, p[1].as_i64().unwrap() as i32))
                        .collect(),
                ),
            };
            match core_for_apic_id(int(input, "apicId"), &cores, map.as_ref()) {
                Some(i) => json!(i),
                None => Value::Null,
            }
        }
        ("smu-core-map", "fallback") => match coremap::fallback(
            int(input, "core"),
            int(input, "ccdCount"),
            int(input, "windowsCores"),
        ) {
            Some(s) => slot_pair(s),
            None => Value::Null,
        },
        ("engine-output", "prime95") => {
            engine_line(engine::classify_prime95(input["line"].as_str().unwrap()))
        }
        ("engine-output", "ycruncher") => {
            engine_line(engine::classify_ycruncher(input["line"].as_str().unwrap()))
        }
        ("engine-output", "prime95Cycles") => {
            let mut cycles = engine::Prime95Cycles::default();
            for k in ints(input, "sizes") {
                cycles.feed(&engine::Line::PassedFft(k as u32));
            }
            json!(cycles.completed)
        }
        ("engine-output", "ycruncherCycles") => {
            json!(engine::ycruncher_cycles(int(input, "headers") as u32))
        }
        ("engine-launch", "prime95Config") => {
            let mode = match input["mode"].as_str().unwrap() {
                "Sse" => engine::Prime95Mode::Sse,
                "Avx" => engine::Prime95Mode::Avx,
                "Avx2" => engine::Prime95Mode::Avx2,
                "Avx512" => engine::Prime95Mode::Avx512,
                other => panic!("unknown mode {other}"),
            };
            let fft = engine::FftPreset::from_name(input["fft"].as_str().unwrap()).unwrap();
            json!(engine::prime95_config(
                mode,
                fft,
                int(input, "memoryMb") as u32,
                int(input, "threads") as u32
            ))
        }
        ("engine-launch", "fftRange") => {
            let (lo, hi) = engine::FftPreset::from_name(input["fft"].as_str().unwrap())
                .unwrap()
                .range();
            json!([lo, hi])
        }
        ("engine-launch", "ycruncherConfig") => {
            let algorithms: Vec<&str> = input["algorithms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap())
                .collect();
            json!(engine::ycruncher_config(
                &ints(input, "logical")
                    .iter()
                    .map(|l| *l as usize)
                    .collect::<Vec<_>>(),
                int(input, "seconds") as u32,
                &algorithms
            ))
        }
        ("engine-launch", "ycruncherArguments") => {
            json!(engine::ycruncher_arguments(
                input["log"].as_str().unwrap(),
                input["config"].as_str().unwrap()
            ))
        }
        ("engine-launch", "ycruncherPrefixes") => {
            json!(engine::ycruncher_preferred_prefixes(
                int(input, "generation") as u32
            ))
        }
        ("log-day", "logDay") => {
            let d = date_of(input);
            let day = logday::log_day(d, int(input, "hour") as u32, int(input, "boundary") as u32);
            json!([day.year, day.month, day.day])
        }
        ("log-day", "fileName") => {
            json!(logday::file_name(date_of(input), int(input, "part") as u32))
        }
        ("log-day", "partFor") => {
            let sizes: Vec<u64> = ints(input, "sizes").iter().map(|s| *s as u64).collect();
            json!(logday::part_for(&sizes, int(input, "limit") as u64))
        }
        ("log-day", "line") => json!(logday::line(
            int(input, "hour") as u32,
            int(input, "minute") as u32,
            int(input, "second") as u32,
            input["run"].as_str().unwrap(),
            input["text"].as_str().unwrap()
        )),
        ("core-order", "arrange") => {
            let order =
                pbo_core::order::CoreOrder::from_id(input["order"].as_str().unwrap()).unwrap();
            let cores: Vec<usize> = input["cores"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_u64().unwrap() as usize)
                .collect();
            json!(pbo_core::order::arrange(order, &cores, 1))
        }
        ("on-error", "policy") => {
            let on_error =
                pbo_core::onerror::OnError::from_id(input["onError"].as_str().unwrap()).unwrap();
            let mut policy = pbo_core::onerror::ErrorPolicy::new(on_error);
            for c in input["failed"].as_array().unwrap() {
                policy.failed(c.as_u64().unwrap() as usize);
            }
            let order: Vec<usize> = input["order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_u64().unwrap() as usize)
                .collect();
            json!({ "stop": policy.stop_requested(), "remaining": policy.remaining(&order) })
        }
        ("on-error", "parse") => {
            match pbo_core::onerror::OnError::from_id(input["id"].as_str().unwrap()) {
                Some(o) => json!(o.id()),
                None => Value::Null,
            }
        }
        ("fft-range", "range") => {
            match pbo_core::engine::FftPreset::from_name(input["text"].as_str().unwrap()) {
                Some(f) => json!([f.range().0, f.range().1]),
                None => Value::Null,
            }
        }
        ("fft-range", "text") => {
            json!(
                pbo_core::engine::FftPreset::from_name(input["text"].as_str().unwrap())
                    .unwrap()
                    .to_text()
            )
        }
        ("core-order", "parse") => {
            match pbo_core::order::CoreOrder::from_id(input["id"].as_str().unwrap()) {
                Some(o) => json!(o.id()),
                None => Value::Null,
            }
        }
        ("transient-mode", "dutyCycle") => {
            let t =
                pbo_core::transient::Transient::from_text(input["text"].as_str().unwrap()).unwrap();
            json!(t.duty_cycle())
        }
        ("transient-mode", "normalise") => {
            match pbo_core::transient::Transient::from_text(input["text"].as_str().unwrap()) {
                Some(t) => json!(t.to_text()),
                None => Value::Null,
            }
        }
        ("transient-mode", "ofProfile") => {
            let p = pbo_core::profiles::by_id(input["id"].as_str().unwrap()).unwrap();
            json!(p.transient_mode().to_text())
        }
        ("auto-intensity", "minutes") => {
            let i =
                pbo_core::auto::Intensity::from_id(input["intensity"].as_str().unwrap()).unwrap();
            json!(i.minutes(int(input, "phase")))
        }
        ("auto-intensity", "searchMode") => {
            let i =
                pbo_core::auto::Intensity::from_id(input["intensity"].as_str().unwrap()).unwrap();
            json!(match i.search_mode() {
                pbo_core::tuner::Mode::Coarse => "Coarse",
                pbo_core::tuner::Mode::Fine => "Fine",
                pbo_core::tuner::Mode::Steps(_) => "Steps",
            })
        }
        ("auto-intensity", "parse") => {
            match pbo_core::auto::Intensity::from_id(input["id"].as_str().unwrap()) {
                Some(i) => json!(i.id()),
                None => Value::Null,
            }
        }
        ("auto-intensity", "estimate") => {
            let i =
                pbo_core::auto::Intensity::from_id(input["intensity"].as_str().unwrap()).unwrap();
            let e = pbo_core::auto::estimate(
                i,
                &core_list(&input["cores"]),
                &parse_margins(&input["margins"]),
                &parse_knowledge(&input["knowledge"]),
                int(input, "chipLimit"),
                int(input, "guardband"),
                int(input, "fromPhase"),
            );
            json!({
                "total": e.total_minutes,
                "perPhase": e.per_phase.iter().map(|(p, m)| json!([p, m])).collect::<Vec<_>>(),
            })
        }
        ("campaign-estimate", "estimate") => {
            let profiles: std::collections::BTreeMap<String, pbo_core::estimate::PhaseProfile> =
                input["profiles"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(name, v)| {
                        let a = v.as_array().unwrap();
                        (
                            name.clone(),
                            pbo_core::estimate::PhaseProfile {
                                minutes_per_core: a[0].as_u64().unwrap() as u32,
                                iterations: a[1].as_u64().unwrap() as u32,
                                legs: a[2].as_u64().unwrap() as u32,
                            },
                        )
                    })
                    .collect();
            let e = pbo_core::estimate::estimate(
                &profiles,
                &core_list(&input["cores"]),
                &parse_margins(&input["margins"]),
                &parse_knowledge(&input["knowledge"]),
                int(input, "chipLimit"),
                int(input, "guardband"),
                int(input, "fromPhase"),
            );
            json!({
                "total": e.total_minutes,
                "perPhase": e.per_phase.iter().map(|(p, m)| json!([p, m])).collect::<Vec<_>>(),
            })
        }
        ("core-quality", "getCoreQualities") => {
            let cppc: Option<Vec<i32>> = input["cppc"]
                .as_array()
                .map(|a| a.iter().map(|x| x.as_i64().unwrap() as i32).collect());
            let q = pbo_core::quality::core_qualities(
                input["coreCount"].as_u64().unwrap() as usize,
                cppc.as_deref(),
            );
            Value::Object(
                q.iter()
                    .map(|q| {
                        (
                            q.core.to_string(),
                            json!({ "rank": q.rank.name(), "cppc": q.cppc }),
                        )
                    })
                    .collect(),
            )
        }
        ("test-profiles", "ids") => json!(pbo_core::profiles::all()
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>()),
        ("test-profiles", "profile") => {
            use pbo_core::profiles::{Order, Transient};
            let p = pbo_core::profiles::by_id(input["id"].as_str().unwrap()).unwrap();
            json!({
                "minutesPerCore": p.minutes_per_core,
                "iterations": p.iterations,
                "threads": p.threads,
                "order": match p.order { Order::Sequential => "Sequential", Order::Alternate => "Alternate" },
                "suspendEverySeconds": p.suspend_every_seconds,
                "suspendForSeconds": p.suspend_for_seconds,
                "transient": match p.transient { Transient::Periodic => "Periodic", Transient::MicroBurst => "MicroBurst" },
                "essential": p.essential,
                "legs": p.legs.iter().map(|l| json!([l.engine.name(), l.mode.name(), l.fft.name()])).collect::<Vec<_>>(),
            })
        }
        ("smu-messages", "getMargin") | ("smu-messages", "setMargin") => {
            use pbo_core::families::{self, Generation};
            let generation = match input["generation"].as_str().unwrap() {
                "Zen3" => Generation::Zen3,
                "Zen4" => Generation::Zen4,
                "Zen5" => Generation::Zen5,
                other => panic!("unknown generation {other}"),
            };
            let message = if function == "getMargin" {
                families::get_margin_message(generation)
            } else {
                families::set_margin_message_mp1(generation)
            };
            json!(message.unwrap())
        }
        ("saved-profile", "check") => {
            let margins: std::collections::BTreeMap<i32, i32> = input["margins"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.parse().unwrap(), v.as_i64().unwrap() as i32))
                .collect();
            let c = pbo_core::profile::check(
                input["profileCpu"].as_str().unwrap(),
                &margins,
                input["currentCpu"].as_str().unwrap(),
                int(input, "coreCount"),
                int(input, "deepest"),
                int(input, "shallowest"),
            );
            json!({
                "margins": c.margins.iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<serde_json::Map<_, _>>(),
                "problems": c.problems,
                "wrongProcessor": c.wrong_processor,
            })
        }
        ("margin-encoding", "psmMarginArg") => {
            json!(pbo_core::margin::psm_arg(int(input, "margin")))
        }
        ("margin-encoding", "setArgument") => json!(pbo_core::margin::set_argument(
            int(input, "coreMask") as u32,
            int(input, "margin")
        )),
        ("curve-optimizer-range", "effective") => {
            json!(depth::effective(
                int(input, "modelLimit"),
                int_or_none(input, "userLimit"),
                &ints(input, "readFromHardware"),
                int_or_none(input, "appliedFloor")
            ))
        }
        ("curve-optimizer-range", "wasKept") => json!(depth::was_kept(
            int(input, "requested"),
            int_or_none(input, "readBack")
        )),
        _ => return None,
    })
}

/// Numbers by value, objects without regard to key order.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= 1e-6 * x.abs().max(1.0)
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn every_vector_holds_in_rust() {
    let mut files: Vec<_> = std::fs::read_dir(vectors_dir())
        .expect("spec/vectors")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no vector files found");

    let mut ran = 0usize;
    let mut failures = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for file in &files {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let area = doc["area"].as_str().unwrap().to_string();
        let default_kind = doc["kind"].as_str().unwrap().to_string();
        let mut area_ran = 0;
        let mut area_skipped = 0;

        for case in doc["cases"].as_array().unwrap() {
            let kind = case
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or(&default_kind);
            assert!(
                kind == "parity" || kind == "new",
                "unknown kind '{kind}' in {}",
                file.display()
            );
            let function = case["function"].as_str().unwrap();
            let name = case["name"].as_str().unwrap();

            match run(&area, function, &case["input"]) {
                Some(actual) => {
                    ran += 1;
                    area_ran += 1;
                    if !same(&case["expected"], &actual) {
                        failures.push(format!(
                            "{area}/{function}: {name}\n    expected {}\n    actual   {actual}",
                            case["expected"]
                        ));
                    }
                }
                None => area_skipped += 1,
            }
        }

        if area_skipped > 0 {
            assert_eq!(
                area_ran, 0,
                "{area} is half implemented: {area_ran} cases ran, {area_skipped} did not"
            );
            pending.push(area);
        }
    }

    assert!(
        failures.is_empty(),
        "{} vector(s) failed in Rust:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    let expected: Vec<String> = PENDING.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        pending, expected,
        "the areas Rust does not run yet changed: update PENDING on purpose"
    );
    println!("{ran} vector cases ran in Rust; pending areas: {pending:?}");
}

fn engine_line(line: pbo_core::engine::Line) -> Value {
    use pbo_core::engine::Line;
    match line {
        Line::PassedFft(k) => json!({ "kind": "passedFft", "fft": k }),
        Line::Passed => json!({ "kind": "passed" }),
        Line::Failure(text) => json!({ "kind": "failure", "text": text }),
        Line::Other => json!({ "kind": "other" }),
    }
}

fn date_of(input: &Value) -> pbo_core::logday::Date {
    let a = input["date"].as_array().unwrap();
    pbo_core::logday::Date {
        year: a[0].as_i64().unwrap() as i32,
        month: a[1].as_u64().unwrap() as u32,
        day: a[2].as_u64().unwrap() as u32,
    }
}
