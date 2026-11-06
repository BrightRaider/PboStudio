//! The files the program keeps, and where (ADR-0008).
//!
//! Everything lives in `runs/` next to the program, in the formats the 1.x program writes, so what a person has
//! measured over hours or days carries over. Writing is atomic: the new content goes into a neighbour file, is forced
//! to disk, and replaces the old one in a single rename, so a crash in the middle leaves the old file and never half
//! of a new one.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pbo_core::knowledge::{CoreKnowledge, Knowledge};
use serde_json::{json, Value};

pub struct Store {
    pub root: PathBuf,
}

/// The program's folder: the one the executable is in.
pub fn program_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension(format!("tmp.{}", std::process::id()));
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temp, path)
}

impl Store {
    pub fn beside_program() -> Store {
        Store {
            root: program_dir().join("runs"),
        }
    }

    #[cfg(test)]
    pub fn at(root: PathBuf) -> Store {
        Store { root }
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    fn read_json(&self, name: &str) -> Option<Value> {
        let bytes = fs::read(self.root.join(name)).ok()?;
        // The 1.x files are written with a byte order mark by some tools; serde_json does not take one.
        let text = String::from_utf8_lossy(&bytes);
        serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
    }

    fn write_json(&self, name: &str, value: &Value) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(value).unwrap_or_default();
        atomic_write(&self.root.join(name), text.as_bytes())
    }

    // ── What each core has been seen to hold and to fail at ───────────────

    /// The memory of the search and the BIOS version it was gathered under. A missing or unreadable file is an empty
    /// memory: losing it is a setback, never a reason to refuse to start.
    pub fn load_knowledge(&self) -> (Knowledge, String) {
        let Some(doc) = self.read_json("core_knowledge.json") else {
            return (Knowledge::new(), String::new());
        };
        // The newer shape wraps the cores with a BIOS stamp; the older one is a bare map of cores.
        let (cores, bios) = match doc.get("Cores") {
            Some(c) => (
                c.clone(),
                doc["BiosVersion"].as_str().unwrap_or("").to_string(),
            ),
            None => (doc.clone(), String::new()),
        };
        let mut knowledge = Knowledge::new();
        if let Some(map) = cores.as_object() {
            for (key, v) in map {
                let Ok(core) = key.parse::<u32>() else {
                    continue;
                };
                knowledge.insert(
                    core,
                    CoreKnowledge {
                        best_passed: v["BestPassed"].as_i64().map(|n| n as i32),
                        worst_failed: v["WorstFailed"].as_i64().map(|n| n as i32),
                    },
                );
            }
        }
        (knowledge, bios)
    }

    pub fn save_knowledge(&self, knowledge: &Knowledge, bios: &str) -> std::io::Result<()> {
        let mut cores = serde_json::Map::new();
        for (core, k) in knowledge {
            cores.insert(
                core.to_string(),
                json!({ "BestPassed": k.best_passed, "WorstFailed": k.worst_failed }),
            );
        }
        self.write_json(
            "core_knowledge.json",
            &json!({ "BiosVersion": bios, "Cores": cores }),
        )
    }

    // ── The profile that is applied at logon ───────────────────────────────

    /// The saved profile as the file has it: the processor it was saved for and a value per core number. Core numbers
    /// and values are not judged here; `pbo_core::profile::check` decides what may be written.
    pub fn load_saved_profile(&self) -> Option<(String, std::collections::BTreeMap<i32, i32>)> {
        let doc = self.read_json("co_saved.json")?;
        let cpu = doc["CpuName"].as_str()?.to_string();
        let map = doc["CoreMargins"].as_object()?;
        let mut margins = std::collections::BTreeMap::new();
        for (key, v) in map {
            margins.insert(key.parse::<i32>().ok()?, i32::try_from(v.as_i64()?).ok()?);
        }
        Some((cpu, margins))
    }

    pub fn save_profile(
        &self,
        cpu_name: &str,
        margins: &[i32],
        saved_at: &str,
    ) -> std::io::Result<()> {
        let mut map = serde_json::Map::new();
        for (core, m) in margins.iter().enumerate() {
            map.insert(core.to_string(), json!(m));
        }
        self.write_json(
            "co_saved.json",
            &json!({ "SavedAt": saved_at, "CpuName": cpu_name, "CoreMargins": map }),
        )
    }

    // ── What the window remembers: theme, language, the plan ──────────────

    /// What the window remembered; each is `None` when it was never saved.
    pub fn load_window(&self) -> WindowFile {
        let Some(doc) = self.read_json("window.json") else {
            return WindowFile::default();
        };
        WindowFile {
            dark: doc["dark"].as_bool(),
            language: doc["language"].as_str().map(str::to_string),
            plan: doc["plan"].as_str().map(str::to_string),
            intensity: doc["intensity"].as_str().map(str::to_string),
            auto_step: doc["auto_step"].as_u64().map(|n| n as u32),
        }
    }

