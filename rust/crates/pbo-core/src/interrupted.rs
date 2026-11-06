//! Crash or closed program (invariant I9).
//!
//! Any run that ends abnormally leaves its state file: a crash, but also a kill from the task manager, a crash of
//! this program, or a window closed mid-run. Every one used to be reported as an unexpected system restart, and the
//! margin the core held was written into the tuner's memory as a failure for good. A core that never failed was
//! capped for something it did not do. It is a crash only if the computer started again after the state was last
//! written. Vectors: `spec/vectors/interrupted-run.json`.

/// `None` when there is no state file, so nothing was interrupted. Otherwise whether the system restarted after
/// the state was last written (`true`: a crash or a reset) or stayed up (`false`: the program was closed or
/// killed, and nothing about the core is known to have failed). Times are seconds on any common scale.
pub fn system_restarted(state_last_written: Option<i64>, system_started: i64) -> Option<bool> {
    state_last_written.map(|written| system_started > written)
}
