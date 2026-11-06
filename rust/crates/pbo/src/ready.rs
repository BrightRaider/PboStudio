//! `pbo ready [--hours N]`: "Ready for the night?" A list that changes nothing (ux.md, block 1).
//!
//! The judging is `pbo_core::readiness`; this reads what Windows says (the logon task, automatic sign-in, the sleep
//! timer, free disk space, the temperature) and prints each item in the person's language. What cannot be read is
//! said so and never reported as fine.

use pbo_app::text::{tr, Key, Lang};
use pbo_core::readiness::{self, Inputs, Item, Level};
use pbo_os as os;

use crate::machine::Machine;
use crate::store::Store;

/// Minutes of inactivity on mains power before sleep (0 is never), from the active power scheme. `powercfg` writes its
/// words in the system language, so the value is found by position: the last two lines with a hexadecimal number are
/// the current setting on mains and on battery, in seconds.
fn sleep_minutes_on_mains() -> Option<u32> {
    let out = os::system::run(
        "powercfg.exe",
        &["/q", "SCHEME_CURRENT", "SUB_SLEEP", "STANDBYIDLE"],
        std::time::Duration::from_secs(30),
    )?;
    let text = out.stdout_text();
    let hex: Vec<&str> = text.lines().filter(|l| l.contains("0x")).collect();
    let ac = hex.get(hex.len().checked_sub(2)?)?;
    let value = ac.rsplit("0x").next()?.trim();
    let seconds = u64::from_str_radix(value, 16).ok()?;
    Some(seconds.div_ceil(60) as u32)
}

pub fn logon_task_installed() -> bool {
    os::system::run(
        "schtasks.exe",
        &["/Query", "/TN", crate::commands::TASK_NAME],
        std::time::Duration::from_secs(30),
    )
    .is_some_and(|o| o.success)
}

/// Whether the logon task also carries Auto on after a crash (it was made with `pbo autostart on --continue`).
pub fn logon_task_continues() -> bool {
    os::system::run(
        "schtasks.exe",
        &["/Query", "/TN", crate::commands::TASK_NAME, "/XML"],
        std::time::Duration::from_secs(30),
    )
    .is_some_and(|o| o.success && o.stdout_text().contains("--continue"))
}

pub fn gather(run_hours: f64) -> Inputs {
    let mut inputs = Inputs {
        run_hours,
        logon_task_installed: logon_task_installed(),
        auto_login: os::local_machine_string(
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon",
            "AutoAdminLogon",
        )
        .map(|v| v == "1")
        // The value is simply absent when automatic sign-in was never set up.
        .or(Some(false)),
        sleep_minutes_on_mains: sleep_minutes_on_mains(),
        free_disk_mb: os::free_disk_mb(
            &Store::beside_program()
                .root
                .parent()
                .map_or_else(crate::store::program_dir, |p| p.to_path_buf()),
        ),
        ..Default::default()
    };
    // The temperature needs the processor, so it is only read when this program may.
    if let Ok(machine) = Machine::open() {
        if let Some((version, _)) = machine.smu.resolve_pm_table() {
            if let (Some(table), Some(layout)) = (
                machine
                    .smu
                    .read_pm_table(pbo_core::pm::table_size(version).unwrap_or(0x994)),
                pbo_core::pm::layout_for(version),
            ) {
                if let Some(s) = layout.read(&table) {
                    inputs.temperature_now = s.temp_now;
                    inputs.temperature_limit = s.temp_limit;
                }
            }
        }
    }
    inputs
}

pub fn run(args: &[String]) -> i32 {
    let hours: f64 = args
        .windows(2)
        .find(|w| w[0] == "--hours")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(8.0);
    let lang = if os::ui_language_is_german() {
        Lang::De
    } else {
        Lang::En
    };
    println!("{} ({hours} h)", tr(lang, Key::ReadyTitle));
    let checks = readiness::evaluate(&gather(hours));
    let mut warnings = 0;
    for c in &checks {
        let (label, hint) = match c.item {
            Item::LogonTask => (Key::ReadyLogonTask, Key::ReadyLogonTaskHint),
            Item::AutoLogin => (Key::ReadyAutoLogin, Key::ReadyAutoLoginHint),
            Item::Sleep => (Key::ReadySleep, Key::ReadySleepHint),
            Item::Disk => (Key::ReadyDisk, Key::ReadyDiskHint),
            Item::Temperature => (Key::ReadyTemperature, Key::ReadyTemperatureHint),
        };
        match c.level {
            Level::Ok => println!("  [ok]   {}", tr(lang, label)),
            Level::Warn => {
                warnings += 1;
                println!("  [warn] {}\n         {}", tr(lang, label), tr(lang, hint));
            }
            Level::Unknown => println!(
                "  [?]    {}: {}",
                tr(lang, label),
                tr(lang, Key::ReadyUnknown)
            ),
        }
    }
    i32::from(warnings > 0)
}
