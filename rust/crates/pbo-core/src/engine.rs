//! What the two stress engines write, and what that means (invariants I2 and I4).
//!
//! Prime95 and y-cruncher report a failed calculation by writing a line into a file. Which lines are failures is
//! the whole difficulty: Prime95 writes its successes and failures into the same file and the word "self-test" is in
//! both ("Self-test 8960K passed!" and "...self-test failed"), and y-cruncher's own banner says "Stop on Error:
//! Enabled". So the markers are phrases that occur only on failure, and any line that announces a pass is excluded
//! first. This module is the pure part: it takes lines and says what they are, and builds the configuration and the
//! command line each engine is started with. Starting the process is the operating system layer's job. Vectors:
//! `spec/vectors/engine-output.json` and `spec/vectors/engine-launch.json`.

use std::collections::BTreeSet;

/// What a line of engine output is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A Prime95 FFT size finished without error.
    PassedFft(u32),
    /// A line that announces success but carries no FFT size.
    Passed,
    /// A calculation error: the line, trimmed.
    Failure(String),
    /// Anything else: banners, progress, the affinity refusals the job object causes.
    Other,
}

const PRIME95_MARKERS: [&str; 9] = [
    "FATAL ERROR",
    "Hardware failure detected",
    // The message Prime95's own readme names for a failed check ("Possible hardware failure, consult the readme file").
    "Possible hardware failure",
    "self-test failed",
    "ERROR: ROUND OFF",
    "Rounding was",
    "Illegal sumout",
    "SUMOUT",
    "TORTURE TEST FAILED",
];

const YCRUNCHER_MARKERS: [&str; 7] = [
    "Coefficient is too large",
    "computation error",
    "hardware error",
    "mismatch",
    "test failed",
    "FAILED",
    "incorrect result",
];

fn contains_ci(line: &str, needle: &str) -> bool {
    line.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// The FFT size in "self-test 8960K passed", whatever the case and the spacing.
fn passed_fft(line: &str) -> Option<u32> {
    let lower = line.to_ascii_lowercase();
    let start = lower.find("self-test")? + "self-test".len();
    let rest = lower[start..].trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let after = rest[digits.len()..].strip_prefix('k')?.trim_start();
    if after.starts_with("passed") {
        digits.parse().ok()
    } else {
        None
    }
}

pub fn classify_prime95(line: &str) -> Line {
    if contains_ci(line, "passed") {
        return match passed_fft(line) {
            Some(k) => Line::PassedFft(k),
            None => Line::Passed,
        };
    }
    if PRIME95_MARKERS.iter().any(|m| contains_ci(line, m)) {
        Line::Failure(line.trim().to_string())
    } else {
        Line::Other
    }
}

pub fn classify_ycruncher(line: &str) -> Line {
    if contains_ci(line, "passed") {
        return Line::Passed;
    }
    // The job object denies every processor outside the mask and y-cruncher announces each refusal. That is the
    // confinement working, not a fault.
    if contains_ci(line, "core affinity") {
        return Line::Other;
    }
    if YCRUNCHER_MARKERS.iter().any(|m| contains_ci(line, m)) {
        Line::Failure(line.trim().to_string())
    } else {
        Line::Other
    }
}

/// Counts finished sweeps of Prime95's FFT list: one is done the moment a size comes round a second time.
#[derive(Debug, Default, Clone)]
pub struct Prime95Cycles {
    seen: BTreeSet<u32>,
    pub completed: u32,
}

impl Prime95Cycles {
    pub fn feed(&mut self, line: &Line) {
        if let Line::PassedFft(k) = line {
            if !self.seen.insert(*k) {
                self.completed += 1;
                self.seen.clear();
                self.seen.insert(*k);
            }
        }
    }
}

/// y-cruncher prints "Iteration: N" at the start of each pass; the pass running now is not finished.
pub fn ycruncher_cycles(iteration_headers_seen: u32) -> u32 {
    iteration_headers_seen.saturating_sub(1)
}

pub fn is_ycruncher_iteration_header(line: &str) -> bool {
    contains_ci(line, "Iteration:")
}

// ── Prime95 launch ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prime95Mode {
    Sse,
    Avx,
    Avx2,
    Avx512,
}

