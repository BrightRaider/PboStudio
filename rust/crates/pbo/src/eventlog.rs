//! WHEA machine checks, read from the Windows event log with the system's own `wevtutil` (invariant I6).
//!
//! The query and the reading of the XML are in `pbo_core::whea`; this runs the query. `wevtutil` ships with
//! Windows, which keeps the event log API out of this program. A failed query is an empty answer: no WHEA provider
//! on this machine, or an unreadable log, is not a reason to stop a test.
//!
//! Only Windows itself writes real machine checks. To try the whole chain (an event appears while a core is under
//! test, and the core is held to account) `PBO_TEST_EVENTLOG=Log|Provider` names a second place to read from, next to
//! the real one and never instead of it.

use pbo_core::whea::{self, WheaEvent};

/// Where machine checks are read from: always the System log's WHEA provider, and the test source when one is named.
fn sources() -> Vec<(String, String)> {
    let mut list = vec![("System".to_string(), whea::PROVIDER.to_string())];
    if let Some(extra) = std::env::var("PBO_TEST_EVENTLOG")
        .ok()
        .and_then(|v| whea::test_source(&v))
    {
        list.push(extra);
    }
    list
}

fn raw(log: &str, xpath: &str, newest_first: bool, max: u32) -> Option<String> {
    let query = format!("/q:{xpath}");
    let order = format!("/rd:{newest_first}");
    let count = format!("/c:{max}");
    let args = [
        "qe",
        log,
        query.as_str(),
        "/f:xml",
        order.as_str(),
        count.as_str(),
    ];
    // By full path and with a limit: a log query that hangs must not freeze the test that asked.
    match pbo_os::system::run("wevtutil.exe", &args, std::time::Duration::from_secs(30)) {
        Some(out) if out.success => Some(out.stdout_text()),
        _ => None,
    }
}

fn query(log: &str, xpath: &str, newest_first: bool, max: u32) -> Vec<WheaEvent> {
    raw(log, xpath, newest_first, max)
        .map(|xml| whea::parse_events(&xml))
        .unwrap_or_default()
}

/// Which core a machine check names, from the processor's own APIC ids (not from the habit that an id is the logical
/// processor's number). Needs no rights.
pub struct Names {
    cores: Vec<pbo_core::apic::Core>,
    apic_to_logical: std::collections::HashMap<i32, i32>,
}

impl Names {
    pub fn measure() -> Names {
        let per_core = pbo_os::threads_per_core().max(1) as usize;
        let cores = (0..pbo_os::logical_processors() / per_core)
            .map(|c| pbo_core::apic::Core {
                index: c,
                logical_processors: (0..per_core).map(|t| (c * per_core + t) as i32).collect(),
            })
            .collect();
        let apic_to_logical = pbo_os::apic_ids()
            .iter()
            .enumerate()
            .filter_map(|(logical, id)| id.map(|id| (id as i32, logical as i32)))
            .collect();
        Names {
            cores,
            apic_to_logical,
        }
    }

    pub fn core_of(&self, event: &WheaEvent) -> Option<usize> {
        let apic_id = event.apic_id? as i32;
        pbo_core::apic::core_for_apic_id(apic_id, &self.cores, Some(&self.apic_to_logical))
    }

    /// `APIC id 8 (core 4)`, or what is known when the id is missing or belongs to no core.
    pub fn describe(&self, event: &WheaEvent) -> String {
        match (event.apic_id, self.core_of(event)) {
            (Some(id), Some(core)) => format!("APIC id {id} (core {core})"),
            (Some(id), None) => format!("APIC id {id} (no core of this processor)"),
            (None, _) => "no processor named".to_string(),
        }
    }
}

/// The unexpected shutdowns (Kernel-Power 41) after `since`, oldest first.
pub fn unexpected_shutdowns(since: &str) -> Vec<WheaEvent> {
    let mut all = query(
        "System",
        &whea::xpath_unexpected_shutdowns(since),
        false,
        200,
    );
    all.sort_by(|a, b| a.time.cmp(&b.time));
    all
}

/// When the System log was last cleared (event 104), if it ever was: what it held before is gone, and nothing
/// found in it says anything about the time before.
pub fn cleared_at() -> Option<String> {
    let xpath = "*[System[Provider[@Name='Microsoft-Windows-Eventlog'] and (EventID=104)]]";
    raw("System", xpath, true, 50).and_then(|xml| whea::cleared_time(&xml, "System"))
}

/// Machine checks after `since` (UTC, as `pbo_os::utc_iso` writes it), oldest first.
pub fn since(since: &str) -> Vec<WheaEvent> {
    let mut all: Vec<WheaEvent> = sources()
        .iter()
        .flat_map(|(log, provider)| {
            query(
                log,
                &whea::xpath_for(provider, Some(since), false),
                false,
                200,
            )
        })
        .collect();
    all.sort_by(|a, b| a.time.cmp(&b.time));
    all
}

/// Fatal machine checks, newest first. Asking for them directly means a run of corrected ones cannot crowd one out.
pub fn fatal(max: u32) -> Vec<WheaEvent> {
    let mut all: Vec<WheaEvent> = sources()
        .iter()
        .flat_map(|(log, provider)| query(log, &whea::xpath_for(provider, None, true), true, max))
        .collect();
    all.sort_by(|a, b| b.time.cmp(&a.time));
    all.truncate(max as usize);
    all
}
