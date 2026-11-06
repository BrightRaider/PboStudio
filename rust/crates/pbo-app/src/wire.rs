//! The embedding contract of `pbo-app` (ADR-0014): everything a host needs to show and drive a plan without a window.
//!
//! The plan as flat keys and values (hosts keep settings that way and bind pages to them), the plan as one flat JSON
//! object with a `schema_version`, a status line and a progress value. No dependency: the JSON here is flat on purpose,
//! so reading it needs forty lines and not a library. No globals, no `process::exit`, no logging set up: whatever is
//! needed (language, folders) is handed in.

use crate::plan::{Engine, Plan, Search};
use crate::text::{self, Key, Lang};
use pbo_core::engine::{FftPreset, Prime95Mode};
use pbo_core::runstate::{HaltReason, State};

pub const SCHEMA_VERSION: u32 = 1;

impl Plan {
    /// The plan as keys and values, in a fixed order.
    pub fn to_pairs(&self) -> Vec<(&'static str, String)> {
        let cores: Vec<String> = self.cores.iter().map(|c| c.to_string()).collect();
        vec![
            ("cores", cores.join(",")),
            (
                "engine",
                match self.engine {
                    Engine::Prime95 => "prime95",
                    Engine::YCruncher => "ycruncher",
                }
                .to_string(),
            ),
            (
                "seconds",
                if self.seconds_per_test == 0 {
                    "auto".to_string()
                } else {
                    self.seconds_per_test.to_string()
                },
            ),
            ("search", self.search.id()),
            ("guardband", self.guardband.to_string()),
            ("start", self.start.to_string()),
            ("smt", self.smt.to_string()),
            ("mode", self.mode.name().to_ascii_lowercase()),
            ("fft", self.fft.to_text()),
            (
                "then",
                self.then
                    .iter()
                    .map(|s| s.to_text())
                    .collect::<Vec<_>>()
                    .join("+"),
            ),
            ("rounds", self.rounds.to_string()),
            ("order", self.order.id().to_string()),
            ("transient", self.transient.to_text()),
            ("on_error", self.on_error.id().to_string()),
            ("notify", self.notify.to_string()),
            ("ycruncher", self.yc_set.id().to_string()),
        ]
    }

