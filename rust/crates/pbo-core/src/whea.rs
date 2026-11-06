//! Machine check events from the Windows event log (invariant I6).
//!
//! An undervolted core usually does not crash outright: it produces a *corrected* machine check (WHEA event 19) and
//! the machine keeps running as if nothing happened. Without watching for these, a value that is quietly corrupting
//! work looks the same as a stable one. This is the pure half: it builds the query and reads the events out of the
//! XML Windows returns; running the query is the operating system layer's job. The XML is read for its numbers
//! (`EventID`, `Level`, `SystemTime`, the `ApicId` data field) and not for the localised description, which is
//! different on every installation. Tests: the unit tests below (the C# side reads the event through the .NET event log API, which has no XML to compare).

pub const PROVIDER: &str = "Microsoft-Windows-WHEA-Logger";

/// The fatal machine check: the machine went down.
pub const EVENT_FATAL: u32 = 18;
/// The corrected machine check: the core got something wrong and the hardware fixed it.
pub const EVENT_CORRECTED: u32 = 19;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheaEvent {
    pub id: u32,
    /// UTC, as Windows writes it: `2026-11-05T10:00:00.123456700Z`. Sorts as text.
    pub time: String,
    /// Level 3. Anything more severe counts as a hard error.
    pub warning: bool,
    /// The processor APIC id the event names, when it carries one.
    pub apic_id: Option<u32>,
}

/// The XPath that selects WHEA events, optionally only those after `since` (UTC, ISO 8601 as above) and optionally
/// only the fatal ones. Asking for the fatal ones directly cannot be crowded out by a long run of corrected ones.
pub fn xpath(since: Option<&str>, fatal_only: bool) -> String {
    xpath_for(PROVIDER, since, fatal_only)
}

/// The same for a provider of the caller's choosing: only the test source (see `test_source`) is not the real one.
pub fn xpath_for(provider: &str, since: Option<&str>, fatal_only: bool) -> String {
    let mut conditions = vec![format!("Provider[@Name='{provider}']")];
    if fatal_only {
        conditions.push(format!("(EventID={EVENT_FATAL})"));
    }
    if let Some(t) = since {
        conditions.push(format!("TimeCreated[@SystemTime>'{t}']"));
    }
    format!("*[System[{}]]", conditions.join(" and "))
}

/// The provider and id of "the system has rebooted without cleanly shutting down first".
pub const POWER_PROVIDER: &str = "Microsoft-Windows-Kernel-Power";
pub const EVENT_UNEXPECTED_SHUTDOWN: u32 = 41;

/// The XPath for the unexpected shutdowns after `since`.
pub fn xpath_unexpected_shutdowns(since: &str) -> String {
    format!(
        "*[System[Provider[@Name='{POWER_PROVIDER}'] and (EventID={EVENT_UNEXPECTED_SHUTDOWN}) and TimeCreated[@SystemTime>'{since}']]]"
    )
}

/// The unexpected shutdown Windows logged just before the fatal checks of `crash`: the last one no more than
/// `window_seconds` before the crash's first check. Windows writes both when it starts again, a few seconds apart.
pub fn shutdown_before<'a>(
    crash: &[WheaEvent],
    shutdowns: &'a [WheaEvent],
    window_seconds: i64,
) -> Option<&'a WheaEvent> {
    let first = crash.iter().filter_map(|e| epoch_seconds(&e.time)).min()?;
    shutdowns
        .iter()
        .filter_map(|s| Some((epoch_seconds(&s.time)?, s)))
        .filter(|(t, _)| *t <= first && first - *t <= window_seconds)
        .max_by_key(|(t, _)| *t)
        .map(|(_, s)| s)
}

