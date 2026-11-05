//! Runs the language-neutral vectors in `spec/vectors` against the Rust implementation (ADR-0002).
//!
//! Every case of every file is run. An area that this crate does not implement yet is listed as pending, and
//! the list of pending areas is pinned below: a new vector file cannot be added without deciding, here, that
//! Rust does not do it yet. When the list is empty, gate G2 is met for the vectors.

use serde_json::{json, Value};
use std::path::PathBuf;

/// Areas whose vectors Rust does not run yet. Shrinks to nothing by gate G2.
const PENDING: &[&str] = &[];

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../spec/vectors")
}

fn int(v: &Value, name: &str) -> i32 {
    v[name].as_i64().unwrap_or_else(|| panic!("input '{name}' is not an integer: {v}")) as i32
}

fn int_or_none(v: &Value, name: &str) -> Option<i32> {
    match v.get(name) {
        None | Some(Value::Null) => None,
        Some(x) => Some(x.as_i64().unwrap_or_else(|| panic!("input '{name}' is not an integer: {v}")) as i32),
    }
}

fn ints(v: &Value, name: &str) -> Vec<i32> {
    v[name].as_array().unwrap_or_else(|| panic!("input '{name}' is not a list: {v}")).iter().map(|x| x.as_i64().expect("integer") as i32).collect()
}

/// Runs one case. `None` means this function is not implemented here.
fn run(area: &str, function: &str, input: &Value) -> Option<Value> {
    use pbo_core::depth;
    Some(match (area, function) {
        ("curve-optimizer-range", "effective") => {
            json!(depth::effective(int(input, "modelLimit"), int_or_none(input, "userLimit"), &ints(input, "readFromHardware"), int_or_none(input, "appliedFloor")))
        }
        ("curve-optimizer-range", "wasKept") => json!(depth::was_kept(int(input, "requested"), int_or_none(input, "readBack"))),
        _ => return None,
    })
}

/// Numbers by value, objects without regard to key order.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            (x - y).abs() <= 1e-6 * x.abs().max(1.0)
        }
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q)),
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        _ => a == b,
    }
}

#[test]
fn every_vector_holds_in_rust() {
    let mut files: Vec<_> = std::fs::read_dir(vectors_dir()).expect("spec/vectors").filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    assert!(!files.is_empty(), "no vector files found");

    let mut ran = 0usize;
    let mut failures = Vec::new();
    let mut pending: Vec<String> = Vec::new();

    for file in &files {
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let area = doc["area"].as_str().unwrap().to_string();
        let default_kind = doc["kind"].as_str().unwrap().to_string();
        let mut area_ran = 0;
        let mut area_skipped = 0;

        for case in doc["cases"].as_array().unwrap() {
            let kind = case.get("kind").and_then(Value::as_str).unwrap_or(&default_kind);
            assert!(kind == "parity" || kind == "new", "unknown kind '{kind}' in {}", file.display());
            let function = case["function"].as_str().unwrap();
            let name = case["name"].as_str().unwrap();

            match run(&area, function, &case["input"]) {
                Some(actual) => {
                    ran += 1;
                    area_ran += 1;
                    if !same(&case["expected"], &actual) {
                        failures.push(format!("{area}/{function}: {name}\n    expected {}\n    actual   {actual}", case["expected"]));
                    }
                }
                None => area_skipped += 1,
            }
        }

        if area_skipped > 0 {
            assert_eq!(area_ran, 0, "{area} is half implemented: {area_ran} cases ran, {area_skipped} did not");
            pending.push(area);
        }
    }

    assert!(failures.is_empty(), "{} vector(s) failed in Rust:\n  {}", failures.len(), failures.join("\n  "));

    let expected: Vec<String> = PENDING.iter().map(|s| s.to_string()).collect();
    assert_eq!(pending, expected, "the areas Rust does not run yet changed: update PENDING on purpose");
    println!("{ran} vector cases ran in Rust; pending areas: {pending:?}");
}