    /// Sets one part of the plan from a key and a value. An unknown key or an unreadable value is an error and changes
    /// nothing.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "cores" => {
                let mut cores = Vec::new();
                for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                    cores.push(
                        part.parse::<usize>()
                            .map_err(|_| format!("'{part}' is not a core number"))?,
                    );
                }
                self.cores = cores;
            }
            "engine" => {
                self.engine = match value {
                    "prime95" => Engine::Prime95,
                    "ycruncher" => Engine::YCruncher,
                    _ => return Err(format!("unknown engine '{value}'")),
                }
            }
            "seconds" if value.eq_ignore_ascii_case("auto") => self.seconds_per_test = 0,
            "seconds" => {
                let s: u32 = value
                    .parse()
                    .map_err(|_| "seconds takes a number, or auto".to_string())?;
                if !(10..=3600).contains(&s) {
                    return Err("seconds must be between 10 and 3600, or auto".into());
                }
                self.seconds_per_test = s;
            }
            "search" => {
                self.search =
                    Search::from_id(value).ok_or_else(|| format!("unknown search '{value}'"))?;
            }
            "guardband" => {
                let g: i32 = value
                    .parse()
                    .map_err(|_| "guardband takes a number".to_string())?;
                if !(0..=10).contains(&g) {
                    return Err("guardband must be between 0 and 10".into());
                }
                self.guardband = g;
            }
            "start" => {
                let s: u32 = value
                    .parse()
                    .map_err(|_| "start takes a number".to_string())?;
                if s > 30 {
                    return Err("start must be between 0 and 30".into());
                }
                self.start = s;
            }
            "mode" => {
                self.mode = match value {
                    "sse" => Prime95Mode::Sse,
                    "avx" => Prime95Mode::Avx,
                    "avx2" => Prime95Mode::Avx2,
                    "avx512" => Prime95Mode::Avx512,
                    _ => return Err(format!("unknown mode '{value}'")),
                }
            }
            "on_error" => {
                self.on_error = pbo_core::onerror::OnError::from_id(value)
                    .ok_or_else(|| format!("unknown on_error '{value}' (continue, skip, stop)"))?;
            }
            "notify" => {
                self.notify = match value {
                    "true" | "on" | "yes" => true,
                    "false" | "off" | "no" => false,
                    _ => return Err(format!("notify takes true or false, not '{value}'")),
                };
            }
            "ycruncher" => {
                self.yc_set = pbo_core::engine::YcSet::from_id(value).ok_or_else(|| {
                    format!("unknown ycruncher set '{value}' (curve-optimizer, fast-discovery)")
                })?;
            }
            "order" => {
                self.order = pbo_core::order::CoreOrder::from_id(value).ok_or_else(|| {
                    format!("unknown core order '{value}' (sequential, alternate, random)")
                })?;
            }
            "transient" => {
                self.transient = pbo_core::transient::Transient::from_text(value).ok_or_else(|| {
                    format!("unknown transient '{value}' (off, bursts, periodic or periodic:EVERY:PAUSE)")
                })?;
            }
            "rounds" => {
                let n: u32 = value
                    .parse()
                    .map_err(|_| "rounds takes a number".to_string())?;
                if !(1..=50).contains(&n) {
                    return Err("rounds must be between 1 and 50".into());
                }
                self.rounds = n;
            }
            "then" => {
                let mut steps = Vec::new();
                for part in value.split('+').map(str::trim).filter(|p| !p.is_empty()) {
                    steps.push(
                        crate::plan::Step::from_text(part)
                            .ok_or_else(|| format!("'{part}' is not a step (engine:mode:fft)"))?,
                    );
                }
                self.then = steps;
            }
            "fft" => {
                self.fft = FftPreset::from_name(value)
                    .ok_or_else(|| format!("unknown FFT preset '{value}'"))?;
            }
            "smt" => {
                self.smt = match value {
                    "true" => true,
                    "false" => false,
                    _ => return Err("smt is true or false".into()),
                }
            }
            _ => return Err(format!("unknown key '{key}'")),
        }
        Ok(())
    }

    /// `{"schema_version":1,"cores":"2,5",...}`: every value a string, so a host can store it as it is.
    pub fn to_json(&self) -> String {
        let mut out = format!("{{\"schema_version\":{SCHEMA_VERSION}");
        for (k, v) in self.to_pairs() {
            out.push_str(&format!(",\"{k}\":\"{}\"", escape(&v)));
        }
        out.push('}');
        out
    }

    /// A plan from flat JSON. Keys that are missing keep the recommendation; a key that is wrong is an error.
    pub fn from_json(json: &str) -> Result<Plan, String> {
        let mut plan = Plan::recommended();
        for (k, v) in parse_flat(json)? {
            if k == "schema_version" {
                if v != SCHEMA_VERSION.to_string() {
                    return Err(format!(
                        "schema_version {v} is not supported (this is {SCHEMA_VERSION})"
                    ));
                }
                continue;
            }
            plan.set(&k, &v)?;
        }
        Ok(plan)
    }
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Reads one flat JSON object: strings, numbers, `true`, `false` and `null` as text. Nested values are an error.
pub fn parse_flat(json: &str) -> Result<Vec<(String, String)>, String> {
    let chars: Vec<char> = json.chars().collect();
    let mut i = 0;
    let skip = |i: &mut usize| {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
    };
    let string = |i: &mut usize| -> Result<String, String> {
        if chars.get(*i) != Some(&'"') {
            return Err("a string was expected".into());
        }
        *i += 1;
        let mut out = String::new();
        loop {
            match chars.get(*i) {
                None => return Err("a string is not closed".into()),
                Some('"') => {
                    *i += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    *i += 1;
                    match chars.get(*i) {
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('u') => {
                            let hex: String = chars
                                .get(*i + 1..*i + 5)
                                .ok_or("bad escape")?
                                .iter()
                                .collect();
                            let code = u32::from_str_radix(&hex, 16)
                                .map_err(|_| "bad escape".to_string())?;
                            out.push(char::from_u32(code).ok_or("bad escape")?);
                            *i += 4;
                        }
                        Some(c @ ('"' | '\\' | '/')) => out.push(*c),
                        _ => return Err("bad escape".into()),
                    }
                    *i += 1;
                }
                Some(c) => {
                    out.push(*c);
                    *i += 1;
                }
            }
        }
    };

    skip(&mut i);
    if chars.get(i) != Some(&'{') {
        return Err("a JSON object was expected".into());
    }
    i += 1;
    let mut pairs = Vec::new();
    loop {
        skip(&mut i);
        match chars.get(i) {
            Some('}') => return Ok(pairs),
            Some('"') => {}
            _ => return Err("a key was expected".into()),
        }
        let key = string(&mut i)?;
        skip(&mut i);
        if chars.get(i) != Some(&':') {
            return Err("a colon was expected".into());
        }
        i += 1;
        skip(&mut i);
        let value = match chars.get(i) {
            Some('"') => string(&mut i)?,
            Some('{') | Some('[') => return Err("nested values are not supported".into()),
            Some(_) => {
                let start = i;
                while i < chars.len() && !matches!(chars[i], ',' | '}') && !chars[i].is_whitespace()
                {
                    i += 1;
                }
                chars[start..i].iter().collect()
            }
            None => return Err("the object is not closed".into()),
        };
        pairs.push((key, value));
        skip(&mut i);
        match chars.get(i) {
            Some(',') => i += 1,
            Some('}') => {}
            _ => return Err("a comma or the end was expected".into()),
        }
    }
}

/// The state's name, as it travels in messages.
pub fn state_name(state: &State) -> &'static str {
    match state {
        State::Ready => "Ready",
        State::Preparing => "Preparing",
        State::Running { .. } => "Running",
        State::Stopping => "Stopping",
        State::Finished => "Finished",
        State::Halted(_) => "Halted",
    }
}