    pub fn save_window(&self, file: &WindowFile) -> std::io::Result<()> {
        self.write_json(
            "window.json",
            &json!({
                "schema_version": 1,
                "dark": file.dark,
                "language": file.language,
                "plan": file.plan,
                "intensity": file.intensity,
                "auto_step": file.auto_step,
            }),
        )
    }

    // ── The run that is going on, so that a crash can be told from a closed window ──

    pub fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn write_run_state(&self, state: &RunState) -> std::io::Result<()> {
        self.write_json("state.json", &state.to_json())
    }

    pub fn read_run_state(&self) -> Option<RunState> {
        RunState::from_json(&self.read_json("state.json")?)
    }

    pub fn clear_run_state(&self) {
        let _ = fs::remove_file(self.state_path());
    }

    /// When the state file was last written, as Unix seconds.
    pub fn run_state_written(&self) -> Option<i64> {
        let modified = fs::metadata(self.state_path()).ok()?.modified().ok()?;
        Some(modified.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
    }
}

/// What the window remembers: the theme, the language, the plan and how thorough Auto is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowFile {
    pub dark: Option<bool>,
    pub language: Option<String>,
    pub plan: Option<String>,
    pub intensity: Option<String>,
    /// The steps Auto searches in when the person chose their own; none: those of the intensity.
    pub auto_step: Option<u32>,
}

/// What the run that is going on is doing at this moment. The value is the one that is running, not whatever the
/// processor holds after a restart (ADR-0008, point 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunState {
    pub run_id: String,
    pub core: u32,
    /// The value being tested on that core.
    pub margin: i32,
    /// What every core held before the run changed anything, to put back.
    pub baseline: Vec<i32>,
    pub started_utc: String,
}

impl RunState {
    fn to_json(&self) -> Value {
        json!({
            "schema_version": 1,
            "run_id": self.run_id,
            "core": self.core,
            "margin": self.margin,
            "baseline": self.baseline,
            "started_utc": self.started_utc,
        })
    }

    fn from_json(v: &Value) -> Option<RunState> {
        Some(RunState {
            run_id: v["run_id"].as_str()?.to_string(),
            core: v["core"].as_u64()? as u32,
            margin: v["margin"].as_i64()? as i32,
            baseline: v["baseline"]
                .as_array()?
                .iter()
                .filter_map(|n| n.as_i64().map(|n| n as i32))
                .collect(),
            started_utc: v["started_utc"].as_str()?.to_string(),
        })
    }
}