impl Prime95Mode {
    pub fn name(self) -> &'static str {
        match self {
            Prime95Mode::Sse => "Sse",
            Prime95Mode::Avx => "Avx",
            Prime95Mode::Avx2 => "Avx2",
            Prime95Mode::Avx512 => "Avx512",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FftPreset {
    Smallest,
    Small,
    Large,
    Huge,
    All,
    Heavy,
    HeavyShort,
    Moderate,
    /// A range of one's own, in K: the smallest and the largest FFT size.
    Custom(u32, u32),
}

/// The smallest and the largest FFT size Prime95 takes, in K.
pub const FFT_MIN_K: u32 = 4;
pub const FFT_MAX_K: u32 = 65536;

impl FftPreset {
    /// A range of one's own, or nothing when it is not one: the smallest size and the largest have to be in order and
    /// within what Prime95 takes.
    pub fn custom(min_k: u32, max_k: u32) -> Option<FftPreset> {
        ((FFT_MIN_K..=FFT_MAX_K).contains(&min_k)
            && (FFT_MIN_K..=FFT_MAX_K).contains(&max_k)
            && min_k <= max_k)
            .then_some(FftPreset::Custom(min_k, max_k))
    }

    /// The text for files and the command line: the name of a preset, or `MIN-MAX` in K.
    pub fn to_text(self) -> String {
        match self {
            FftPreset::Custom(lo, hi) => format!("{lo}-{hi}"),
            other => other.name().to_string(),
        }
    }

    /// FFT range in K, matching CoreCycler's presets so results are comparable between the two.
    pub fn range(self) -> (u32, u32) {
        match self {
            FftPreset::Smallest => (4, 21),
            FftPreset::Small => (36, 248),
            FftPreset::Large => (426, 8192),
            FftPreset::Huge => (8960, 32768),
            FftPreset::All => (4, 32768),
            FftPreset::Heavy => (4, 1344),
            FftPreset::HeavyShort => (4, 160),
            FftPreset::Moderate => (1344, 4096),
            FftPreset::Custom(lo, hi) => (lo, hi),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FftPreset::Smallest => "Smallest",
            FftPreset::Small => "Small",
            FftPreset::Large => "Large",
            FftPreset::Huge => "Huge",
            FftPreset::All => "All",
            FftPreset::Heavy => "Heavy",
            FftPreset::HeavyShort => "HeavyShort",
            FftPreset::Moderate => "Moderate",
            FftPreset::Custom(..) => "Custom",
        }
    }

    pub fn from_name(name: &str) -> Option<FftPreset> {
        Some(match name {
            "Smallest" => FftPreset::Smallest,
            "Small" => FftPreset::Small,
            "Large" => FftPreset::Large,
            "Huge" => FftPreset::Huge,
            "All" => FftPreset::All,
            "Heavy" => FftPreset::Heavy,
            "HeavyShort" => FftPreset::HeavyShort,
            "Moderate" => FftPreset::Moderate,
            other => {
                let (lo, hi) = other.split_once('-')?;
                return FftPreset::custom(lo.trim().parse().ok()?, hi.trim().parse().ok()?);
            }
        })
    }
}

/// Which algorithms y-cruncher runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YcSet {
    /// The five that find Curve Optimizer instability best.
    #[default]
    CurveOptimizer,
    /// Three, for a quick look.
    FastDiscovery,
}

impl YcSet {
    pub const ALL: [YcSet; 2] = [YcSet::CurveOptimizer, YcSet::FastDiscovery];

    /// The name in files and on the command line.
    pub fn id(self) -> &'static str {
        match self {
            YcSet::CurveOptimizer => "curve-optimizer",
            YcSet::FastDiscovery => "fast-discovery",
        }
    }

    pub fn from_id(id: &str) -> Option<YcSet> {
        YcSet::ALL
            .into_iter()
            .find(|s| s.id().eq_ignore_ascii_case(id.trim()))
    }

    pub fn algorithms(self) -> &'static [&'static str] {
        match self {
            YcSet::CurveOptimizer => &YCRUNCHER_CURVE_OPTIMIZER,
            YcSet::FastDiscovery => &YCRUNCHER_FAST_DISCOVERY,
        }
    }
}