/// One line for a host's status bar, in the language handed in.
pub fn status_line(lang: Lang, state: &State) -> String {
    match state {
        State::Ready => text::tr(lang, Key::StateReady).to_string(),
        State::Preparing => text::tr(lang, Key::StatePreparing).to_string(),
        State::Running { phase, core } => text::progress(lang, *phase, *core),
        State::Stopping => text::tr(lang, Key::StateStopping).to_string(),
        State::Finished => text::tr(lang, Key::StateFinished).to_string(),
        State::Halted(HaltReason::StoppedByPerson) => text::tr(lang, Key::StateStopped).to_string(),
        State::Halted(HaltReason::LimitReached) => text::tr(lang, Key::StateLimit).to_string(),
        State::Halted(HaltReason::Inconclusive) => {
            text::tr(lang, Key::StateInconclusive).to_string()
        }
    }
}

/// Phase and core while a core is under test.
pub fn progress(state: &State) -> Option<(u32, u32)> {
    match state {
        State::Running { phase, core } => Some((*phase, *core)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plan_survives_json() {
        let mut p = Plan::recommended();
        p.cores = vec![2, 5];
        p.search = Search::Fine;
        p.seconds_per_test = 90;
        p.smt = true;
        assert_eq!(Plan::from_json(&p.to_json()).unwrap(), p);
    }

    #[test]
    fn the_flat_pairs_come_in_a_fixed_order() {
        let keys: Vec<&str> = Plan::recommended()
            .to_pairs()
            .iter()
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(
            keys,
            [
                "cores",
                "engine",
                "seconds",
                "search",
                "guardband",
                "start",
                "smt",
                "mode",
                "fft",
                "then",
                "rounds",
                "order",
                "transient",
                "on_error",
                "notify",
                "ycruncher"
            ]
        );
    }

    #[test]
    fn a_wrong_value_changes_nothing() {
        let mut p = Plan::recommended();
        assert!(p.set("seconds", "5").is_err());
        assert!(p.set("seconds", "abc").is_err());
        assert!(p.set("cores", "1,x").is_err());
        assert!(p.set("nope", "1").is_err());
        assert_eq!(p, Plan::recommended());
    }

    #[test]
    fn a_newer_schema_is_refused_not_guessed() {
        assert!(Plan::from_json("{\"schema_version\":2}").is_err());
        assert!(Plan::from_json("{\"schema_version\":1}").is_ok());
    }

    #[test]
    fn missing_keys_keep_the_recommendation() {
        assert_eq!(Plan::from_json("{}").unwrap(), Plan::recommended());
    }

    #[test]
    fn flat_json_reads_strings_numbers_and_escapes() {
        let pairs = parse_flat(" { \"a\" : \"x\\\"y\" , \"n\": 12, \"b\":true } ").unwrap();
        assert_eq!(
            pairs,
            vec![
                ("a".to_string(), "x\"y".to_string()),
                ("n".to_string(), "12".to_string()),
                ("b".to_string(), "true".to_string())
            ]
        );
        assert!(parse_flat("{\"a\":{}}").is_err());
        assert!(parse_flat("[1]").is_err());
        assert!(parse_flat("{\"a\":\"x").is_err());
    }

    #[test]
    fn the_status_line_follows_the_state_and_the_language() {
        assert_eq!(status_line(Lang::En, &State::Ready), "Ready");
        assert_eq!(status_line(Lang::De, &State::Ready), "Bereit");
        assert_eq!(
            status_line(Lang::En, &State::Running { phase: 1, core: 3 }),
            "Testing core 3 (step 1)"
        );
        assert_eq!(
            progress(&State::Running { phase: 2, core: 4 }),
            Some((2, 4))
        );
        assert_eq!(progress(&State::Ready), None);
    }
}

/// The strings of `"key":[...]` in a JSON object, for answers that carry a list of lines. Anything that is not a string
/// inside the list is skipped; a missing key is an empty list.
pub fn parse_string_array(json: &str, key: &str) -> Vec<String> {
    let Some(at) = json.find(&format!("\"{key}\":[")) else {
        return Vec::new();
    };
    let chars: Vec<char> = json[at + key.len() + 4..].chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ']' => break,
            '"' => {
                i += 1;
                let mut s = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        match chars[i] {
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            'u' if i + 4 < chars.len() => {
                                let hex: String = chars[i + 1..i + 5].iter().collect();
                                if let Some(c) =
                                    u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
                                {
                                    s.push(c);
                                }
                                i += 4;
                            }
                            c => s.push(c),
                        }
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                out.push(s);
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

#[cfg(test)]
mod array_tests {
    use super::*;

    #[test]
    fn a_list_of_lines_is_read_with_its_escapes() {
        let j = r#"{"ok":true,"lines":["a \"b\"","c\\d",""],"finished":0}"#;
        assert_eq!(parse_string_array(j, "lines"), vec!["a \"b\"", r"c\d", ""]);
        assert!(parse_string_array("{\"ok\":true}", "lines").is_empty());
        assert!(parse_string_array("{\"lines\":[]}", "lines").is_empty());
    }
}
