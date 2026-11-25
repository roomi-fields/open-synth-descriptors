//! The runner of `conformance/RUNNING.md`: a reader is an executable that takes one case folder
//! as its only argument and writes one JSON object on its standard output; the runner compares
//! that object to the case, and reports. It uses nothing of `osd`: the adapter of the reference
//! reader and the toy reader of the tests are judged by it alike.

#![allow(dead_code)]

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Keys of `case.json` that expect nothing: the kind selectors and the inputs a case states.
const NOT_EXPECTATIONS: [&str; 11] = [
    "read", "convert", "resolve", "resolve_assembly", "coding", "bytes", "judge_needs", "match_identity",
    "same_product_instance_id", "device", "schema_pointer",
];
/// Keys of `case.json` about the schema, not the reader.
const ABOUT_THE_SCHEMA: [&str; 2] = ["schema_accepts", "schema_refused"];

/// The keys of a case a reader's output is compared on.
pub fn expectation_keys(case: &Value) -> Vec<String> {
    case.as_object()
        .unwrap()
        .keys()
        .filter(|k| !k.starts_with('_') && !NOT_EXPECTATIONS.contains(&k.as_str()) && !ABOUT_THE_SCHEMA.contains(&k.as_str()))
        .cloned()
        .collect()
}

/// Whether two JSON values are the same: objects key order aside, arrays in order, numbers by
/// value, strings exact.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        (Value::Array(x), Value::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(v, w)| same(v, w)),
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        _ => a == b,
    }
}

/// The comparison of RUNNING.md: what the case expects, key by key, against what the reader
/// wrote. `Err` says the first difference. A fault is compared by `kind`, `pointer` and, for a
/// resolve case, `file`; keys the case does not expect are ignored in the output.
pub fn judge(case: &Value, got: &Value) -> Result<(), String> {
    let resolves = case.get("resolve").is_some() || case.get("resolve_assembly").is_some();
    for key in expectation_keys(case) {
        let want = &case[key.as_str()];
        let Some(have) = got.get(&key) else { return Err(format!("`{key}` missing from the output")) };
        if key == "fault" {
            let mut parts = vec!["kind", "pointer"];
            if resolves {
                parts.push("file");
            }
            for part in parts {
                let (w, h) = (want.get(part), have.get(part));
                let equal = match (w, h) {
                    (None, None) => true,
                    (Some(w), Some(h)) => same(w, h),
                    _ => false,
                };
                if !equal {
                    return Err(format!("fault {part}: expected {w:?}, got {h:?}"));
                }
            }
        } else if !same(want, have) {
            return Err(format!("`{key}`: expected {want}, got {have}"));
        }
    }
    Ok(())
}

/// A reader: the program and the arguments that come before the case folder.
pub struct Reader {
    pub program: PathBuf,
    pub args: Vec<String>,
}

/// What a reader wrote on a case: its output object, or why it could not be read.
pub fn run_case(reader: &Reader, dir: &Path) -> Result<Value, String> {
    let out = Command::new(&reader.program).args(&reader.args).arg(dir).output().map_err(|e| format!("cannot run the reader: {e}"))?;
    if out.status.code() != Some(0) {
        return Err(format!("exit {:?}: {}", out.status.code(), String::from_utf8_lossy(&out.stderr)));
    }
    let got: Value = serde_json::from_slice(&out.stdout).map_err(|e| format!("the standard output is not JSON: {e}"))?;
    if got.is_object() {
        Ok(got)
    } else {
        Err("the standard output is not one JSON object".into())
    }
}

/// The cases of a version folder: `<version>/<kind>/<case>/`, sorted.
pub fn cases_of(version_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for kind in sorted(version_dir) {
        for case in sorted(&kind) {
            if case.join("case.json").is_file() {
                out.push(case);
            }
        }
    }
    out
}

fn sorted(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

/// The versions the suite holds a folder for, sorted.
pub fn versions(root: &Path) -> Vec<String> {
    sorted(root)
        .into_iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap().to_string())
        .filter(|n| n.starts_with(|c: char| c.is_ascii_digit()))
        .collect()
}

/// The kind of case a folder is: the name of its kind folder (`read`, `vocabulary`…).
pub fn kind_folder(case: &Path) -> String {
    case.parent().unwrap().file_name().unwrap().to_str().unwrap().to_string()
}

/// What a run of the suite reports: the version of the format, the version of the suite, and the
/// counts.
pub struct Report {
    pub format_version: String,
    pub suite_version: String,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub failures: Vec<String>,
}

impl Report {
    /// The line a runner writes.
    pub fn line(&self) -> String {
        format!(
            "format {} suite {}: {} passed, {} failed, {} skipped",
            self.format_version, self.suite_version, self.passed, self.failed, self.skipped
        )
    }
}

/// Runs `cases` (folders under `root`) of the format `version` on `reader`; a case whose path
/// under `root` is in `optional` is skipped, never passed.
pub fn run_cases(reader: &Reader, root: &Path, version: &str, cases: &[PathBuf], optional: &BTreeSet<String>) -> Report {
    let suite_version = std::fs::read_to_string(root.join("VERSION")).expect("conformance/VERSION").trim_end().to_string();
    let mut report = Report { format_version: version.to_string(), suite_version, passed: 0, failed: 0, skipped: 0, failures: Vec::new() };
    for dir in cases {
        let rel = dir.strip_prefix(root).unwrap().to_str().unwrap().to_string();
        if optional.contains(&rel) {
            report.skipped += 1;
            continue;
        }
        let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        match run_case(reader, dir).and_then(|got| judge(&case, &got)) {
            Ok(()) => report.passed += 1,
            Err(why) => {
                report.failed += 1;
                report.failures.push(format!("{rel}: {why}"));
            }
        }
    }
    report
}
