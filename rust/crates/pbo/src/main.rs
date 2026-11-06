//! `pbo`: the PboStudio command line.
//!
//! `pbo diag` reads what this machine reports and writes a text report that can be attached to an issue. It is
//! read-only. `pbo status` shows the Curve Optimizer values and the live readings. `pbo selftest` writes each core's
//! own value back and reads it, which changes nothing when the write path works. `pbo run` tests the cores at the
//! values they hold. `pbo tune` searches each core's value. Everything that writes a *different* value goes through
//! [`machine::Machine::set_margin`] and its read-back.

#![cfg(windows)]

mod autorun;
mod commands;
mod dailylog;
mod diag;
mod eventlog;
mod gui;
mod hash;
mod helper;
mod helperlink;
mod history;
mod machine;
mod pulse;
mod ready;
mod report;
mod session;
mod setup;
mod store;
mod stress;
mod update;

use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        // Started without a command (a double click): the window. It needs the rights to reach the processor, so it asks for
        // them once, here, and carries on as the copy that has them. Declined (or with PBO_NO_ELEVATE set) it opens without
        // them and offers the helper.
        None => {
            if !machine::is_elevated()
                && std::env::var_os("PBO_NO_ELEVATE").is_none()
                && pbo_os::relaunch_elevated()
            {
                std::process::exit(0);
            }
            gui::run()
        }
        Some("diag") => diag::run(&args[1..]),
        Some("gui-preview") => gui::preview(&args[1..]),
        Some("status") => status(),
        Some("selftest") => selftest(args.iter().any(|a| a == "--step")),
        Some("run") => commands::run(&args[1..]),
        Some("tune") => commands::tune(&args[1..]),
        Some("resume") => commands::resume(&args[1..]),
        Some("knowledge") => commands::show_knowledge(),
        Some("whea") => history::run(&args[1..]),
        Some("auto") => autorun::auto(&args[1..]),
        Some("report") => report::run(&args[1..]),
        Some("set") => commands::set(&args[1..]),
        Some("apply") => commands::apply(),
        Some("save") => commands::save(),
        Some("update") => update::run(&args[1..]),
        Some("estimate") => commands::estimate_cmd(&args[1..]),
        Some("setup") => setup::run(&args[1..]),
        Some("ready") => ready::run(&args[1..]),
        Some("hash") => hash::hash(&args[1..]),
        Some("verify") => hash::verify(&args[1..]),
        Some("version" | "--version" | "-V") => hash::version(),
        Some("plan") => commands::plan(&args[1..]),
        Some("helper" | "--helper") => helper::run(&args[1..]),
        Some("helper-call") => helper::call(&args[1..]),
        Some("helper-watch") => helper::watch(&args[1..]),
        Some("import") => commands::import(&args[1..]),
        Some("autostart") => commands::autostart(&args[1..]),
        // Hidden: the busy child the job-object check starts.
        Some("burn") => burn(&args[1..]),
        Some("help" | "--help" | "-h" | "/?") | Some(_) => {
            help();
            2
        }
    };
    std::process::exit(code);
}

