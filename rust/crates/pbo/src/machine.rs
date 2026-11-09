//! The processor this program runs on, opened once: who it is, which SMU slot each core is, and the reads and
//! writes the rest of the program needs. Every write to the processor goes through [`Machine::set_margin`], which
//! checks the range, reads the value back and returns what the processor actually holds (invariants I1 and I14).

use std::time::Duration;

use pbo_core::families::{self, Family, Vcore};
use pbo_core::{coremap, cpu, margin, pm, pstate, topology, voltage};
use pbo_os as os;
use pbo_smu::{AmdFamily17, RyzenSmu};
use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

/// APERF and MPERF of one core, or `None` when the driver module does not let this program read them.
type ClockCounters = Option<(u64, u64)>;

pub struct Machine {
    pub brand: String,
    pub id: cpu::Id,
    pub family: Family,
    pub x3d: bool,
    /// Physical cores Windows lists.
    pub cores: usize,
    pub threads_per_core: usize,
    /// One (chiplet, slot) per core, in core order.
    pub slots: Vec<coremap::Slot>,
    pub amd: AmdFamily17,
    pub smu: RyzenSmu,
    /// APERF and MPERF of each core at the last look, to take the next clock from.
    clock_prev: std::sync::Mutex<Option<Vec<ClockCounters>>>,
}

/// One telemetry reading. A field is `None` when it could not be read or the layout is not known (never a guess).
#[derive(Debug, Clone, Default)]
pub struct Sample {
    pub temperature: Option<f32>,
    pub ppt_watts: Option<f32>,
    pub tdc_amps: Option<f32>,
    pub edc_amps: Option<f32>,
    /// The limits the processor publishes in the same table, where the layout is known.
    pub temp_limit: Option<f32>,
    pub ppt_limit: Option<f32>,
    pub tdc_limit: Option<f32>,
    pub edc_limit: Option<f32>,
    /// The effective clock of the fastest core, and of each core, in MHz: the time stamp rate times APERF/MPERF over
    /// the interval since the last look. Measured, not requested.
    pub clock_mhz: Option<f32>,
    pub core_clocks: Vec<Option<f32>>,
    pub core_volts: Option<f64>,
    /// Each core's own requested voltage, from its P-state MSR.
    pub core_vids: Vec<Option<f64>>,
}

pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

impl Machine {
    pub fn open() -> Result<Machine, String> {
        if !is_elevated() {
            return Err("start the program as administrator: the processor registers are only reachable that way".into());
        }
        let brand = os::brand_string();
        let id = cpu::Id::decode(os::cpuid(1, 0)[0]);
        let family = families::lookup(id);
        let threads_per_core = os::threads_per_core().max(1) as usize;
        let cores = os::logical_processors() / threads_per_core;
        let amd = AmdFamily17::open().map_err(|e| format!("driver module AMDFamily17: {e}"))?;
        let smu = RyzenSmu::open().map_err(|e| format!("driver module RyzenSMU: {e}"))?;

        let fa = topology::fuse_addresses(id.family, id.model);
        let present = amd
            .read_smn(fa.ccds_present)
            .ok_or("the chiplet fuses could not be read")?;
        let down = amd
            .read_smn(fa.ccds_down)
            .ok_or("the chiplet fuses could not be read")?;
        let c = topology::chiplets(present, down);
        let mut maps = Vec::new();
        for ccd in 0..c.count {
            if c.enable_map & (1 << ccd) == 0 && c.count > 1 {
                continue;
            }
            let v = amd
                .read_smn((ccd << 25) + fa.core_disable)
                .ok_or("a core disable map could not be read")?;
            maps.push(v & 0xFF);
        }
        let slots = coremap::build(&maps, cores).ok_or_else(|| {
            format!("the fuses do not add up to the {cores} cores CPUID reports: the mapping is refused, not guessed")
        })?;

        Ok(Machine {
            x3d: brand.to_ascii_uppercase().contains("X3D"),
            brand,
            id,
            family,
            cores,
            threads_per_core,
            slots,
            amd,
            smu,
            clock_prev: std::sync::Mutex::new(None),
        })
    }

    pub fn core_mask(&self, core: usize) -> u32 {
        let s = self.slots[core];
        margin::core_mask(self.id.family, s.ccd, s.slot)
    }

