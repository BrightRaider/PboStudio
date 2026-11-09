//! How the load is interrupted so that a core has to boost back up.
//!
//! A steady load is the easy case for a core. A game switches between idle and peak boost far faster, and it is the
//! rate of voltage change on the way back up that a too-aggressive curve cannot follow: cores that held for hours under
//! steady load have thrown machine checks in ordinary play. So a test can interrupt its load:
//!
//! * **Periodic**: the engine is frozen for a moment every so many seconds. A dozen transitions in a six-minute slot.
//! * **Bursts**: the engine is frozen and released in fractions of a second, hundreds of times a minute.
//! * **Game**: the rhythm of a game such as League of Legends or Teamfight Tactics: frames of a few milliseconds of
//!   load at a game's frame rate, irregular, and stretches of near idle with single spikes (a menu, the time between
//!   rounds). Light load at peak boost and the jump to it out of idle are where a too-deep value fails in play.
//!
//! Vectors: `spec/vectors/transient-mode.json`.

use crate::profiles::{Profile, Transient as ProfileTransient};

/// Milliseconds the load runs before each interruption, and the core is left idle between bursts (the values of 1.x).
pub const BURST_LOAD_MS: u32 = 300;
pub const BURST_IDLE_MS: u32 = 100;

/// What the plan says about the interruptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transient {
    /// A steady load.
    Off,
    /// A pause of `pause_s` seconds every `every_s` seconds.
    Periodic { every_s: u32, pause_s: u32 },
    /// Sub-second pulses.
    Bursts,
    /// The rhythm of a game: see [`GamePattern`].
    Game,
}

impl Default for Transient {
    /// What 1.x does without being asked: a pause of a second every half minute.
    fn default() -> Transient {
        Transient::Periodic {
            every_s: 30,
            pause_s: 1,
        }
    }
}

impl Transient {
    /// The share of the time the load really runs. The check that the engine used its core compares what it used
    /// against this and not against a solid 100 %, or bursting would report every core as idle.
    pub fn duty_cycle(self) -> f64 {
        match self {
            Transient::Bursts => {
                f64::from(BURST_LOAD_MS) / f64::from(BURST_LOAD_MS + BURST_IDLE_MS)
            }
            // An estimate; a test measures the share the load really ran (the pulser counts it).
            Transient::Game => GAME_SHARE_ESTIMATE,
            _ => 1.0,
        }
    }

    /// The text for files and the command line: `off`, `bursts`, or `periodic:EVERY:PAUSE` in seconds.
    pub fn to_text(self) -> String {
        match self {
            Transient::Off => "off".to_string(),
            Transient::Bursts => "bursts".to_string(),
            Transient::Game => "game".to_string(),
            Transient::Periodic { every_s, pause_s } => format!("periodic:{every_s}:{pause_s}"),
        }
    }

    /// Reads `off`, `bursts`, `periodic` (30 s and 1 s) or `periodic:EVERY:PAUSE`.
    pub fn from_text(text: &str) -> Option<Transient> {
        let text = text.trim().to_ascii_lowercase();
        match text.as_str() {
            "off" | "none" => return Some(Transient::Off),
            "bursts" | "burst" => return Some(Transient::Bursts),
            "game" => return Some(Transient::Game),
            "periodic" => return Some(Transient::default()),
            _ => {}
        }
        let mut parts = text.split(':');
        if parts.next()? != "periodic" {
            return None;
        }
        let every_s: u32 = parts.next()?.parse().ok()?;
        let pause_s: u32 = parts.next()?.parse().ok()?;
        if parts.next().is_some() || every_s == 0 || pause_s == 0 || pause_s >= every_s {
            return None;
        }
        Some(Transient::Periodic { every_s, pause_s })
    }
}

/// About the share of the time the load runs in the rhythm of a game (frames about 40 %, quiet stretches about 2 %).
pub const GAME_SHARE_ESTIMATE: f64 = 0.3;

/// The rhythm of a game, slice by slice: milliseconds of load, then of idle. It switches between play (2 to 6 s of
/// frames: 2 to 8 ms of load, 3 to 10 ms of idle, about a hundred a second) and quiet (1 to 3 s: a spike of 1 to 3 ms
/// every 40 to 250 ms), with a little randomness everywhere, as a game is never even.
#[derive(Debug, Clone)]
pub struct GamePattern {
    state: u64,
    phase_left_ms: u32,
    quiet: bool,
}