/// A second place machine checks are read from, for trying the whole chain without a real one: `Log|Provider`, the
/// name of an event log and of a provider that writes into it. It only ever adds to the real source, so it can make
/// the program see more, never less.
pub fn test_source(text: &str) -> Option<(String, String)> {
    let (log, provider) = text.split_once('|')?;
    let (log, provider) = (log.trim(), provider.trim());
    // They go into a command line and an XPath: plain names only.
    let plain = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
    };
    (plain(log) && plain(provider)).then(|| (log.to_string(), provider.to_string()))
}

/// The text between `<tag ...>` and `</tag>` in `s`, and what follows the closing tag.
fn element<'a>(s: &'a str, tag: &str) -> Option<(&'a str, &'a str)> {
    let open = format!("<{tag}");
    let mut from = 0;
    loop {
        let at = s[from..].find(&open)? + from;
        let after = &s[at + open.len()..];
        // The tag name has to end here: `<Event` must not match `<EventID`.
        match after.chars().next()? {
            '>' | ' ' | '/' | '\t' | '\r' | '\n' => {
                let gt = after.find('>')?;
                if after[..gt].ends_with('/') {
                    return Some(("", &after[gt + 1..]));
                }
                let body = &after[gt + 1..];
                let close = format!("</{tag}>");
                let end = body.find(&close)?;
                return Some((&body[..end], &body[end + close.len()..]));
            }
            _ => from = at + open.len(),
        }
    }
}

/// The value of an attribute inside the first `<tag ...>` of `s`.
fn attribute(s: &str, tag: &str, name: &str) -> Option<String> {
    let at = s.find(&format!("<{tag} "))?;
    let head = &s[at..at + s[at..].find('>')?];
    for quote in ['\'', '"'] {
        let key = format!("{name}={quote}");
        if let Some(k) = head.find(&key) {
            let rest = &head[k + key.len()..];
            return rest.find(quote).map(|e| rest[..e].to_string());
        }
    }
    None
}

fn number(text: &str) -> Option<u32> {
    let t = text.trim();
    match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => t.parse().ok(),
    }
}

/// Every event in a block of event XML (one `<Event>` per record, as `wevtutil qe /f:xml` writes them).
/// Records it cannot read are left out, never guessed.
pub fn parse_events(xml: &str) -> Vec<WheaEvent> {
    let mut events = Vec::new();
    let mut rest = xml;
    while let Some((event, after)) = element(rest, "Event") {
        rest = after;
        let Some((system, _)) = element(event, "System") else {
            continue;
        };
        let Some(id) = element(system, "EventID").and_then(|(t, _)| number(t)) else {
            continue;
        };
        let level = element(system, "Level").and_then(|(t, _)| number(t));
        let Some(time) = attribute(system, "TimeCreated", "SystemTime") else {
            continue;
        };

        // The processor the event names, from the data field named ApicId (any capitalisation).
        let mut apic_id = None;
        if let Some((data, _)) = element(event, "EventData") {
            let mut d = data;
            while let Some((value, after)) = element(d, "Data") {
                let head = &d[..d.len() - after.len()];
                let named_apic = attribute(head, "Data", "Name")
                    .is_some_and(|n| n.to_ascii_lowercase().contains("apic"));
                if named_apic {
                    apic_id = number(value);
                }
                d = after;
            }
        }

        events.push(WheaEvent {
            id,
            time,
            warning: level == Some(3),
            apic_id,
        });
    }
    events
}

/// When the event log named `channel` was last cleared, from the records of event 104 (`wevtutil qe /f:xml`, newest
/// first). Event 104 is written into the System log whenever *any* log is cleared and names that log inside the
/// record, so the records that name another log are not counted.
pub fn cleared_time(xml: &str, channel: &str) -> Option<String> {
    let mut rest = xml;
    while let Some((event, after)) = element(rest, "Event") {
        rest = after;
        let named = element(event, "UserData")
            .and_then(|(u, _)| element(u, "LogFileCleared"))
            .and_then(|(l, _)| element(l, "Channel"))
            .map(|(c, _)| c.trim().to_string());
        if named.is_some_and(|c| c.eq_ignore_ascii_case(channel)) {
            let (system, _) = element(event, "System")?;
            return attribute(system, "TimeCreated", "SystemTime");
        }
    }
    None
}