    /// The logical processors that belong to a core (its threads), in order.
    pub fn threads_of(&self, core: usize) -> Vec<usize> {
        (0..self.threads_per_core)
            .map(|t| core * self.threads_per_core + t)
            .collect()
    }

    /// What the processor holds for one core.
    pub fn read_margin(&self, core: usize) -> Result<i32, String> {
        if !self.family.curve_optimizer {
            return Err("this family has no Curve Optimizer".into());
        }
        let mask = self.core_mask(core);
        let message = families::get_margin_message(self.family.generation)
            .ok_or("no read message is known for this generation")?;
        self.smu
            .send_command(message, [mask, 0, 0, 0, 0, 0])
            .map(|a| margin::decode(a[0]))
            .map_err(|e| format!("core {core}: {e}"))
    }

    pub fn read_margins(&self) -> Result<Vec<i32>, String> {
        (0..self.cores).map(|c| self.read_margin(c)).collect()
    }

    /// How the cores rank, best first being the highest: what each reports as its highest performance (CPPC), bits 31:24
    /// of MSR C001_02B0; where that reads nothing, the low byte of C001_02B3 as 1.x read it; and where the processor gives
    /// neither (a 5800X3D reads 0 in both), the ranking Windows takes from the firmware (the scheduling class of each
    /// core, plus one, so that 0 still means "no reading"). `None` on a family without Curve Optimizer, where the
    /// ranking is of no use.
    pub fn cppc(&self) -> Option<Vec<i32>> {
        if !self.family.curve_optimizer {
            return None;
        }
        let read = |msr: u32, value: fn(u64) -> i32| -> Vec<i32> {
            (0..self.cores)
                .map(|core| {
                    self.amd
                        .read_msr_on(core * self.threads_per_core, msr)
                        .map_or(0, value)
                })
                .collect()
        };
        let highest = read(0xC001_02B0, |v| ((v >> 24) & 0xFF) as i32);
        if highest.iter().any(|v| *v != 0) {
            return Some(highest);
        }
        let request = read(0xC001_02B3, |v| (v & 0xFF) as i32);
        if request.iter().any(|v| *v != 0) {
            return Some(request);
        }
        let classes = os::scheduling_classes().unwrap_or_default();
        Some(
            (0..self.cores)
                .map(|core| {
                    classes
                        .get(core * self.threads_per_core)
                        .map_or(0, |c| i32::from(*c) + 1)
                })
                .collect(),
        )
    }

    /// The cores the processor ranks highest (gold and silver). They boost furthest and carry the background work, so
    /// the tuner keeps extra headroom on them. Empty when the processor reports no usable ranking.
    pub fn preferred_cores(&self) -> std::collections::BTreeSet<usize> {
        let cppc = self.cppc();
        pbo_core::quality::core_qualities(self.cores, cppc.as_deref())
            .into_iter()
            .filter(|q| q.rank != pbo_core::quality::Rank::Standard)
            .map(|q| q.core)
            .collect()
    }

    /// The deepest value a write may ask for here, and the shallowest (X3D parts take no positive value).
    pub fn allowed_range(&self) -> (i32, i32) {
        let deepest = self.family.start_depth.min(-30);
        (deepest, if self.x3d { 0 } else { 30 })
    }

    /// Writes one core's margin and reads it back. Returns the value the processor now holds: equal to `value` when
    /// it stuck, anything else when it was clamped or refused (the caller decides what that means).
    pub fn set_margin(&self, core: usize, value: i32) -> Result<i32, String> {
        let (deepest, shallowest) = self.allowed_range();
        if value < deepest || value > shallowest {
            return Err(format!(
                "{value} is outside what this processor may be asked for ({deepest} to {shallowest})"
            ));
        }
        self.smu
            .set_core_margin(self.core_mask(core), value)
            .map_err(|e| format!("core {core}: {e}"))?;
        self.read_margin(core)
    }

    fn read_clock_counters(&self) -> Vec<ClockCounters> {
        (0..self.cores)
            .map(|core| {
                let logical = core * self.threads_per_core;
                Some((
                    self.amd.read_msr_on(logical, 0xC000_00E8)?,
                    self.amd.read_msr_on(logical, 0xC000_00E7)?,
                ))
            })
            .collect()
    }

