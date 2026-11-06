//! "Ready for the night?": a list that changes nothing and says what is missing before a long run (ux.md, block 1).
//!
//! A long run is likely to crash the machine at least once (that is what the search is for). What happens next decides
//! whether the person finds a finished result in the morning or a logon screen: without the logon task nothing carries
//! on, and without automatic sign-in the machine stops at the logon screen and the task never runs. The program does not
//! set up automatic sign-in (it is a security setting); it says so. Tests: the unit tests below.

/// From this many hours on a run counts as long, and the checks that only matter for long runs apply.
pub const LONG_RUN_HOURS: f64 = 1.0;
/// Less free space than this on the drive of the log folder is a warning.
pub const MIN_FREE_DISK_MB: u64 = 500;
/// Less distance than this (in degrees) between the idle temperature and the limit is a warning.
pub const MIN_TEMPERATURE_HEADROOM: f32 = 25.0;

#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// The longest the planned run can take.
    pub run_hours: f64,
    pub logon_task_installed: bool,
    /// Whether Windows signs the user in by itself. `None` when it could not be read.
    pub auto_login: Option<bool>,
    /// Minutes of inactivity on mains power before the machine sleeps; 0 means never. `None` when unknown.
    pub sleep_minutes_on_mains: Option<u32>,
    pub free_disk_mb: Option<u64>,
    pub temperature_now: Option<f32>,
    pub temperature_limit: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    LogonTask,
    AutoLogin,
    Sleep,
    Disk,
    Temperature,
}

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::LogonTask => "LogonTask",
            Item::AutoLogin => "AutoLogin",
            Item::Sleep => "Sleep",
            Item::Disk => "Disk",
            Item::Temperature => "Temperature",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Ok,
    /// Something should be done before a long run.
    Warn,
    /// It could not be read; nothing is claimed.
    Unknown,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Ok => "Ok",
            Level::Warn => "Warn",
            Level::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Check {
    pub item: Item,
    pub level: Level,
}

pub fn evaluate(i: &Inputs) -> Vec<Check> {
    let long = i.run_hours >= LONG_RUN_HOURS;

    let logon_task = if i.logon_task_installed || !long {
        Level::Ok
    } else {
        Level::Warn
    };
    let auto_login = match i.auto_login {
        Some(true) => Level::Ok,
        Some(false) if long => Level::Warn,
        Some(false) => Level::Ok,
        None => Level::Unknown,
    };
    let sleep = match i.sleep_minutes_on_mains {
        Some(0) => Level::Ok,
        Some(_) if long => Level::Warn,
        Some(_) => Level::Ok,
        None => Level::Unknown,
    };
    let disk = match i.free_disk_mb {
        Some(mb) if mb < MIN_FREE_DISK_MB => Level::Warn,
        Some(_) => Level::Ok,
        None => Level::Unknown,
    };
    let temperature = match (i.temperature_now, i.temperature_limit) {
        (Some(now), Some(limit)) if limit - now < MIN_TEMPERATURE_HEADROOM => Level::Warn,
        (Some(_), Some(_)) => Level::Ok,
        _ => Level::Unknown,
    };

    vec![
        Check {
            item: Item::LogonTask,
            level: logon_task,
        },
        Check {
            item: Item::AutoLogin,
            level: auto_login,
        },
        Check {
            item: Item::Sleep,
            level: sleep,
        },
        Check {
            item: Item::Disk,
            level: disk,
        },
        Check {
            item: Item::Temperature,
            level: temperature,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(checks: &[Check], item: Item) -> Level {
        checks.iter().find(|c| c.item == item).unwrap().level
    }

    #[test]
    fn a_long_run_without_the_logon_task_warns_and_a_short_one_does_not() {
        let long = evaluate(&Inputs {
            run_hours: 8.0,
            ..Default::default()
        });
        assert_eq!(level(&long, Item::LogonTask), Level::Warn);
        let short = evaluate(&Inputs {
            run_hours: 0.2,
            ..Default::default()
        });
        assert_eq!(level(&short, Item::LogonTask), Level::Ok);
    }

    #[test]
    fn without_automatic_sign_in_a_long_run_stops_at_the_logon_screen() {
        let c = evaluate(&Inputs {
            run_hours: 8.0,
            auto_login: Some(false),
            logon_task_installed: true,
            ..Default::default()
        });
        assert_eq!(level(&c, Item::AutoLogin), Level::Warn);
        assert_eq!(level(&c, Item::LogonTask), Level::Ok);
    }

    #[test]
    fn what_could_not_be_read_is_unknown_and_never_ok() {
        let c = evaluate(&Inputs {
            run_hours: 8.0,
            ..Default::default()
        });
        for item in [Item::AutoLogin, Item::Sleep, Item::Disk, Item::Temperature] {
            assert_eq!(level(&c, item), Level::Unknown, "{item:?}");
        }
    }

    #[test]
    fn sleep_disk_and_temperature_have_their_thresholds() {
        let c = evaluate(&Inputs {
            run_hours: 8.0,
            sleep_minutes_on_mains: Some(30),
            free_disk_mb: Some(100),
            temperature_now: Some(70.0),
            temperature_limit: Some(90.0),
            ..Default::default()
        });
        assert_eq!(level(&c, Item::Sleep), Level::Warn);
        assert_eq!(level(&c, Item::Disk), Level::Warn);
        assert_eq!(level(&c, Item::Temperature), Level::Warn);
        let ok = evaluate(&Inputs {
            run_hours: 8.0,
            sleep_minutes_on_mains: Some(0),
            free_disk_mb: Some(900),
            temperature_now: Some(40.0),
            temperature_limit: Some(90.0),
            ..Default::default()
        });
        assert_eq!(level(&ok, Item::Sleep), Level::Ok);
        assert_eq!(level(&ok, Item::Disk), Level::Ok);
        assert_eq!(level(&ok, Item::Temperature), Level::Ok);
    }
}
