//! `pbo whea`: the machine checks Windows has logged, by crash and by core. It only reads, and needs no rights.
//!
//! A fatal machine check (WHEA event 18) is what a core that gave a wrong answer under a Curve Optimizer value
//! leaves behind when the machine goes down without a trace: no dump, nothing but this event, written together with
//! the others of that crash when Windows starts again. Naming the core it belongs to is what links a crash in a game
//! to a value, so this lists them the way the program counts them during a test. The unexpected shutdowns
//! (Kernel-Power 41) are listed with them: one with no machine check next to it was not a machine check at all
//! (power, a hang, a reset), and says nothing about a core.

use std::collections::BTreeMap;

use pbo_core::whea::{self, WheaEvent};

use crate::eventlog::{self, Names};

/// Fatal checks written no more than this many seconds apart belong to one crash.
const CRASH_WINDOW_SECONDS: i64 = 120;
/// An unexpected shutdown logged this many seconds before the checks of a crash belongs to it.
const SHUTDOWN_WINDOW_SECONDS: i64 = 300;

/// `2026-08-06T13:14:06.0000000Z` as `2026-08-06 13:14:06`.
fn short(time: &str) -> String {
    time.chars().take(19).collect::<String>().replace('T', " ")
}

/// What was read from the log.
pub struct Found<'a> {
    pub fatal: &'a [WheaEvent],
    pub corrected: &'a [WheaEvent],
    pub shutdowns: &'a [WheaEvent],
    /// When the System log was last cleared, if it ever was.
    pub cleared: Option<&'a str>,
    pub days: i64,
}

/// The report as lines. `core_of` says which core an event names, `describe` how to write that down.
pub fn report(
    found: &Found,
    core_of: &dyn Fn(&WheaEvent) -> Option<usize>,
    describe: &dyn Fn(&WheaEvent) -> String,
) -> Vec<String> {
    let mut lines = vec![format!(
        "Machine checks Windows logged in the last {} days (times are UTC):",
        found.days
    )];
    if let Some(at) = found.cleared {
        lines.push(format!(
            "  The System log was last cleared at {}: what it held before is gone, so this says nothing about the time before.",
            short(at)
        ));
    }
    let crashes = whea::crashes(found.fatal, CRASH_WINDOW_SECONDS);
    if crashes.is_empty() && found.corrected.is_empty() && found.shutdowns.is_empty() {
        lines.push("  none.".to_string());
        return lines;
    }
    // Per core: the crashes that name it (once each, however many threads are named) and the corrected checks.
    let mut per_core: BTreeMap<usize, (u32, u32)> = BTreeMap::new();
    if !crashes.is_empty() {
        lines.push("  Fatal (the machine went down), one line for each crash:".to_string());
    }
    let mut explained = 0;
    for crash in &crashes {
        let when = crash.first().map_or(String::new(), |e| short(&e.time));
        let named: Vec<String> = crash.iter().map(describe).collect();
        let after = match whea::shutdown_before(crash, found.shutdowns, SHUTDOWN_WINDOW_SECONDS) {
            Some(s) => {
                explained += 1;
                format!("   (unexpected shutdown logged at {})", short(&s.time))
            }
            None => String::new(),
        };
        lines.push(format!("    {when}  {}{after}", named.join(", ")));
        let mut cores: Vec<usize> = crash.iter().filter_map(core_of).collect();
        cores.sort_unstable();
        cores.dedup();
        for c in cores {
            per_core.entry(c).or_default().0 += 1;
        }
    }
    if !found.shutdowns.is_empty() {
        let alone = found.shutdowns.len().saturating_sub(explained);
        lines.push(format!(
            "  Unexpected shutdowns (Kernel-Power 41): {}, {alone} of them with no fatal machine check next to them.",
            found.shutdowns.len()
        ));
    }
    if !found.corrected.is_empty() {
        lines.push(format!(
            "  Corrected (the hardware fixed it): {} in all.",
            found.corrected.len()
        ));
    }
    let mut unnamed = 0;
    for e in found.corrected {
        match core_of(e) {
            Some(c) => per_core.entry(c).or_default().1 += 1,
            None => unnamed += 1,
        }
    }
    if !per_core.is_empty() || unnamed > 0 {
        lines.push("  By core:".to_string());
        for (core, (fatal_n, corrected_n)) in &per_core {
            lines.push(format!(
                "    core {core}: named in {fatal_n} crash(es), {corrected_n} corrected check(s)"
            ));
        }
        if unnamed > 0 {
            lines.push(format!("    {unnamed} corrected check(s) name no core"));
        }
    }
    lines
}