/// With the time per core set to "auto", a test runs until the engine has been through its whole work list once. It
/// never runs past this, so an engine that stops reporting cannot hold a core for ever.
pub const AUTO_RUNTIME_CAP_SECONDS: u64 = 3600;

/// The whole of prime.txt. Prime95 30.x folds local.txt into prime.txt on start, so every setting goes into the one
/// file; and it picks its own worker count unless `NumWorkers` and `CoresPerTest` are pinned to 1, which otherwise
/// spreads the load across cores and makes single-core testing meaningless.
pub fn prime95_config(mode: Prime95Mode, fft: FftPreset, memory_mb: u32, threads: u32) -> String {
    let (min_fft, max_fft) = fft.range();
    let flag = |on: bool| u8::from(on);
    let lines = [
        "StressTester=1".to_string(),
        "UsePrimenet=0".to_string(),
        "V24OptionsConverted=1".to_string(),
        "WGUID_version=2".to_string(),
        // Round-off and input checking is what turns a wrong result into a reported error, not a silent one.
        "ErrorCheck=1".to_string(),
        "SumInputsErrorCheck=1".to_string(),
        format!("CpuSupportsAVX={}", flag(mode >= Prime95Mode::Avx)),
        format!("CpuSupportsAVX2={}", flag(mode >= Prime95Mode::Avx2)),
        format!("CpuSupportsFMA3={}", flag(mode >= Prime95Mode::Avx2)),
        format!("CpuSupportsAVX512F={}", flag(mode >= Prime95Mode::Avx512)),
        format!("MinTortureFFT={min_fft}"),
        format!("MaxTortureFFT={max_fft}"),
        format!("TortureMem={memory_mb}"),
        "TortureTime=1".to_string(),
        format!("TortureThreads={threads}"),
        "TortureHyperthreading=0".to_string(),
        "NumWorkers=1".to_string(),
        "CoresPerTest=1".to_string(),
        // Prime95 30.10 and newer take the number of cores from `NumCores` (`TortureThreads` is the old name): without
        // it the worker uses every core of the machine, and squeezed onto the one processor the engine is confined to
        // it spins and reports nothing for minutes. Found by comparing with what CoreCycler writes.
        format!("NumCores={threads}"),
        // The engine is confined to its processors from outside (a job object). If Prime95 binds its worker threads
        // itself it asks for processors it may not use and never gets going. Together with `NumCores` this is what
        // makes it report a finished FFT size at all.
        "EnableSetAffinity=0".to_string(),
        "EnableSetPriority=0".to_string(),
        "V30OptionsConverted=1".to_string(),
        "ExitOnX=1".to_string(),
        // How often the timestamp goes into the results file.
        "ResultsFileTimestampInterval=60".to_string(),
    ];
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

/// The command line of a y-cruncher stress run. `logfile` is a global switch and has to precede the verb. The stress
/// settings are in a configuration file (see `ycruncher_config`): the `stress` verb on the command line cannot say
/// which logical processors to use, and without that y-cruncher starts one thread for every logical processor of the
/// machine. Squeezed onto the one processor the engine is confined to, those threads run in turn, each with a sixteenth
/// of the memory, and a test of one core means little.
pub fn ycruncher_arguments(log_path: &str, config_path: &str) -> String {
    format!("logfile:\"{log_path}\" config \"{config_path}\"")
}

/// The memory y-cruncher's stress test gets for each logical processor it runs on.
pub const YCRUNCHER_MEMORY_PER_THREAD: u64 = 64 << 20;

/// The stress tests y-cruncher names on its command line, in the spelling it wants.
pub const YCRUNCHER_TAGS: [&str; 10] = [
    "BKT", "BBP", "SFTv4", "SNT", "SVT", "FFTv4", "NTT63", "N63", "VSTv3", "VT3",
];

/// A y-cruncher test tag in its own spelling, from any capitalisation.
pub fn ycruncher_tag(text: &str) -> Option<&'static str> {
    YCRUNCHER_TAGS
        .into_iter()
        .find(|t| t.eq_ignore_ascii_case(text.trim()))
}

