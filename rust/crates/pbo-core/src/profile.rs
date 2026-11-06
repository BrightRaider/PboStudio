//! What of a saved profile it is safe to write back at logon (invariant I10).
//!
//! The logon task used to write the saved values into the processor unexamined. Two ways that goes wrong without
//! anybody noticing: the profile belongs to another processor (swapping a 5800X3D for a 7800X3D leaves eight cores and
//! an eight-entry file, and every old value is applied to a chip it was never tested on), and a value is outside what
//! any processor accepts (a damaged file, or a figure the processor clamps). Vectors: `spec/vectors/saved-profile.json`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// What may be written: only cores that exist, only values in range.
    pub margins: BTreeMap<i32, i32>,
    /// What was left out and why, for the log. Empty when everything was usable.
    pub problems: Vec<String>,
    pub wrong_processor: bool,
}

pub fn check(
    profile_cpu: &str,
    margins: &BTreeMap<i32, i32>,
    current_cpu: &str,
    core_count: i32,
    deepest_allowed: i32,
    shallowest_allowed: i32,
) -> Check {
    if !profile_cpu.trim().eq_ignore_ascii_case(current_cpu.trim()) {
        return Check {
            margins: BTreeMap::new(),
            problems: vec![format!(
                "saved for \"{profile_cpu}\", this is \"{current_cpu}\""
            )],
            wrong_processor: true,
        };
    }

    let mut usable = BTreeMap::new();
    let mut problems = Vec::new();
    for (core, value) in margins {
        if *core < 0 || *core >= core_count {
            problems.push(format!("core {core} does not exist on this processor"));
        } else if *value < deepest_allowed || *value > shallowest_allowed {
            problems.push(format!(
                "core {core}: {value} is outside {deepest_allowed}..{shallowest_allowed}"
            ));
        } else {
            usable.insert(*core, *value);
        }
    }
    Check {
        margins: usable,
        problems,
        wrong_processor: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(pairs: &[(i32, i32)]) -> BTreeMap<i32, i32> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn another_processor_gets_nothing_written() {
        let c = check(
            "AMD Ryzen 7 5800X3D",
            &m(&[(0, -30)]),
            "AMD Ryzen 7 7800X3D",
            8,
            -30,
            30,
        );
        assert!(c.wrong_processor && c.margins.is_empty() && c.problems.len() == 1);
    }

    #[test]
    fn the_name_is_compared_without_case_and_edges() {
        let c = check(
            " amd ryzen 7 5800x3d ",
            &m(&[(0, -30)]),
            "AMD Ryzen 7 5800X3D",
            8,
            -30,
            30,
        );
        assert!(!c.wrong_processor && c.margins.len() == 1);
    }

    #[test]
    fn a_core_that_does_not_exist_or_a_value_out_of_range_is_left_out_with_a_reason() {
        let c = check(
            "X",
            &m(&[(-1, -5), (0, -31), (1, -10), (8, -5), (2, 31)]),
            "X",
            8,
            -30,
            30,
        );
        assert_eq!(c.margins, m(&[(1, -10)]));
        assert_eq!(c.problems.len(), 4);
    }
}