pub fn run(args: &[String]) -> i32 {
    let mut days: i64 = 365;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--days" => match it
                .next()
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|d| (1..=3650).contains(d))
            {
                Some(d) => days = d,
                None => {
                    eprintln!("--days takes a number from 1 to 3650");
                    return 2;
                }
            },
            other => {
                eprintln!("unknown option {other}");
                return 2;
            }
        }
    }
    let cutoff = whea::iso_from_epoch(crate::store::now_unix() - days * 86_400);
    let names = Names::measure();
    // The fatal ones are asked for directly: a long run of corrected ones cannot crowd one out.
    let fatal: Vec<WheaEvent> = eventlog::fatal(200)
        .into_iter()
        .filter(|e| e.time > cutoff)
        .collect();
    let corrected: Vec<WheaEvent> = eventlog::since(&cutoff)
        .into_iter()
        .filter(|e| e.id == whea::EVENT_CORRECTED)
        .collect();
    let shutdowns = eventlog::unexpected_shutdowns(&cutoff);
    let cleared = eventlog::cleared_at();
    let found = Found {
        fatal: &fatal,
        corrected: &corrected,
        shutdowns: &shutdowns,
        cleared: cleared.as_deref(),
        days,
    };
    for line in report(&found, &|e| names.core_of(e), &|e| names.describe(e)) {
        println!("{line}");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: u32, time: &str, apic: Option<u32>) -> WheaEvent {
        WheaEvent {
            id,
            time: time.to_string(),
            warning: id == 19,
            apic_id: apic,
        }
    }

    /// Eight cores of two threads, the ids being the logical processors' numbers.
    fn core_of(e: &WheaEvent) -> Option<usize> {
        e.apic_id.filter(|a| *a < 16).map(|a| a as usize / 2)
    }

    fn describe(e: &WheaEvent) -> String {
        match (e.apic_id, core_of(e)) {
            (Some(a), Some(c)) => format!("APIC id {a} (core {c})"),
            _ => "no processor named".to_string(),
        }
    }

    fn lines(found: &Found) -> String {
        report(found, &core_of, &describe).join("\n")
    }

    const NONE: &[WheaEvent] = &[];

    #[test]
    fn two_crashes_that_name_the_same_two_cores_are_counted_per_core() {
        let fatal = vec![
            event(18, "2026-08-06T13:14:06.0000000Z", Some(8)),
            event(18, "2026-08-06T13:14:06.5000000Z", Some(0)),
            event(18, "2026-09-20T11:52:22.0000000Z", Some(9)),
            event(18, "2026-09-20T11:52:22.2000000Z", Some(0)),
        ];
        let text = lines(&Found {
            fatal: &fatal,
            corrected: NONE,
            shutdowns: NONE,
            cleared: Some("2026-10-30T08:00:00.0000000Z"),
            days: 365,
        });
        assert!(text.contains("2026-08-06 13:14:06  APIC id 8 (core 4), APIC id 0 (core 0)"));
        assert!(text.contains("2026-09-20 11:52:22  APIC id 9 (core 4), APIC id 0 (core 0)"));
        assert!(text.contains("core 0: named in 2 crash(es), 0 corrected"));
        assert!(text.contains("core 4: named in 2 crash(es), 0 corrected"));
        assert!(text.contains("last cleared at 2026-10-30 08:00:00"));
    }

    #[test]
    fn two_threads_of_one_core_in_one_crash_count_the_crash_once() {
        let fatal = vec![
            event(18, "2026-08-06T13:14:06Z", Some(8)),
            event(18, "2026-08-06T13:14:07Z", Some(9)),
        ];
        let text = lines(&Found {
            fatal: &fatal,
            corrected: NONE,
            shutdowns: NONE,
            cleared: None,
            days: 30,
        });
        assert!(text.contains("core 4: named in 1 crash(es)"), "{text}");
        assert!(!text.contains("cleared"));
    }

    #[test]
    fn corrected_checks_are_counted_per_core_and_one_without_a_core_is_said() {
        let corrected = vec![
            event(19, "2026-09-01T09:00:00Z", Some(8)),
            event(19, "2026-09-01T09:05:00Z", Some(8)),
            event(19, "2026-09-01T09:06:00Z", None),
        ];
        let text = lines(&Found {
            fatal: NONE,
            corrected: &corrected,
            shutdowns: NONE,
            cleared: None,
            days: 90,
        });
        assert!(
            text.contains("core 4: named in 0 crash(es), 2 corrected"),
            "{text}"
        );
        assert!(text.contains("1 corrected check(s) name no core"));
    }

    #[test]
    fn an_empty_log_says_none_and_a_cleared_one_says_that_too() {
        let text = lines(&Found {
            fatal: NONE,
            corrected: NONE,
            shutdowns: NONE,
            cleared: Some("2026-10-30T08:00:00Z"),
            days: 365,
        });
        assert!(text.contains("none."));
        assert!(text.contains("cleared"));
    }

    #[test]
    fn a_shutdown_just_before_the_checks_belongs_to_the_crash_and_one_alone_is_counted_as_such() {
        let fatal = vec![
            event(18, "2026-08-06T13:14:06Z", Some(8)),
            event(18, "2026-08-06T13:14:06Z", Some(0)),
        ];
        let shutdowns = vec![
            event(41, "2026-08-06T13:14:00Z", None),
            // Days later, with no machine check next to it: a power loss or a hang.
            event(41, "2026-08-12T20:00:00Z", None),
        ];
        let text = lines(&Found {
            fatal: &fatal,
            corrected: NONE,
            shutdowns: &shutdowns,
            cleared: None,
            days: 365,
        });
        assert!(
            text.contains("(unexpected shutdown logged at 2026-08-06 13:14:00)"),
            "{text}"
        );
        assert!(
            text.contains("Unexpected shutdowns (Kernel-Power 41): 2, 1 of them with no fatal machine check next to them."),
            "{text}"
        );
    }

    #[test]
    fn shutdowns_alone_are_not_none() {
        let shutdowns = vec![event(41, "2026-08-12T20:00:00Z", None)];
        let text = lines(&Found {
            fatal: NONE,
            corrected: NONE,
            shutdowns: &shutdowns,
            cleared: None,
            days: 30,
        });
        assert!(!text.contains("none."), "{text}");
        assert!(text.contains("1 of them with no fatal machine check"));
    }
}