/// Whose fault an event is, while one core is under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blame {
    /// Not a machine check that counts (other event ids).
    Ignored,
    /// The event names the core under test.
    TestedCore,
    /// The event names another processor: not this test's business.
    OtherCore,
    /// The event names no processor at all. Only one core is loaded, so it most likely came from the core under test, and
    /// missing a failure is the one mistake a stability test cannot afford: it is counted against the tested core.
    Unattributed,
}

/// `named_core` is the core the event's APIC id belongs to, when the id is known and maps to a core.
pub fn blame(event: &WheaEvent, named_core: Option<usize>, tested_core: usize) -> Blame {
    if event.id != EVENT_CORRECTED && event.id != EVENT_FATAL {
        return Blame::Ignored;
    }
    match (event.apic_id, named_core) {
        (None, _) => Blame::Unattributed,
        (Some(_), Some(core)) if core == tested_core => Blame::TestedCore,
        (Some(_), _) => Blame::OtherCore,
    }
}

/// Fatal machine checks among `events`.
pub fn fatal(events: &[WheaEvent]) -> impl Iterator<Item = &WheaEvent> {
    events.iter().filter(|e| e.id == EVENT_FATAL)
}

/// Days since 1970-01-01 of a calendar date (proleptic Gregorian).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The calendar date of a day count since 1970-01-01: the other way round.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Seconds since 1970-01-01 (UTC) of an event time (`2026-11-05T10:00:00.1234567Z`); the fraction is dropped.
pub fn epoch_seconds(time: &str) -> Option<i64> {
    let (date, clock) = time.split_once('T')?;
    let mut d = date.split('-');
    let year: i64 = d.next()?.parse().ok()?;
    let month: i64 = d.next()?.parse().ok()?;
    let day: i64 = d.next()?.parse().ok()?;
    let mut c = clock.trim_end_matches('Z').split(':');
    let hour: i64 = c.next()?.parse().ok()?;
    let minute: i64 = c.next()?.parse().ok()?;
    let second: i64 = c.next()?.split('.').next()?.parse().ok()?;
    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && (0..24).contains(&hour)
        && (0..60).contains(&minute)
        && (0..61).contains(&second);
    valid.then(|| days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

/// The event time (`2026-11-05T10:00:00.000Z`) of a number of seconds since 1970-01-01, for a query.
pub fn iso_from_epoch(seconds: i64) -> String {
    let (y, m, d) = civil_from_days(seconds.div_euclid(86_400));
    let s = seconds.rem_euclid(86_400);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z",
        s / 3600,
        s % 3600 / 60,
        s % 60
    )
}

/// The fatal machine checks grouped by crash, oldest crash first. When Windows starts again after a crash it writes
/// the fatal checks of that crash together, one for each processor that reported, so fatal checks no more than
/// `window_seconds` apart belong to one crash.
pub fn crashes(events: &[WheaEvent], window_seconds: i64) -> Vec<Vec<WheaEvent>> {
    let mut timed: Vec<(i64, &WheaEvent)> = fatal(events)
        .filter_map(|e| Some((epoch_seconds(&e.time)?, e)))
        .collect();
    timed.sort_by_key(|(t, _)| *t);
    let mut groups: Vec<Vec<WheaEvent>> = Vec::new();
    let mut last = i64::MIN;
    for (t, e) in timed {
        match groups.last_mut() {
            Some(group) if t - last <= window_seconds => group.push(e.clone()),
            _ => groups.push(vec![e.clone()]),
        }
        last = t;
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System>\
<Provider Name='Microsoft-Windows-WHEA-Logger' Guid='{c26c4f3c-3f66-4e99-8f8a-39405cfed220}'/>\
<EventID>19</EventID><Version>0</Version><Level>3</Level><Task>0</Task>\
<TimeCreated SystemTime='2026-11-05T10:00:00.1234567Z'/><EventRecordID>1</EventRecordID></System>\
<EventData><Data Name='ErrorSource'>3</Data><Data Name='ApicId'>6</Data><Data Name='MCABank'>5</Data></EventData></Event>";

    #[test]
    fn reads_id_level_time_and_apic() {
        let e = parse_events(ONE);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].id, 19);
        assert!(e[0].warning);
        assert_eq!(e[0].time, "2026-11-05T10:00:00.1234567Z");
        assert_eq!(e[0].apic_id, Some(6));
    }

    #[test]
    fn eventid_is_not_mistaken_for_event() {
        // `<EventID>` and `<EventData>` begin with `<Event`; only the real element may match.
        let e = parse_events(&format!("{ONE}{ONE}"));
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn a_record_without_an_id_is_left_out() {
        let broken =
            "<Event><System><TimeCreated SystemTime='2026-11-05T10:00:00Z'/></System></Event>";
        assert!(parse_events(broken).is_empty());
    }

    #[test]
    fn who_is_blamed_while_a_core_is_under_test() {
        let ev = |id, apic| WheaEvent {
            id,
            time: String::new(),
            warning: true,
            apic_id: apic,
        };
        assert_eq!(blame(&ev(19, Some(4)), Some(2), 2), Blame::TestedCore);
        assert_eq!(blame(&ev(19, Some(8)), Some(4), 2), Blame::OtherCore);
        // An id no core owns names another processor, not the tested one.
        assert_eq!(blame(&ev(19, Some(99)), None, 2), Blame::OtherCore);
        // No id at all: counted against the tested core, never waved through.
        assert_eq!(blame(&ev(19, None), None, 2), Blame::Unattributed);
        assert_eq!(blame(&ev(18, None), None, 2), Blame::Unattributed);
        // Events that are not machine checks never count.
        assert_eq!(blame(&ev(17, None), None, 2), Blame::Ignored);
    }

    /// Two crashes of one machine, shaped after a second-hand description of a log that no longer exists (a summary,
    /// not an export, so the field names and numbers are unconfirmed): each time two fatal checks written within
    /// seconds of each other, one naming APIC id 8 or 9 (a thread of core 4) and one naming APIC id 0.
    fn crash_xml(id: u32, time: &str, apic: u32, bank: u32) -> String {
        format!(
            "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System>\
<Provider Name='Microsoft-Windows-WHEA-Logger' Guid='{{c26c4f3c-3f66-4e99-8f8a-39405cfed220}}'/>\
<EventID>{id}</EventID><Version>0</Version><Level>1</Level><Task>0</Task>\
<TimeCreated SystemTime='{time}'/><EventRecordID>7</EventRecordID></System>\
<EventData><Data Name='ErrorSource'>0</Data><Data Name='ApicId'>{apic}</Data><Data Name='MCABank'>{bank}</Data>\
<Data Name='MciStat'>0xbaa0000000060150</Data><Data Name='MemHierarchyLvl'>3</Data></EventData></Event>"
        )
    }

    fn two_crashes() -> Vec<WheaEvent> {
        let xml = [
            crash_xml(18, "2026-08-06T13:14:06.0000000Z", 8, 5),
            crash_xml(18, "2026-08-06T13:14:06.5000000Z", 0, 22),
            crash_xml(18, "2026-09-20T11:52:22.0000000Z", 9, 5),
            crash_xml(18, "2026-09-20T11:52:22.2000000Z", 0, 22),
            // A corrected check on its own is not a crash.
            crash_xml(19, "2026-09-01T09:00:00.0000000Z", 8, 5),
        ]
        .concat();
        parse_events(&xml)
    }

    #[test]
    fn fatal_checks_of_one_boot_are_one_crash_and_name_both_processors() {
        let events = two_crashes();
        assert_eq!(events.len(), 5);
        // Level 1 (critical) is not a warning: these are the hard errors.
        assert!(events.iter().filter(|e| e.id == 18).all(|e| !e.warning));
        let crashes = crashes(&events, 120);
        assert_eq!(crashes.len(), 2);
        let ids = |g: &Vec<WheaEvent>| g.iter().map(|e| e.apic_id).collect::<Vec<_>>();
        assert_eq!(ids(&crashes[0]), vec![Some(8), Some(0)]);
        assert_eq!(ids(&crashes[1]), vec![Some(9), Some(0)]);
    }

    #[test]
    fn the_apic_ids_of_such_a_crash_belong_to_cores_four_and_zero() {
        // Eight cores of two threads each, the ids being the logical processors' numbers (this part's habit).
        let cores: Vec<crate::apic::Core> = (0..8)
            .map(|c| crate::apic::Core {
                index: c,
                logical_processors: vec![c as i32 * 2, c as i32 * 2 + 1],
            })
            .collect();
        let named = |apic| crate::apic::core_for_apic_id(apic, &cores, None);
        assert_eq!(named(8), Some(4));
        assert_eq!(named(9), Some(4));
        assert_eq!(named(0), Some(0));
    }

    #[test]
    fn checks_far_apart_are_different_crashes_and_a_corrected_one_is_none() {
        let events = two_crashes();
        assert_eq!(crashes(&events, 120).len(), 2);
        // A window as wide as the gap joins them.
        assert_eq!(crashes(&events, 60 * 60 * 24 * 60).len(), 1);
        assert!(crashes(&[], 120).is_empty());
    }

    #[test]
    fn event_times_and_epoch_seconds_agree() {
        assert_eq!(epoch_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            epoch_seconds("2026-11-05T10:00:00.1234567Z"),
            Some(1_793_872_800)
        );
        assert_eq!(epoch_seconds("2024-02-29T23:59:59.9Z"), Some(1_709_251_199));
        assert_eq!(epoch_seconds("not a time"), None);
        assert_eq!(epoch_seconds("2026-13-05T10:00:00Z"), None);
        for t in [0, 1_709_251_199, 1_793_872_800, 4_102_444_800] {
            assert_eq!(epoch_seconds(&iso_from_epoch(t)), Some(t));
        }
        assert_eq!(iso_from_epoch(1_793_872_800), "2026-11-05T10:00:00.000Z");
    }

    /// Two real records of event 104 (names of the user and the computer removed): the newest is the removal of some
    /// other log, the one before it the clearing of the System log itself.
    const CLEARED: &str = "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System><Provider Name='Microsoft-Windows-Eventlog' Guid='{fc65ddd8-d6ef-4962-83d5-6e5cfe9ce148}'/><EventID>104</EventID><Version>1</Version><Level>4</Level><Task>104</Task><Opcode>0</Opcode><Keywords>0x8000000000000000</Keywords><TimeCreated SystemTime='2026-11-06T12:28:42.3115763Z'/><EventRecordID>22525</EventRecordID><Correlation/><Execution ProcessID='1536' ThreadID='19444'/><Channel>System</Channel><Computer>PC</Computer><Security UserID='S-1-5-21-1-2-3-1001'/></System><UserData><LogFileCleared xmlns='http://manifests.microsoft.com/win/2004/08/windows/eventlog'><SubjectUserName>user</SubjectUserName><SubjectDomainName>PC</SubjectDomainName><Channel>SomeOtherLog</Channel><BackupPath></BackupPath><ClientProcessId>11692</ClientProcessId><ClientProcessStartKey>33495522228652750</ClientProcessStartKey></LogFileCleared></UserData></Event>\n\
<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'><System><Provider Name='Microsoft-Windows-Eventlog' Guid='{fc65ddd8-d6ef-4962-83d5-6e5cfe9ce148}'/><EventID>104</EventID><Version>1</Version><Level>4</Level><Task>104</Task><Opcode>0</Opcode><Keywords>0x8000000000000000</Keywords><TimeCreated SystemTime='2026-11-05T19:37:46.8004781Z'/><EventRecordID>22458</EventRecordID><Correlation/><Execution ProcessID='1536' ThreadID='8136'/><Channel>System</Channel><Computer>PC</Computer><Security UserID='S-1-5-21-1-2-3-1001'/></System><UserData><LogFileCleared xmlns='http://manifests.microsoft.com/win/2004/08/windows/eventlog'><SubjectUserName>user</SubjectUserName><SubjectDomainName>PC</SubjectDomainName><Channel>System</Channel><BackupPath></BackupPath><ClientProcessId>20532</ClientProcessId><ClientProcessStartKey>33495522228621076</ClientProcessStartKey></LogFileCleared></UserData></Event>";

    #[test]
    fn the_clearing_of_another_log_is_not_the_clearing_of_the_system_log() {
        assert_eq!(
            cleared_time(CLEARED, "System").as_deref(),
            Some("2026-11-05T19:37:46.8004781Z")
        );
        assert_eq!(
            cleared_time(CLEARED, "SomeOtherLog").as_deref(),
            Some("2026-11-06T12:28:42.3115763Z")
        );
        assert_eq!(cleared_time(CLEARED, "Application"), None);
        assert_eq!(cleared_time("", "System"), None);
    }

    #[test]
    fn the_shutdown_that_belongs_to_a_crash_is_the_last_one_shortly_before_it() {
        let ev = |id, time: &str| WheaEvent {
            id,
            time: time.to_string(),
            warning: false,
            apic_id: None,
        };
        let crash = vec![ev(18, "2026-08-06T13:14:06Z")];
        let shutdowns = vec![
            ev(41, "2026-08-06T12:00:00Z"),
            ev(41, "2026-08-06T13:14:00Z"),
            // After the crash: not its shutdown.
            ev(41, "2026-08-06T13:14:30Z"),
        ];
        let hit = shutdown_before(&crash, &shutdowns, 300).expect("one belongs to it");
        assert_eq!(hit.time, "2026-08-06T13:14:00Z");
        // Too long before it, no shutdown at all, no crash.
        assert!(shutdown_before(&crash, &shutdowns[..1], 300).is_none());
        assert!(shutdown_before(&crash, &[], 300).is_none());
        assert!(shutdown_before(&[], &shutdowns, 300).is_none());
        assert_eq!(
            xpath_unexpected_shutdowns("2026-01-01T00:00:00.000Z"),
            "*[System[Provider[@Name='Microsoft-Windows-Kernel-Power'] and (EventID=41) and TimeCreated[@SystemTime>'2026-01-01T00:00:00.000Z']]]"
        );
    }

    #[test]
    fn the_test_source_takes_plain_names_only() {
        assert_eq!(
            test_source("PboStudioTest|PboStudioTest"),
            Some(("PboStudioTest".to_string(), "PboStudioTest".to_string()))
        );
        assert_eq!(test_source("no separator"), None);
        assert_eq!(test_source("|Provider"), None);
        assert_eq!(test_source("Log|Prov'ider"), None);
        assert_eq!(test_source("Log|a]]]"), None);
        assert!(xpath_for("PboStudioTest", None, false).contains("@Name='PboStudioTest'"));
    }

    #[test]
    fn the_query_asks_for_what_it_says() {
        assert_eq!(
            xpath(None, false),
            "*[System[Provider[@Name='Microsoft-Windows-WHEA-Logger']]]"
        );
        assert_eq!(
            xpath(Some("2026-11-05T10:00:00Z"), true),
            "*[System[Provider[@Name='Microsoft-Windows-WHEA-Logger'] and (EventID=18) and TimeCreated[@SystemTime>'2026-11-05T10:00:00Z']]]"
        );
    }
}
