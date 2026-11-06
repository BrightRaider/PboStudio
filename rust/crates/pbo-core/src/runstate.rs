//! The life of a run as a state machine (ADR-0004).
//!
//! The faults found in 1.x all had one origin: state that lived only in the window. A second start next to the first,
//! a start that was cancelled and still stood as a run, a close in the middle of a run that left the cores at 0, two
//! cancel buttons on one field. Each of those is a transition that does not exist here. The window and the command line
//! send commands and read the state; neither decides. Which controls are visible is a function of the state, in
//! [`Controls::of`], and not code in the window.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Nothing is running; a run may be started.
    Ready,
    /// The engines and the processor are being made ready; nothing is under test yet.
    Preparing,
    /// A core is under test.
    Running { phase: u32, core: u32 },
    /// A stop was asked for and is being carried out: the engine ends, the values are put back.
    Stopping,
    /// The run ended with its plan done.
    Finished,
    /// The run was stopped before its plan was done, by the person or by a failure that could not be handled.
    Halted(HaltReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaltReason {
    StoppedByPerson,
    /// The temperature, a hardware error or the like.
    LimitReached,
    /// The test did not run properly.
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Start,
    /// Preparation is done; testing begins.
    Begin {
        phase: u32,
        core: u32,
    },
    /// The test moves on to another core or phase.
    Advance {
        phase: u32,
        core: u32,
    },
    /// The person asks to stop.
    Stop,
    /// A limit was reached or a test could not run.
    Abort(HaltReason),
    /// The plan is done, or the stop has been carried out.
    Done,
    /// Back to the start screen after a result has been seen.
    Dismiss,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: State,
    pub command: Command,
}

/// The next state. A command that does not belong in the current state is an error, never a second run.
pub fn apply(state: &State, command: Command) -> Result<State, InvalidTransition> {
    use Command::*;
    use State::*;
    let next = match (state, command) {
        (Ready, Start) => Preparing,
        (Preparing, Begin { phase, core }) => Running { phase, core },
        (Running { .. }, Advance { phase, core }) => Running { phase, core },

        // Stopping is possible while preparing and while running, and only then.
        (Preparing | Running { .. }, Stop) => Stopping,
        (Stopping, Done) => Halted(HaltReason::StoppedByPerson),

        (Preparing | Running { .. }, Abort(reason)) => Halted(reason),
        (Running { .. }, Done) => Finished,

        (Finished | Halted(_), Dismiss) => Ready,
        _ => {
            return Err(InvalidTransition {
                from: state.clone(),
                command,
            })
        }
    };
    Ok(next)
}

/// What the window may show, as a function of the state alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Controls {
    pub can_start: bool,
    /// Exactly one stop control is ever visible.
    pub can_stop: bool,
    /// The plan can be edited only when nothing is going on.
    pub can_edit_plan: bool,
    pub shows_progress: bool,
    pub shows_result: bool,
}

impl Controls {
    pub fn of(state: &State) -> Controls {
        match state {
            State::Ready => Controls {
                can_start: true,
                can_stop: false,
                can_edit_plan: true,
                shows_progress: false,
                shows_result: false,
            },
            State::Preparing | State::Running { .. } => Controls {
                can_start: false,
                can_stop: true,
                can_edit_plan: false,
                shows_progress: true,
                shows_result: false,
            },
            State::Stopping => Controls {
                can_start: false,
                can_stop: false,
                can_edit_plan: false,
                shows_progress: true,
                shows_result: false,
            },
            State::Finished | State::Halted(_) => Controls {
                can_start: false,
                can_stop: false,
                can_edit_plan: false,
                shows_progress: false,
                shows_result: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_states() -> Vec<State> {
        vec![
            State::Ready,
            State::Preparing,
            State::Running { phase: 1, core: 0 },
            State::Stopping,
            State::Finished,
            State::Halted(HaltReason::StoppedByPerson),
        ]
    }

    fn all_commands() -> Vec<Command> {
        vec![
            Command::Start,
            Command::Begin { phase: 1, core: 0 },
            Command::Advance { phase: 1, core: 1 },
            Command::Stop,
            Command::Abort(HaltReason::LimitReached),
            Command::Done,
            Command::Dismiss,
        ]
    }

    #[test]
    fn a_second_start_during_preparation_is_an_error_not_a_second_run() {
        assert!(apply(&State::Preparing, Command::Start).is_err());
        assert!(apply(&State::Running { phase: 1, core: 0 }, Command::Start).is_err());
        assert!(apply(&State::Stopping, Command::Start).is_err());
    }

    #[test]
    fn the_whole_table_of_transitions() {
        // Every pair of state and command, with its answer. A change to the machine has to change this table.
        let ok = |s: &State, c: Command| apply(s, c).is_ok();
        let mut valid = 0;
        for s in all_states() {
            for c in all_commands() {
                let expected = matches!(
                    (&s, c),
                    (State::Ready, Command::Start)
                        | (State::Preparing, Command::Begin { .. })
                        | (State::Running { .. }, Command::Advance { .. })
                        | (State::Preparing | State::Running { .. }, Command::Stop)
                        | (State::Stopping, Command::Done)
                        | (State::Preparing | State::Running { .. }, Command::Abort(_))
                        | (State::Running { .. }, Command::Done)
                        | (State::Finished | State::Halted(_), Command::Dismiss)
                );
                assert_eq!(ok(&s, c), expected, "{s:?} + {c:?}");
                if expected {
                    valid += 1;
                }
            }
        }
        assert_eq!(valid, 11);
    }

    #[test]
    fn a_normal_run() {
        let mut s = State::Ready;
        for c in [
            Command::Start,
            Command::Begin { phase: 1, core: 0 },
            Command::Advance { phase: 1, core: 1 },
            Command::Done,
            Command::Dismiss,
        ] {
            s = apply(&s, c).unwrap();
        }
        assert_eq!(s, State::Ready);
    }

    #[test]
    fn stopping_ends_halted_by_the_person() {
        let s = apply(&State::Running { phase: 2, core: 3 }, Command::Stop).unwrap();
        assert_eq!(s, State::Stopping);
        assert_eq!(
            apply(&s, Command::Done).unwrap(),
            State::Halted(HaltReason::StoppedByPerson)
        );
    }

    #[test]
    fn exactly_one_stop_control_while_running_and_none_otherwise() {
        for s in all_states() {
            let c = Controls::of(&s);
            let running = matches!(s, State::Preparing | State::Running { .. });
            assert_eq!(c.can_stop, running, "{s:?}");
            assert!(!(c.can_start && c.can_stop), "{s:?}");
        }
    }

    #[test]
    fn the_plan_is_editable_only_when_nothing_goes_on() {
        for s in all_states() {
            assert_eq!(Controls::of(&s).can_edit_plan, s == State::Ready, "{s:?}");
        }
    }
}
