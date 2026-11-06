//! What a test of the held values does when a core fails.
//!
//! A core that failed at the value it holds has told what there was to tell: testing it again in the next round costs
//! time and finds nothing new. So by default the core is left out of the later rounds. Some want the whole run to stop
//! at the first error instead (the night is over, the value is wrong), and some want every core tested every round
//! whatever happens. A search is not covered: failing is how a search finds the edge. Vectors:
//! `spec/vectors/on-error.json`.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnError {
    /// The core is tested again in the next round.
    Continue,
    /// The core is left out of the later rounds.
    #[default]
    SkipCore,
    /// The whole run ends at the first error.
    Stop,
}

impl OnError {
    pub const ALL: [OnError; 3] = [OnError::Continue, OnError::SkipCore, OnError::Stop];

    /// The name in files and on the command line.
    pub fn id(self) -> &'static str {
        match self {
            OnError::Continue => "continue",
            OnError::SkipCore => "skip",
            OnError::Stop => "stop",
        }
    }

    pub fn from_id(id: &str) -> Option<OnError> {
        OnError::ALL
            .into_iter()
            .find(|o| o.id().eq_ignore_ascii_case(id.trim()))
    }
}

/// The memory of one run: which cores failed, and whether that ends the run.
#[derive(Debug, Clone)]
pub struct ErrorPolicy {
    on_error: OnError,
    skipped: BTreeSet<usize>,
    stopped: bool,
}

impl ErrorPolicy {
    pub fn new(on_error: OnError) -> ErrorPolicy {
        ErrorPolicy {
            on_error,
            skipped: BTreeSet::new(),
            stopped: false,
        }
    }

    /// Records that `core` failed.
    pub fn failed(&mut self, core: usize) {
        match self.on_error {
            OnError::Continue => {}
            OnError::SkipCore => {
                self.skipped.insert(core);
            }
            OnError::Stop => self.stopped = true,
        }
    }

    /// Whether `core` is left out from here on.
    pub fn skips(&self, core: usize) -> bool {
        self.skipped.contains(&core)
    }

    /// Whether the run ends now.
    pub fn stop_requested(&self) -> bool {
        self.stopped
    }

    /// The cores of `order` that are still to be tested, in order.
    pub fn remaining(&self, order: &[usize]) -> Vec<usize> {
        order.iter().copied().filter(|c| !self.skips(*c)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_default_a_failed_core_is_left_out_of_the_later_rounds() {
        let mut p = ErrorPolicy::new(OnError::default());
        p.failed(3);
        assert!(p.skips(3));
        assert!(!p.skips(2));
        assert!(!p.stop_requested());
        assert_eq!(p.remaining(&[1, 3, 5]), vec![1, 5]);
    }

    #[test]
    fn continuing_tests_every_core_every_round() {
        let mut p = ErrorPolicy::new(OnError::Continue);
        p.failed(3);
        assert!(!p.skips(3));
        assert!(!p.stop_requested());
        assert_eq!(p.remaining(&[1, 3, 5]), vec![1, 3, 5]);
    }

    #[test]
    fn stopping_ends_the_run_at_the_first_error() {
        let mut p = ErrorPolicy::new(OnError::Stop);
        assert!(!p.stop_requested());
        p.failed(0);
        assert!(p.stop_requested());
    }

    #[test]
    fn the_names_round_trip() {
        for o in OnError::ALL {
            assert_eq!(OnError::from_id(o.id()), Some(o));
        }
        assert_eq!(OnError::from_id(" Skip "), Some(OnError::SkipCore));
        assert_eq!(OnError::from_id("ignore"), None);
        assert_eq!(OnError::default(), OnError::SkipCore);
    }
}