fn help() {
    eprintln!("pbo {}", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!(
        "  pbo diag [--raw]     read this machine and write pbo-diag.txt next to the program"
    );
    eprintln!(
        "  pbo status           the Curve Optimizer value of every core and the live readings"
    );
    eprintln!(
        "  pbo selftest         write each core's own value back and read it (changes nothing)"
    );
    eprintln!("  pbo selftest --step  also move each core one step toward zero and put it back");
    eprintln!("  pbo run              test the cores at the values they hold (changes nothing on the processor)");
    eprintln!("  pbo tune             search each core's Curve Optimizer value; ends with the result applied");
    eprintln!(
        "  pbo resume           after a crash: put every core back to what it was proven to hold"
    );
    eprintln!("  pbo knowledge        what the program remembers about each core");
    eprintln!("  pbo whea [--days N]  the machine checks Windows has logged, by crash and by core");
    eprintln!("  pbo auto [--intensity fast|normal|thorough]   the window's Auto without a window: all three phases, one after the other");
    eprintln!("  pbo report           one file (pbo-report.txt) to attach to an issue");
    eprintln!("  pbo set CORE VALUE   set one core's value (checked, read back)");
    eprintln!("  pbo import [FOLDER]   take over what a 1.x install kept (memory, profile, logs)");
    eprintln!("  pbo helper [--pipe NAME]   serve the processor to a window or module without rights, over a pipe");
    eprintln!("  pbo helper-call NAME MESSAGE   send one message to a helper");
    eprintln!("  pbo plan new|show|command|run FILE   the plan as a file you can edit and share");
    eprintln!("  pbo hash FILE...     SHA-256 of files; pbo verify FILE SHA256 compares; pbo version shows this program's");
    eprintln!("  pbo ready [--hours N]   ready for the night? a list that changes nothing");
    eprintln!("  pbo setup status|prime95|ycruncher|all   fetch the test programs (https, vendor domain, SHA-256 checked)");
    eprintln!("  pbo estimate [--cores ..] [--guardband N] [--intensity fast|normal|thorough]   how long the campaign still has to run, phase by phase");
    eprintln!("  pbo update [--beta]   is there a newer release? (only looks; never replaces the program)");
    eprintln!("  pbo save             keep the values the cores hold now as the profile for logon");
    eprintln!("  pbo autostart on|off|status   the task that runs 'pbo resume' at logon");
    eprintln!(
        "  pbo apply            write the saved profile (runs/co_saved.json) to the processor"
    );
    eprintln!();
    eprintln!("  options of run, tune and resume:");
    eprintln!("    --seconds N        seconds per test (60)");
    eprintln!("    --cores 0,1,2      only these cores");
    eprintln!("    --engine NAME      prime95 (default) or ycruncher");
    eprintln!("    --fft NAME         Prime95 FFT range: Smallest (default), Small, Large, Huge, All, Heavy, HeavyShort, Moderate");
    eprintln!("    --smt              load both threads of a core");
    eprintln!("    --fine             tune: steps of 1 instead of 3");
    eprintln!(
        "    --step N           tune: steps of N counts, 1 to 6 (3 is the default, 1 is --fine)"
    );
    eprintln!("    --guardband N      tune: steps of headroom kept above what held (3)");
    eprintln!("    --start N          tune: where a core that holds nothing begins, N counts below zero (1 to 30)");
    eprintln!("    --pause N          rest N seconds between two cores");
    eprintln!("    --yc-tests A,B     y-cruncher: these tests (BKT, BBP, SFTv4, SNT, SVT, FFTv4, NTT63, N63, VSTv3, VT3)");
    eprintln!("    --yc-memory MB     y-cruncher: memory for each logical processor (default 64)");
    eprintln!("    resume only: --continue (carry Auto on after a crash, after --wait N seconds), --cancel (stop that countdown)");
    eprintln!();
    eprintln!("Run as administrator: the processor registers are only reachable that way.");
}

fn open() -> Result<machine::Machine, i32> {
    machine::Machine::open().map_err(|e| {
        eprintln!("{e}");
        1
    })
}

fn describe(machine: &machine::Machine) {
    println!(
        "{}  ({}, {})",
        machine.brand,
        machine.family.name,
        if machine.family.verified {
            "checked on hardware"
        } else {
            "NOT checked on hardware"
        }
    );
}

fn status() -> i32 {
    let machine = match open() {
        Ok(m) => m,
        Err(c) => return c,
    };
    describe(&machine);
    match machine.read_margins() {
        Ok(values) => {
            let list: Vec<String> = values.iter().map(|v| v.to_string()).collect();
            println!("Curve Optimizer: {}", list.join(" "));
        }
        Err(e) => println!("Curve Optimizer: n/a ({e})"),
    }
    let s = machine.sample();
    let f = |v: Option<f32>, unit: &str| v.map_or("n/a".to_string(), |x| format!("{x:.1} {unit}"));
    println!("Temperature    : {}", f(s.temperature, "°C"));
    println!("PPT            : {}", f(s.ppt_watts, "W"));
    println!(
        "TDC / EDC      : {} / {}",
        f(s.tdc_amps, "A"),
        f(s.edc_amps, "A")
    );
    match s.core_volts {
        Some(v) => println!("Core voltage   : {v:.3} V"),
        None => println!("Core voltage   : n/a (no known source for this family)"),
    }
    let vids: Vec<String> = s
        .core_vids
        .iter()
        .map(|v| v.map_or("n/a".to_string(), |x| format!("{x:.3}")))
        .collect();
    println!("Per-core VID   : {}", vids.join(" "));
    match machine.cppc() {
        Some(cppc) => {
            let q = pbo_core::quality::core_qualities(machine.cores, Some(&cppc));
            let list: Vec<String> = q
                .iter()
                .map(|q| match q.rank {
                    pbo_core::quality::Rank::GoldStar => format!("{} (gold, {})", q.core, q.cppc),
                    pbo_core::quality::Rank::SilverStar => {
                        format!("{} (silver, {})", q.core, q.cppc)
                    }
                    pbo_core::quality::Rank::Standard => format!("{}", q.core),
                })
                .collect();
            println!("CPPC ranking   : {}", list.join(" "));
            let raw: Vec<String> = cppc.iter().map(|v| v.to_string()).collect();
            println!("CPPC raw        : {}", raw.join(" "));
        }
        None => println!("CPPC ranking   : n/a"),
    }
    0
}

/// `pbo selftest`: for every core, write the value it already holds and read it back.
fn selftest(step: bool) -> i32 {
    let machine = match open() {
        Ok(m) => m,
        Err(c) => return c,
    };
    describe(&machine);
    let mut failed = 0;
    for core in 0..machine.cores {
        if step {
            match machine.selftest_step(core) {
                Ok((original, moved, after)) if moved != original && after == original => {
                    println!(
                        "core {core:2}: held {original}, moved to {moved}, restored {after}  ok"
                    )
                }
                Ok((original, moved, after)) => {
                    failed += 1;
                    println!("core {core:2}: held {original}, moved to {moved}, restored {after}  the value did not move")
                }
                Err(e) => {
                    failed += 1;
                    println!("core {core:2}: {e}");
                }
            }
            continue;
        }
        match machine.selftest(core) {
            Ok((before, after)) if before == after => {
                println!("core {core:2}: held {before}, wrote {before}, read back {after}  ok")
            }
            Ok((before, after)) => {
                failed += 1;
                println!(
                    "core {core:2}: held {before}, wrote {before}, read back {after}  MISMATCH"
                );
            }
            Err(e) => {
                failed += 1;
                println!("core {core:2}: {e}");
            }
        }
    }
    if failed == 0 {
        println!("The write path works on this processor.");
        0
    } else {
        println!("{failed} core(s) failed: do not use the write path on this processor.");
        1
    }
}

/// Keeps `threads` threads busy for `seconds`. Used only to check that a job object's affinity holds.
fn burn(args: &[String]) -> i32 {
    let threads: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(2);
    let seconds: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(8);
    let end = Instant::now() + Duration::from_secs(seconds);
    let workers: Vec<_> = (0..threads)
        .map(|_| {
            std::thread::spawn(move || {
                let mut x = 1u64;
                while Instant::now() < end {
                    for _ in 0..1_000_000 {
                        x = std::hint::black_box(
                            x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1),
                        );
                    }
                }
                x
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
    0
}