/// The configuration file of a y-cruncher stress run on exactly the logical processors in `logical` (their numbers
/// as Windows counts them). The layout is the one y-cruncher itself reads for `config`; CoreCycler writes the same.
pub fn ycruncher_config(logical: &[usize], seconds_per_test: u32, algorithms: &[&str]) -> String {
    ycruncher_config_with(
        logical,
        seconds_per_test,
        algorithms,
        YCRUNCHER_MEMORY_PER_THREAD,
    )
}

/// The same with the memory for each logical processor of one's own, in bytes.
pub fn ycruncher_config_with(
    logical: &[usize],
    seconds_per_test: u32,
    algorithms: &[&str],
    memory_per_thread: u64,
) -> String {
    let cores: Vec<String> = logical.iter().map(usize::to_string).collect();
    let memory = memory_per_thread * logical.len().max(1) as u64;
    let mut lines = vec![
        "{".to_string(),
        "    Action : \"StressTest\"".to_string(),
        "    StressTest : {".to_string(),
        "        AllocateLocally : \"true\"".to_string(),
        format!("        LogicalCores : [{}]", cores.join(" ")),
        format!("        TotalMemory : {memory}"),
        format!("        SecondsPerTest : {seconds_per_test}"),
        "        SecondsTotal : 0".to_string(),
        "        StopOnError : \"true\"".to_string(),
        "        Tests : [".to_string(),
    ];
    lines.extend(algorithms.iter().map(|a| format!("            \"{a}\"")));
    lines.extend([
        "        ]".to_string(),
        "    }".to_string(),
        "}".to_string(),
    ]);
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

/// The algorithms of each y-cruncher set (names are literal: v0.7's "SFT" and "FFT" became "SFTv4" and "FFTv4").
pub const YCRUNCHER_CURVE_OPTIMIZER: [&str; 5] = ["SFTv4", "SVT", "FFTv4", "N63", "VT3"];
pub const YCRUNCHER_FAST_DISCOVERY: [&str; 3] = ["VT3", "FFTv4", "N63"];

/// Which y-cruncher binary to prefer, by the filename prefix y-cruncher gives them ("19-ZN2 ~ Kagari.exe"), coldest
/// instruction set last. `generation` is 1 to 5 for Zen 1 to Zen 5.
pub fn ycruncher_preferred_prefixes(generation: u32) -> &'static [&'static str] {
    match generation {
        5 => &["24-ZN5", "22-ZN4", "19-ZN2"],
        4 => &["22-ZN4", "19-ZN2"],
        2 | 3 => &["19-ZN2", "17-ZN1"],
        1 => &["17-ZN1", "13-HSW"],
        _ => &["13-HSW", "11-SNB", "00-x86"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pass_is_never_a_failure() {
        assert_eq!(
            classify_prime95("Self-test 8960K passed!"),
            Line::PassedFft(8960)
        );
        assert_eq!(
            classify_prime95("self-test  12K  PASSED"),
            Line::PassedFft(12)
        );
    }

    #[test]
    fn the_messages_prime95_documents_for_a_failure_are_failures_and_its_notes_are_not() {
        // From Prime95's readme.txt and stress.txt, and from what 30.x writes in a torture test.
        for line in [
            "Possible hardware failure, consult the readme file.",
            "[Worker #1 Nov 6 10:00] Hardware failure detected running 10752K FFT size, consult stress.txt file.",
            "FATAL ERROR: Rounding was 0.5, expected less than 0.4",
            "ERROR: ROUND OFF > 0.40",
            "Self-test 1024K self-test failed",
        ] {
            assert!(
                matches!(classify_prime95(line), Line::Failure(_)),
                "not a failure: {line}"
            );
        }
        // A note that says a flagged error may have been a good result, a start-up line and a pass are not failures.
        for line in [
            "Disregard last error.  Result was reproducible and thus not a hardware problem.",
            "Beginning a continuous self-test to check your computer.",
            "Self-test 4K passed!",
            "[Fri Nov  6 12:05:30 2026]",
        ] {
            assert!(
                !matches!(classify_prime95(line), Line::Failure(_)),
                "a failure by mistake: {line}"
            );
        }
    }

    #[test]
    fn a_failed_self_test_is_a_failure() {
        assert!(matches!(
            classify_prime95("  FATAL ERROR: Rounding was 0.5, expected less than 0.4  "),
            Line::Failure(_)
        ));
        assert!(matches!(
            classify_prime95("Self-test 1024K self-test failed"),
            Line::Failure(_)
        ));
    }

    #[test]
    fn the_banner_is_not_a_failure() {
        assert_eq!(classify_ycruncher("Stop on Error: Enabled"), Line::Other);
    }

    #[test]
    fn affinity_refusals_are_the_job_object_working() {
        assert_eq!(
            classify_ycruncher("Failed to set core affinity: access denied"),
            Line::Other
        );
    }

    #[test]
    fn cycles_count_when_a_size_repeats() {
        let mut c = Prime95Cycles::default();
        for k in [4, 5, 6, 4, 5, 6, 4] {
            c.feed(&Line::PassedFft(k));
        }
        assert_eq!(c.completed, 2);
    }
}

#[cfg(test)]
mod custom_tests {
    use super::*;

    #[test]
    fn y_cruncher_tags_are_found_in_any_capitalisation_and_nothing_else() {
        assert_eq!(ycruncher_tag("sftv4"), Some("SFTv4"));
        assert_eq!(ycruncher_tag(" VT3 "), Some("VT3"));
        assert_eq!(ycruncher_tag("n63"), Some("N63"));
        assert_eq!(ycruncher_tag("SFT"), None);
        assert_eq!(ycruncher_tag(""), None);
    }

    #[test]
    fn the_memory_of_a_y_cruncher_run_is_a_share_for_each_logical_processor() {
        let one = ycruncher_config_with(&[2], 10, &["VT3"], 128 << 20);
        assert!(one.contains("TotalMemory : 134217728"), "{one}");
        let two = ycruncher_config_with(&[2, 3], 10, &["VT3"], 128 << 20);
        assert!(two.contains("TotalMemory : 268435456"), "{two}");
        assert_eq!(
            ycruncher_config(&[2], 10, &["VT3"]),
            ycruncher_config_with(&[2], 10, &["VT3"], YCRUNCHER_MEMORY_PER_THREAD)
        );
    }

    #[test]
    fn a_range_of_ones_own_is_checked_and_round_trips() {
        assert_eq!(FftPreset::custom(4, 1344), Some(FftPreset::Custom(4, 1344)));
        assert_eq!(FftPreset::Custom(36, 248).range(), (36, 248));
        assert_eq!(FftPreset::Custom(36, 248).to_text(), "36-248");
        assert_eq!(
            FftPreset::from_name("36-248"),
            Some(FftPreset::Custom(36, 248))
        );
        assert_eq!(
            FftPreset::from_name(" 36 - 248 "),
            Some(FftPreset::Custom(36, 248))
        );
        // Out of order, below the smallest size, above the largest, not numbers.
        for bad in ["248-36", "2-100", "4-70000", "a-b", "4-", "-9", "4_9"] {
            assert_eq!(FftPreset::from_name(bad), None, "{bad}");
        }
        // The presets keep their names.
        assert_eq!(FftPreset::from_name("Heavy"), Some(FftPreset::Heavy));
        assert_eq!(FftPreset::Heavy.to_text(), "Heavy");
    }

    #[test]
    fn a_range_of_ones_own_goes_into_the_prime95_settings() {
        let text = prime95_config(Prime95Mode::Avx2, FftPreset::Custom(100, 2000), 0, 1);
        assert!(text.contains("MinTortureFFT=100"), "{text}");
        assert!(text.contains("MaxTortureFFT=2000"), "{text}");
    }

    #[test]
    fn the_two_sets_of_y_cruncher_algorithms() {
        assert_eq!(YcSet::default(), YcSet::CurveOptimizer);
        assert_eq!(YcSet::CurveOptimizer.algorithms().len(), 5);
        assert_eq!(YcSet::FastDiscovery.algorithms().len(), 3);
        for s in YcSet::ALL {
            assert_eq!(YcSet::from_id(s.id()), Some(s));
        }
        assert_eq!(YcSet::from_id("everything"), None);
    }
}