/// Unix seconds now.
pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("pbo-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        Store::at(dir)
    }

    #[test]
    fn knowledge_round_trips_in_the_1x_shape() {
        let store = temp_store("k");
        let mut k = Knowledge::new();
        k.insert(
            0,
            CoreKnowledge {
                best_passed: Some(-20),
                worst_failed: None,
            },
        );
        k.insert(
            3,
            CoreKnowledge {
                best_passed: Some(-24),
                worst_failed: Some(-27),
            },
        );
        store.save_knowledge(&k, "F20").unwrap();
        let text = fs::read_to_string(store.root.join("core_knowledge.json")).unwrap();
        assert!(text.contains("\"BestPassed\"") && text.contains("\"BiosVersion\""));
        let (back, bios) = store.load_knowledge();
        assert_eq!(back, k);
        assert_eq!(bios, "F20");
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn a_bare_core_map_from_before_the_bios_stamp_is_read() {
        let store = temp_store("bare");
        fs::create_dir_all(&store.root).unwrap();
        fs::write(
            store.root.join("core_knowledge.json"),
            "\u{feff}{\"0\":{\"BestPassed\":-12,\"WorstFailed\":null}}",
        )
        .unwrap();
        let (k, bios) = store.load_knowledge();
        assert_eq!(k[&0].best_passed, Some(-12));
        assert_eq!(bios, "");
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn a_missing_or_broken_file_is_an_empty_memory() {
        let store = temp_store("broken");
        assert!(store.load_knowledge().0.is_empty());
        fs::create_dir_all(&store.root).unwrap();
        fs::write(store.root.join("core_knowledge.json"), "{ not json").unwrap();
        assert!(store.load_knowledge().0.is_empty());
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn the_saved_profile_of_1x_is_read() {
        let store = temp_store("profile");
        fs::create_dir_all(&store.root).unwrap();
        fs::write(
            store.root.join("co_saved.json"),
            "{\"SavedAt\":\"x\",\"CpuName\":\"AMD Ryzen 7 5800X3D\",\"CoreMargins\":{\"0\":-30,\"1\":-25}}",
        )
        .unwrap();
        assert_eq!(
            store.load_saved_profile(),
            Some((
                "AMD Ryzen 7 5800X3D".to_string(),
                [(0, -30), (1, -25)].into()
            ))
        );
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn run_state_round_trips_and_clears() {
        let store = temp_store("state");
        let s = RunState {
            run_id: "ab12".into(),
            core: 4,
            margin: -27,
            baseline: vec![-20, -25],
            started_utc: "2026-11-05T10:00:00.000Z".into(),
        };
        store.write_run_state(&s).unwrap();
        assert_eq!(store.read_run_state(), Some(s));
        assert!(store.run_state_written().is_some());
        store.clear_run_state();
        assert_eq!(store.read_run_state(), None);
        let _ = fs::remove_dir_all(&store.root);
    }

    #[test]
    fn a_write_leaves_no_temporary_file_behind() {
        let store = temp_store("atomic");
        store.save_knowledge(&Knowledge::new(), "").unwrap();
        let leftovers = fs::read_dir(&store.root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .count();
        assert_eq!(leftovers, 0);
        let _ = fs::remove_dir_all(&store.root);
    }
}

/// What `pbo import` did with one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Imported {
    Copied(String),
    /// Left alone because the destination already has a file of that name.
    Kept(String),
}

/// The files of a 1.x program that are worth keeping, by name. `state.json` is left out on purpose: it describes a run
/// of the other program, and reading it as a crash of this one would put a failure into the memory that never happened.
const IMPORTABLE: [&str; 6] = [
    "core_knowledge.json",
    "co_saved.json",
    "campaign.json",
    "settings.json",
    "whea_seen.json",
    "whea_reset.txt",
];

impl Store {
    /// Copies what a 1.x install keeps into this store. `from` is the install folder, its `runs` folder, or its
    /// `engines/runs` folder. Nothing is overwritten and nothing is deleted in either place.
    pub fn import_from(&self, from: &Path) -> std::io::Result<Vec<Imported>> {
        let mut places = vec![
            from.to_path_buf(),
            from.join("runs"),
            from.join("engines").join("runs"),
        ];
        places.dedup();
        let mut done = Vec::new();
        for place in places.iter().filter(|p| p.is_dir()) {
            for name in IMPORTABLE {
                let source = place.join(name);
                if !source.is_file() {
                    continue;
                }
                let target = self.root.join(name);
                if target.exists()
                    || done
                        .iter()
                        .any(|d: &Imported| matches!(d, Imported::Copied(n) if n == name))
                {
                    done.push(Imported::Kept(name.to_string()));
                } else {
                    fs::create_dir_all(&self.root)?;
                    fs::copy(&source, &target)?;
                    done.push(Imported::Copied(name.to_string()));
                }
            }
            let logs = place.join("logs");
            if logs.is_dir() {
                for entry in fs::read_dir(&logs)?.filter_map(Result::ok) {
                    let target = self.logs_dir().join(entry.file_name());
                    let label = format!("logs/{}", entry.file_name().to_string_lossy());
                    if !entry.path().is_file() {
                        continue;
                    }
                    if target.exists() {
                        done.push(Imported::Kept(label));
                    } else {
                        fs::create_dir_all(self.logs_dir())?;
                        fs::copy(entry.path(), &target)?;
                        done.push(Imported::Copied(label));
                    }
                }
            }
        }
        Ok(done)
    }
}

#[cfg(test)]
mod import_tests {
    use super::*;

    #[test]
    fn a_1x_folder_is_copied_without_overwriting() {
        let base = std::env::temp_dir().join(format!("pbo-import-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let old = base.join("old");
        fs::create_dir_all(old.join("runs").join("logs")).unwrap();
        fs::create_dir_all(old.join("engines").join("runs")).unwrap();
        fs::write(old.join("runs").join("core_knowledge.json"), "{}").unwrap();
        fs::write(old.join("runs").join("co_saved.json"), "{}").unwrap();
        fs::write(old.join("runs").join("logs").join("session_a.log"), "x").unwrap();
        fs::write(old.join("engines").join("runs").join("state.json"), "{}").unwrap();
        let store = Store::at(base.join("new"));
        fs::create_dir_all(&store.root).unwrap();
        fs::write(store.root.join("co_saved.json"), "mine").unwrap();

        let done = store.import_from(&old).unwrap();
        assert!(done.contains(&Imported::Copied("core_knowledge.json".into())));
        assert!(done.contains(&Imported::Kept("co_saved.json".into())));
        assert!(done.contains(&Imported::Copied("logs/session_a.log".into())));
        assert_eq!(
            fs::read_to_string(store.root.join("co_saved.json")).unwrap(),
            "mine"
        );
        // The other program's run state is never taken over.
        assert!(!store.root.join("state.json").exists());
        let _ = fs::remove_dir_all(&base);
    }
}