    /// The effective clock of every core since the last call. The first call measures over a short interval of its own.
    pub fn clocks(&self) -> Vec<Option<f32>> {
        let tsc = os::tsc_mhz();
        let Ok(mut prev) = self.clock_prev.lock() else {
            return vec![None; self.cores];
        };
        let before = match prev.take() {
            Some(p) => p,
            None => {
                let first = self.read_clock_counters();
                std::thread::sleep(Duration::from_millis(50));
                first
            }
        };
        let after = self.read_clock_counters();
        *prev = Some(after.clone());
        before
            .iter()
            .zip(&after)
            .map(|(b, a)| match (b, a) {
                (Some((ab, mb)), Some((aa, ma))) if ma > mb && tsc > 0.0 => {
                    Some((tsc * aa.wrapping_sub(*ab) as f64 / ma.wrapping_sub(*mb) as f64) as f32)
                }
                _ => None,
            })
            .collect()
    }

    /// One telemetry reading: the PM table, the core voltage plane and each core's own VID.
    pub fn sample(&self) -> Sample {
        let mut s = Sample::default();
        if let Some((version, _)) = self.smu.resolve_pm_table() {
            let size = pm::table_size(version).unwrap_or(0x994);
            if let (Some(table), Some(layout)) =
                (self.smu.read_pm_table(size), pm::layout_for(version))
            {
                if let Some(snap) = layout.read(&table) {
                    s.temperature = snap.temp_now;
                    s.ppt_watts = snap.ppt_now;
                    s.tdc_amps = snap.tdc_now;
                    s.edc_amps = snap.edc_now;
                    s.temp_limit = snap.temp_limit;
                    s.ppt_limit = snap.ppt_limit;
                    s.tdc_limit = snap.tdc_limit;
                    s.edc_limit = snap.edc_limit;
                }
            }
        }
        s.core_clocks = self.clocks();
        s.clock_mhz = s
            .core_clocks
            .iter()
            .flatten()
            .copied()
            .fold(None, |m: Option<f32>, v| Some(m.map_or(v, |m| m.max(v))));
        if let Vcore::Svi2 { core_plane, .. } = self.family.vcore {
            s.core_volts = self
                .amd
                .read_smn(core_plane)
                .map(|c| voltage::svi2(voltage::svi2_telemetry_code(c)));
        }
        s.core_vids = (0..self.cores)
            .map(|core| {
                let v = self
                    .amd
                    .read_msr_on(core * self.threads_per_core, pstate::MSR)?;
                let st = pstate::Status::decode(v as u32);
                Some(match self.family.vcore {
                    Vcore::Svi2 { .. } => voltage::svi2(st.vid),
                    _ => voltage::svi3(st.vid),
                })
            })
            .collect();
        s
    }

    /// The self-test: write the value a core already holds, read it back, compare. Changes nothing when it works.
    /// This is what must pass before a write path that was never checked on this family is used.
    pub fn selftest(&self, core: usize) -> Result<(i32, i32), String> {
        let before = self.read_margin(core)?;
        let (deepest, shallowest) = self.allowed_range();
        if before < deepest || before > shallowest {
            return Err(format!(
                "core {core} holds {before}, outside the range this program writes; not testing with it"
            ));
        }
        let after = self.set_margin(core, before)?;
        std::thread::sleep(Duration::from_millis(50));
        Ok((before, after))
    }
}

impl Machine {
    /// The stronger self-test: move a core one step toward zero (the safe direction, more voltage), read it back, and
    /// put the original value back. Returns (original, while moved, after restoring). The restore is attempted on
    /// every path, and a failure to restore is reported as an error carrying the original so the caller can say so.
    pub fn selftest_step(&self, core: usize) -> Result<(i32, i32, i32), String> {
        let original = self.read_margin(core)?;
        let (deepest, shallowest) = self.allowed_range();
        if original < deepest || original > shallowest {
            return Err(format!(
                "core {core} holds {original}, outside the range this program writes; not testing with it"
            ));
        }
        let moved_to = if original < shallowest {
            original + 1
        } else {
            original - 1
        };
        let moved = self.set_margin(core, moved_to);
        let restored = self.set_margin(core, original);
        match (moved, restored) {
            (Ok(moved), Ok(after)) => {
                if after != original {
                    Err(format!(
                        "core {core}: the original value {original} did not come back (reads {after})"
                    ))
                } else {
                    Ok((original, moved, after))
                }
            }
            (_, Err(e)) => Err(format!(
                "core {core}: RESTORING {original} FAILED ({e}); set it again by hand"
            )),
            (Err(e), Ok(_)) => Err(e),
        }
    }
}
