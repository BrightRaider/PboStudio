//! `pbo diag`: read this machine and report it. Read-only.
//!
//! Each section says what it read, or why it could not. A number that cannot be trusted is never printed
//! as a number: it is "n/a" with the reason (invariant I12).

use std::time::Duration;

use pbo_core::{coremap, cpu, margin, pm, pstate, topology, voltage};
use pbo_os as os;
use pbo_smu::{AmdFamily17, RyzenSmu};
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

struct Report {
    text: String,
    problems: u32,
}

impl Report {
    fn line(&mut self, s: impl AsRef<str>) {
        println!("{}", s.as_ref());
        self.text.push_str(s.as_ref());
        self.text.push('\n');
    }

    fn section(&mut self, title: &str) {
        self.line("");
        self.line(format!("== {title}"));
    }

    /// Something that should have worked did not.
    fn problem(&mut self, s: impl AsRef<str>) {
        self.problems += 1;
        self.line(format!("  PROBLEM: {}", s.as_ref()));
    }
}

fn hex(v: u64) -> String {
    format!("0x{v:X}")
}

pub fn run(args: &[String]) -> i32 {
    let raw = args.iter().any(|a| a == "--raw");
    let mut r = Report {
        text: String::new(),
        problems: 0,
    };

    r.line(format!(
        "pbo diag {}  (stage E0, read-only: nothing is written to the processor)",
        env!("CARGO_PKG_VERSION")
    ));
    let elevated = unsafe { IsUserAnAdmin() } != 0;
    r.line(format!(
        "Elevated: {}",
        if elevated {
            "yes"
        } else {
            "no, so the hardware sections will be skipped"
        }
    ));

    // ── The processor, from CPUID ─────────────────────────────────
    r.section("Processor (CPUID)");
    let id = cpu::Id::decode(os::cpuid(1, 0)[0]);
    let (generation, verified) = id.generation();
    r.line(format!("Name        : {}", os::brand_string()));
    r.line(format!(
        "Family/model: {:#X} / {:#X}, stepping {}   -> {}{}",
        id.family,
        id.model,
        id.stepping,
        generation,
        if verified {
            "  (checked on hardware)"
        } else {
            "  (NOT checked on hardware)"
        }
    ));
    let logical = os::logical_processors();
    let tpc = os::threads_per_core().max(1);
    let cores = logical / tpc as usize;
    r.line(format!(
        "Topology    : {logical} logical processors, {tpc} thread(s) per core, {cores} cores"
    ));

    // ── APIC ids ───────────────────────────────────────────────────
    r.section("APIC ids (CPUID leaf 0xB, thread pinned to each processor)");
    let apic = os::apic_ids();
    let mut line = String::new();
    for (i, a) in apic.iter().enumerate() {
        match a {
            Some(v) => line.push_str(&format!("{i}:{v}  ")),
            None => line.push_str(&format!("{i}:n/a  ")),
        }
    }
    r.line(format!("  {}", line.trim_end()));
    let missing = apic.iter().filter(|a| a.is_none()).count();
    if missing > 0 {
        r.problem(format!(
            "{missing} processor(s) could not be pinned, so their APIC id is unknown"
        ));
    }

    // ── Job object ─────────────────────────────────────────────────
    r.section("Child process in a job object with a restricted affinity");
    let target = if logical > 2 { 2 } else { 0 };
    match std::env::current_exe() {
        Ok(exe) => match os::Child::start_in_job(&exe, "burn 2 6", 1usize << target) {
            Ok(child) => {
                std::thread::sleep(Duration::from_millis(500));
                let load = child.measure_load(Duration::from_secs(3));
                r.line(format!(
                    "  child pid {} with 2 busy threads, limited to logical processor {target}: used {load:.2} cores",
                    child.pid
                ));
                if load > 1.15 {
                    r.problem("the child used more than one core: the affinity limit did not hold");
                } else if load < 0.5 {
                    r.problem("the child used far less than one core: it did not run, or the measurement failed");
                } else {
                    r.line("  affinity holds (about one core although two threads wanted to run)");
                }
            }
            Err(e) => r.problem(format!("could not start the child: {e}")),
        },
        Err(e) => r.problem(format!("could not find this program: {e}")),
    }

    // ── Hardware ──────────────────────────────────────────────────
    if !elevated {
        r.section("Hardware");
        r.line("  skipped: start the program as administrator to read the SMU, the PM table and the fuses");
        return finish(r, false);
    }

    r.section("Driver (PawnIO)");
    let amd = match AmdFamily17::open() {
        Ok(a) => {
            r.line("  module AMDFamily17 loaded");
            Some(a)
        }
        Err(e) => {
            r.problem(format!("AMDFamily17: {e}"));
            None
        }
    };
    let smu = match RyzenSmu::open() {
        Ok(s) => {
            r.line("  module RyzenSMU loaded");
            Some(s)
        }
        Err(e) => {
            r.problem(format!("RyzenSMU: {e}"));
            None
        }
    };

    if let Some(smu) = &smu {
        r.section("SMU and PM table");
        match smu.version() {
            Some(v) => r.line(format!("  SMU version      : {:#010X}", v)),
            None => r.problem("SMU version could not be read"),
        }
        match smu.resolve_pm_table() {
            Some((version, base)) => {
                r.line(format!(
                    "  PM table version : {:#010X}   at DRAM {}",
                    version,
                    hex(base)
                ));
                let size = pm::table_size(version);
                let bytes = size.unwrap_or(0x994);
                if size.is_none() {
                    r.line(format!("  table size       : unknown for this version, reading {bytes:#X} bytes (the library's default)"));
                } else {
                    r.line(format!("  table size       : {bytes:#X} bytes"));
                }
                match smu.read_pm_table(bytes) {
                    Some(table) => {
                        r.line(format!("  table read       : {} values", table.len()));
                        match pm::layout_for(version) {
                            Some(layout) => {
                                r.line(format!(
                                    "  layout           : {} ({})",
                                    layout.name,
                                    if layout.verified { "checked on hardware" } else { "positions from the library, NOT checked on hardware" }
                                ));
                                match layout.read(&table) {
                                    Some(s) => {
                                        let f = |v: Option<f32>, unit: &str| match v {
                                            Some(x) => format!("{x:.1} {unit}"),
                                            None => "n/a".to_string(),
                                        };
                                        r.line(format!("  temperature      : {}   (limit {})", f(s.temp_now, "°C"), f(s.temp_limit, "°C")));
                                        r.line(format!("  PPT              : {}   (limit {})", f(s.ppt_now, "W"), f(s.ppt_limit, "W")));
                                        r.line(format!("  TDC              : {}   (limit {})", f(s.tdc_now, "A"), f(s.tdc_limit, "A")));
                                        r.line(format!("  EDC              : {}   (limit {})", f(s.edc_now, "A"), f(s.edc_limit, "A")));
                                    }
                                    None => r.problem("the table is too short for its layout"),
                                }
                            }
                            None => r.line("  layout           : none known for this version, so no readings are shown (never a guess)"),
                        }
                        if raw {
                            r.line("  -- whole table (index: value) --");
                            for (i, v) in table.iter().enumerate() {
                                r.line(format!("  {i:4}: {v}"));
                            }
                        }
                    }
                    None => r.problem("the PM table could not be read"),
                }
            }
            None => r.problem("the PM table could not be resolved"),
        }
    }

    // Chiplets and cores from the fuses, then the Curve Optimizer per core.
    let mut slots: Vec<(u32, u32)> = Vec::new();
    if let Some(amd) = &amd {
        r.section("Chiplets and cores (fuses over SMN)");
        let fa = topology::fuse_addresses(id.family, id.model);
        r.line(format!(
            "  fuse registers {} and {}, core map base {}{}",
            hex(fa.ccds_present.into()),
            hex(fa.ccds_down.into()),
            hex(fa.core_disable.into()),
            if verified {
                ""
            } else {
                "   (addresses from the library, NOT checked for this family)"
            }
        ));
        match (amd.read_smn(fa.ccds_present), amd.read_smn(fa.ccds_down)) {
            (Some(present), Some(down)) => {
                let c = topology::chiplets(present, down);
                r.line(format!(
                    "  present {} down {}  -> enable map {:#b}, disable map {:#b}, {} chiplet(s)",
                    hex(present.into()),
                    hex(down.into()),
                    c.enable_map,
                    c.disable_map,
                    c.count
                ));
                let mut maps = Vec::new();
                for ccd in 0..c.count {
                    if c.enable_map & (1 << ccd) == 0 && c.count > 1 {
                        continue;
                    }
                    match amd.read_smn((ccd << 25) + fa.core_disable) {
                        Some(v) => {
                            let map = v & 0xFF;
                            r.line(format!(
                                "  chiplet {ccd}: core disable map {map:#010b} -> {} enabled cores",
                                topology::enabled_cores(map)
                            ));
                            maps.push(map);
                        }
                        None => r.problem(format!(
                            "core disable map of chiplet {ccd} could not be read"
                        )),
                    }
                }
                match coremap::build(&maps, cores) {
                    Some(map) => {
                        slots = map.iter().map(|s| (s.ccd, s.slot)).collect();
                        let list: Vec<String> = map.iter().map(|s| format!("{}.{}", s.ccd, s.slot)).collect();
                        r.line(format!("  SMU slots (chiplet.slot) in order: {}", list.join(" ")));
                    }
                    None => r.problem(format!(
                        "the fuses do not add up to the {cores} cores CPUID reports: the mapping is refused, not guessed"
                    )),
                }
            }
            _ => r.problem("the chiplet fuses could not be read"),
        }
    }

    if let (Some(smu), true) = (&smu, !slots.is_empty()) {
        r.section("Curve Optimizer (read from the SMU, per core)");
        if let Some(message) =
            pbo_core::families::get_margin_message(pbo_core::families::lookup(id).generation)
        {
            r.line(format!(
                "  message {message:#X}{}",
                if verified {
                    ""
                } else {
                    "   (the library's number for this generation, NOT checked on hardware)"
                }
            ));
            let mut values = Vec::new();
            for (n, (ccd, slot)) in slots.iter().enumerate() {
                let mask = margin::core_mask(id.family, *ccd, *slot);
                match smu.send_command(message, [mask, 0, 0, 0, 0, 0]) {
                    Ok(answer) => {
                        let m = margin::decode(answer[0]);
                        values.push(m.to_string());
                        r.line(format!(
                            "  core {n:2} (chiplet {ccd}, slot {slot}, mask {}): {m}",
                            hex(mask.into())
                        ));
                    }
                    Err(e) => r.problem(format!("core {n}: {e}")),
                }
            }
            r.line(format!("  all: {}", values.join(" ")));
        } else {
            r.line("  not implemented for this family in E0");
        }
    }

    // The effective clock: APERF and MPERF on each core over a short interval, times the rate MPERF counts at.
    if let Some(amd) = &amd {
        r.section("Clock (APERF / MPERF)");
        let tsc = os::tsc_mhz();
        r.line(format!("  time stamp counter: {tsc:.0} MHz (measured)"));
        let read = |core: usize| {
            let l = core * tpc as usize;
            Some((
                amd.read_msr_on(l, 0xC000_00E8)?,
                amd.read_msr_on(l, 0xC000_00E7)?,
            ))
        };
        let before: Vec<_> = (0..cores).map(read).collect();
        std::thread::sleep(Duration::from_millis(200));
        let after: Vec<_> = (0..cores).map(read).collect();
        let mut parts = Vec::new();
        for (core, (b, a)) in before.iter().zip(&after).enumerate() {
            match (b, a) {
                (Some((ab, mb)), Some((aa, ma))) if ma > mb => {
                    let mhz = tsc * (aa.wrapping_sub(*ab)) as f64 / (ma.wrapping_sub(*mb)) as f64;
                    parts.push(format!("c{core}: {mhz:.0}"));
                }
                _ => parts.push(format!("c{core}: n/a")),
            }
        }
        r.line(format!("  effective MHz: {}", parts.join(" | ")));
        r.line("  (n/a means the driver module does not let this program read those registers)");
    }

    // Core voltage: the SVI2 telemetry plane over SMN, and each core's own VID from the P-state MSR.
    if let Some(amd) = &amd {
        r.section("Core voltage");
        let planes = match (id.family, id.model) {
            (0x19, 0x21) => Some((0x5A010u32, 0x5A00Cu32)),
            _ => None,
        };
        match planes {
            Some((core_plane, soc_plane)) => {
                r.line(format!(
                    "  SVI2 telemetry: core plane {}, SoC plane {}",
                    hex(core_plane.into()),
                    hex(soc_plane.into())
                ));
                for sample in 0..3 {
                    let line = match (amd.read_smn(core_plane), amd.read_smn(soc_plane)) {
                        (Some(c), Some(s)) => format!(
                            "  sample {sample}: core {:.3} V (raw {:#010X})   SoC {:.3} V",
                            voltage::svi2(voltage::svi2_telemetry_code(c)),
                            c,
                            voltage::svi2(voltage::svi2_telemetry_code(s))
                        ),
                        _ => "  sample: SMN read failed".to_string(),
                    };
                    r.line(line);
                    // Per-core VID from MSR C001_0293 on the first thread of every core.
                    let mut parts = Vec::new();
                    for core in 0..cores {
                        match amd.read_msr_on(core * tpc as usize, pstate::MSR) {
                            Some(v) => {
                                let s = pstate::Status::decode(v as u32);
                                parts.push(format!("c{core}: {:.3}V", voltage::svi2(s.vid)));
                            }
                            None => parts.push(format!("c{core}: n/a")),
                        }
                    }
                    r.line(format!("    per-core VID: {}", parts.join(" | ")));
                    std::thread::sleep(Duration::from_millis(500));
                }
                r.line("  (the clock in the same register is the requested state, not the measured clock, and is not shown)");
            }
            None => {
                r.line("  No telemetry address is known for this family (the library marks Zen 4 unknown and has none for Zen 5).");
                if pbo_core::families::lookup(id).vcore
                    == pbo_core::families::Vcore::Svi3PerCoreOnly
                {
                    // The per-core VID register is architectural, so it is read and decoded with the SVI3 formula.
                    // That formula has not been checked against a measurement on this family: it is labelled so.
                    r.line("  Each core's own VID, MSR C001_0293 decoded with the SVI3 formula (NOT checked on hardware):");
                    for sample in 0..3 {
                        let mut parts = Vec::new();
                        for core in 0..cores {
                            match amd.read_msr_on(core * tpc as usize, pstate::MSR) {
                                Some(v) => {
                                    let s = pstate::Status::decode(v as u32);
                                    parts.push(format!(
                                        "c{core}: {:.3}V (raw vid {})",
                                        voltage::svi3(s.vid),
                                        s.vid
                                    ));
                                }
                                None => parts.push(format!("c{core}: n/a")),
                            }
                        }
                        r.line(format!("    sample {sample}: {}", parts.join(" | ")));
                        std::thread::sleep(Duration::from_millis(500));
                    }
                    r.line("  Please compare these with what another tool shows at the same moment and report both in the issue.");
                } else {
                    r.line("  This is the part testers can help with.");
                }
            }
        }
    }

    finish(r, true)
}

/// Writes the report next to the program and returns the exit code.
fn finish(mut r: Report, hardware: bool) -> i32 {
    r.line("");
    r.line(format!(
        "{} problem(s). {}",
        r.problems,
        if hardware {
            ""
        } else {
            "Hardware sections were skipped: run as administrator for the full report."
        }
    ));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join("pbo-diag.txt");
            match std::fs::write(&path, &r.text) {
                Ok(()) => println!("Report written to {}", path.display()),
                Err(e) => eprintln!("Could not write {}: {e}", path.display()),
            }
        }
    }
    if r.problems > 0 {
        1
    } else {
        0
    }
}