impl GamePattern {
    pub fn new(seed: u64) -> GamePattern {
        GamePattern {
            state: seed | 1,
            phase_left_ms: 0,
            quiet: true,
        }
    }

    /// A number from `low` to `high`, both included.
    fn between(&mut self, low: u32, high: u32) -> u32 {
        // xorshift64*
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        let r = self.state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        low + (r >> 33) as u32 % (high - low + 1)
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// The next slice: milliseconds of load, then milliseconds of idle.
    pub fn next_slice(&mut self) -> (u32, u32) {
        if self.phase_left_ms == 0 {
            self.quiet = !self.quiet;
            self.phase_left_ms = if self.quiet {
                self.between(1000, 3000)
            } else {
                self.between(2000, 6000)
            };
        }
        let slice = if self.quiet {
            (self.between(1, 3), self.between(40, 250))
        } else {
            (self.between(2, 8), self.between(3, 10))
        };
        self.phase_left_ms = self.phase_left_ms.saturating_sub(slice.0 + slice.1);
        slice
    }
}

impl Profile {
    /// How this ready-made list interrupts its load. A pause every 0 seconds is no pause.
    pub fn transient_mode(&self) -> Transient {
        match self.transient {
            ProfileTransient::MicroBurst => Transient::Bursts,
            ProfileTransient::Periodic => {
                if self.suspend_every_seconds == 0 || self.suspend_for_seconds == 0 {
                    Transient::Off
                } else {
                    Transient::Periodic {
                        every_s: self.suspend_every_seconds,
                        pause_s: self.suspend_for_seconds,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bursting_runs_the_load_three_quarters_of_the_time_and_nothing_else_costs_any() {
        assert!((Transient::Bursts.duty_cycle() - 0.75).abs() < 1e-9);
        assert_eq!(Transient::Off.duty_cycle(), 1.0);
        assert_eq!(Transient::default().duty_cycle(), 1.0);
    }

    #[test]
    fn the_text_round_trips() {
        for t in [
            Transient::Off,
            Transient::Bursts,
            Transient::Periodic {
                every_s: 5,
                pause_s: 1,
            },
            Transient::default(),
        ] {
            assert_eq!(Transient::from_text(&t.to_text()), Some(t));
        }
    }

    #[test]
    fn what_cannot_be_read_is_nothing() {
        for bad in [
            "",
            "periodic:0:1",
            "periodic:5:0",
            "periodic:5:5",
            "periodic:x:1",
            "periodic:5",
            "sometimes",
            "periodic:5:1:2",
        ] {
            assert_eq!(Transient::from_text(bad), None, "{bad}");
        }
        assert_eq!(
            Transient::from_text(" PERIODIC "),
            Some(Transient::default())
        );
    }

    #[test]
    fn the_ready_made_lists_keep_how_they_interrupt() {
        let all = crate::profiles::all();
        let breaking = all.iter().find(|p| p.id == "BreakingPoint").unwrap();
        assert_eq!(
            breaking.transient_mode(),
            Transient::Periodic {
                every_s: 5,
                pause_s: 1
            }
        );
        let micro = all.iter().find(|p| p.id == "MicroBurst").unwrap();
        assert_eq!(micro.transient_mode(), Transient::Bursts);
        let quick = all.iter().find(|p| p.id == "QuickCheck").unwrap();
        assert_eq!(quick.transient_mode(), Transient::default());
    }
    #[test]
    fn the_rhythm_of_a_game_has_frames_and_quiet_stretches_and_runs_about_a_third_of_the_time() {
        let mut g = GamePattern::new(42);
        let (mut load, mut idle, mut frames, mut spikes) = (0u64, 0u64, 0u32, 0u32);
        // Ten minutes of slices.
        while load + idle < 600_000 {
            let quiet_before = g.is_quiet();
            let (l, i) = g.next_slice();
            if g.is_quiet() {
                assert!((1..=3).contains(&l) && (40..=250).contains(&i), "{l} {i}");
                spikes += 1;
            } else {
                assert!((2..=8).contains(&l) && (3..=10).contains(&i), "{l} {i}");
                frames += 1;
            }
            let _ = quiet_before;
            load += u64::from(l);
            idle += u64::from(i);
        }
        assert!(
            frames > 10_000 && spikes > 100,
            "{frames} frames, {spikes} spikes"
        );
        let share = load as f64 / (load + idle) as f64;
        assert!((0.2..0.4).contains(&share), "{share}");
        assert_eq!(Transient::from_text("GAME"), Some(Transient::Game));
        assert_eq!(Transient::Game.to_text(), "game");
    }
}
