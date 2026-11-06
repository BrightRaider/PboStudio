//! The plan as a readable file (ux.md): a small subset of TOML, by hand writable and commentable, the same plan the
//! window shows and the command line runs.
//!
//! ```text
//! # PboStudio plan
//! schema_version = 1
//! cores = [2, 5]          # nothing chosen means all cores
//! engine = "ycruncher"
//! seconds = 120
//! search = "fine"
//! guardband = 3
//! smt = false
//! mode = "avx2"
//! fft = "Smallest"
//! then = "ycruncher:avx2:Smallest"   # further steps per core, joined with +
//! ```
//!
//! Only `key = value` lines, comments and blank lines. A value is a string, a number, `true` or `false`, or (for
//! `cores`) a list of numbers. Anything else is an error that names its line; a file is never half applied.

use crate::plan::Plan;
use crate::wire::SCHEMA_VERSION;

impl Plan {
    pub fn to_toml(&self) -> String {
        let pairs = self.to_pairs();
        let value = |key: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let cores: Vec<String> = self.cores.iter().map(|c| c.to_string()).collect();
        format!(
            "# PboStudio plan. Nothing chosen for cores means all of them.\n\
             schema_version = {SCHEMA_VERSION}\n\
             cores = [{}]\n\
             engine = \"{}\"\n\
             seconds = {}\n\
             search = \"{}\"\n\
             guardband = {}\n\
             start = {}\n\
             smt = {}\n\
             mode = \"{}\"\n\
             fft = \"{}\"\n\
             then = \"{}\"\n\
             rounds = {}\n\
             order = \"{}\"\n\
             transient = \"{}\"\n\
             on_error = \"{}\"\n\
             notify = {}\n\
             ycruncher = \"{}\"\n",
            cores.join(", "),
            value("engine"),
            // "auto" is a word, not a number, and is written as one.
            if value("seconds") == "auto" {
                "\"auto\"".to_string()
            } else {
                value("seconds")
            },
            value("search"),
            value("guardband"),
            value("start"),
            value("smt"),
            value("mode"),
            value("fft"),
            value("then"),
            value("rounds"),
            value("order"),
            value("transient"),
            value("on_error"),
            value("notify"),
            value("ycruncher"),
        )
    }

    /// A plan from the text of a plan file. Missing keys keep the recommendation.
    pub fn from_toml(text: &str) -> Result<Plan, String> {
        let mut plan = Plan::recommended();
        for (n, raw) in text.lines().enumerate() {
            let line = n + 1;
            let content = strip_comment(raw).trim().to_string();
            if content.is_empty() {
                continue;
            }
            let Some((key, value)) = content.split_once('=') else {
                return Err(format!("line {line}: a key = value line was expected"));
            };
            let key = key.trim();
            let value = value.trim();
            let value =
                if let Some(list) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
                    if key != "cores" {
                        return Err(format!("line {line}: only cores takes a list"));
                    }
                    let parts: Vec<&str> = list
                        .split(',')
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .collect();
                    parts.join(",")
                } else if let Some(s) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                    s.to_string()
                } else if value.starts_with('"') || value.contains(['"', '[', ']']) {
                    return Err(format!("line {line}: the value is not closed"));
                } else {
                    value.to_string()
                };
            if key == "schema_version" {
                if value != SCHEMA_VERSION.to_string() {
                    return Err(format!(
                        "line {line}: schema_version {value} is not supported (this is {SCHEMA_VERSION})"
                    ));
                }
                continue;
            }
            plan.set(key, &value)
                .map_err(|e| format!("line {line}: {e}"))?;
        }
        Ok(plan)
    }
}

/// The text before a `#` that is not inside quotes.
fn strip_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return &line[..i],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Engine, Search};

    #[test]
    fn a_plan_survives_its_file() {
        let mut p = Plan::recommended();
        p.cores = vec![2, 5];
        p.engine = Engine::YCruncher;
        p.search = Search::Fine;
        p.seconds_per_test = 120;
        p.smt = true;
        assert_eq!(Plan::from_toml(&p.to_toml()).unwrap(), p);
        assert_eq!(
            Plan::from_toml(&Plan::recommended().to_toml()).unwrap(),
            Plan::recommended()
        );
    }

    #[test]
    fn comments_blank_lines_and_hash_inside_quotes() {
        let p = Plan::from_toml(
            "# a plan\n\nseconds = 90   # a minute and a half\nengine = \"prime95\"\n",
        )
        .unwrap();
        assert_eq!(p.seconds_per_test, 90);
        assert_eq!(strip_comment("a = \"x # y\" # z"), "a = \"x # y\" ");
    }

    #[test]
    fn an_error_names_its_line_and_nothing_is_half_applied() {
        let e = Plan::from_toml("seconds = 90\nsearch = \"sideways\"\n").unwrap_err();
        assert!(e.starts_with("line 2:"), "{e}");
        assert!(Plan::from_toml("what is this")
            .unwrap_err()
            .starts_with("line 1:"));
        assert!(Plan::from_toml("engine = \"prime95").is_err());
        assert!(Plan::from_toml("seconds = [1]").is_err());
        assert!(Plan::from_toml("schema_version = 2").is_err());
    }

    #[test]
    fn an_empty_core_list_is_all_cores() {
        assert!(Plan::from_toml("cores = []").unwrap().cores.is_empty());
        assert_eq!(Plan::from_toml("cores = [3, 1]").unwrap().cores, vec![3, 1]);
    }
}
