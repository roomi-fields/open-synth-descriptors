//! The suite of conformance cases has its own version (`conformance/VERSION`), a checksum of each
//! case file (`conformance/SHA256SUMS`), a list of the optional cases (`conformance/optional.json`)
//! and the contract a reader runs it by (`conformance/RUNNING.md`).

use crate::common;

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn suite() -> PathBuf {
    common::repo().join("conformance")
}

/// Every file under the version folders of the suite, by path under `root`.
fn case_files(root: &Path) -> BTreeMap<String, PathBuf> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                out.insert(p.strip_prefix(root).unwrap().to_str().unwrap().to_string(), p);
            }
        }
    }
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(root).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() && p.file_name().unwrap().to_str().unwrap().starts_with(|c: char| c.is_ascii_digit()) {
            walk(&p, root, &mut out);
        }
    }
    out
}

fn checksum(path: &Path) -> String {
    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap()))
}

/// What is wrong between `root/SHA256SUMS` (`<sha256>  <path under root>` per line) and the case
/// files: a file whose checksum differs, a recorded file that is gone, a file not recorded.
fn sums_faults(root: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(root.join("SHA256SUMS")).expect("conformance/SHA256SUMS");
    let mut recorded = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let (sum, path) = line.split_once("  ").unwrap_or_else(|| panic!("not `<sha256>  <path>`: {line}"));
        recorded.insert(path.to_string(), sum.to_string());
    }
    let files = case_files(root);
    let mut faults = Vec::new();
    for (path, sum) in &recorded {
        match files.get(path) {
            None => faults.push(format!("{path}: recorded, but gone")),
            Some(p) if checksum(p) != *sum => faults.push(format!("{path}: checksum differs")),
            Some(_) => {}
        }
    }
    for path in files.keys().filter(|p| !recorded.contains_key(*p)) {
        faults.push(format!("{path}: not recorded"));
    }
    faults
}

fn small_suite(tmp: &Path) {
    let case = tmp.join("0.6/read/a");
    std::fs::create_dir_all(&case).unwrap();
    std::fs::write(case.join("case.json"), "{\"read\": \"device\"}\n").unwrap();
    std::fs::write(case.join("descriptor.json"), "{}\n").unwrap();
    record(tmp);
}

fn record(tmp: &Path) {
    let lines: Vec<String> = case_files(tmp).iter().map(|(p, f)| format!("{}  {p}", checksum(f))).collect();
    std::fs::write(tmp.join("SHA256SUMS"), lines.join("\n") + "\n").unwrap();
}

#[test]
fn the_suite_version_is_one_line_major_minor_patch() {
    let text = std::fs::read_to_string(suite().join("VERSION")).expect("conformance/VERSION");
    let line = text.strip_suffix('\n').unwrap_or(&text);
    assert!(!line.contains('\n'), "one line: {text:?}");
    let parts: Vec<&str> = line.split('.').collect();
    assert_eq!(parts.len(), 3, "{line:?}");
    for p in parts {
        assert!(!p.is_empty() && p.chars().all(|c| c.is_ascii_digit()), "{line:?}");
    }
}

#[test]
fn every_case_file_of_the_suite_has_the_checksum_recorded_and_none_recorded_is_gone() {
    let faults = sums_faults(&suite());
    assert!(faults.is_empty(), "{} faults:\n{}", faults.len(), faults.iter().take(10).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn the_check_of_the_checksums_refuses_a_changed_file_a_gone_file_and_an_unrecorded_file() {
    let tmp = tempfile::tempdir().unwrap();
    small_suite(tmp.path());
    assert!(sums_faults(tmp.path()).is_empty());

    std::fs::write(tmp.path().join("0.6/read/a/descriptor.json"), "{\"x\": 1}\n").unwrap();
    assert_eq!(sums_faults(tmp.path()), vec!["0.6/read/a/descriptor.json: checksum differs"]);

    record(tmp.path());
    std::fs::write(tmp.path().join("0.6/read/a/more.json"), "{}\n").unwrap();
    assert_eq!(sums_faults(tmp.path()), vec!["0.6/read/a/more.json: not recorded"]);

    record(tmp.path());
    std::fs::remove_file(tmp.path().join("0.6/read/a/more.json")).unwrap();
    assert_eq!(sums_faults(tmp.path()), vec!["0.6/read/a/more.json: recorded, but gone"]);
}

// ---- the optional cases ----

fn has_key(value: &Value, name: &str) -> bool {
    match value {
        Value::Object(m) => m.iter().any(|(k, v)| k == name || has_key(v, name)),
        Value::Array(a) => a.iter().any(|v| has_key(v, name)),
        _ => false,
    }
}

fn json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            json_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
}

/// Whether a case is optional: `case.json` carries `identity_support` or `identities`, or a
/// descriptor of the case holds a JSON key named `lua` at any depth.
fn is_optional(dir: &Path) -> bool {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    if case.get("identity_support").is_some() || case.get("identities").is_some() {
        return true;
    }
    let mut files = Vec::new();
    json_files(dir, &mut files);
    files.iter().filter(|f| **f != dir.join("case.json")).any(|f| {
        // A descriptor whose text is not JSON (a syntax case) holds no key.
        serde_json::from_str::<Value>(&std::fs::read_to_string(f).unwrap()).is_ok_and(|doc| has_key(&doc, "lua"))
    })
}

fn case_dirs(root: &Path) -> Vec<String> {
    case_files(root)
        .keys()
        .filter(|p| p.ends_with("/case.json") && p.split('/').count() == 4)
        .map(|p| p.trim_end_matches("/case.json").to_string())
        .collect()
}

fn listed_optional() -> Vec<String> {
    let text = std::fs::read_to_string(suite().join("optional.json")).expect("conformance/optional.json");
    let list: Value = serde_json::from_str(&text).unwrap();
    list.as_array().expect("an array of paths").iter().map(|v| v.as_str().expect("a path").to_string()).collect()
}

#[test]
fn the_optional_list_holds_every_case_with_identities_or_a_lua_key_and_each_listed_path_exists() {
    let root = suite();
    let listed: BTreeSet<String> = listed_optional().into_iter().collect();
    let found: Vec<String> = case_dirs(&root).into_iter().filter(|d| is_optional(&root.join(d))).collect();
    assert!(found.len() > 10, "{} optional cases found", found.len());
    let missing: Vec<&String> = found.iter().filter(|d| !listed.contains(*d)).collect();
    assert!(missing.is_empty(), "optional cases not listed: {missing:?}");
    for path in &listed {
        assert!(root.join(path).join("case.json").is_file(), "{path} is listed but is no case");
    }
}

#[test]
fn a_case_that_states_identities_or_holds_a_lua_key_is_optional_and_another_is_not() {
    let tmp = tempfile::tempdir().unwrap();
    let write = |name: &str, case: &str, descriptor: &str| {
        let d = tmp.path().join(name);
        std::fs::create_dir_all(d.join("files/acme")).unwrap();
        std::fs::write(d.join("case.json"), case).unwrap();
        std::fs::write(d.join("files/acme/a.json"), descriptor).unwrap();
        d
    };
    assert!(is_optional(&write("support", r#"{"identity_support": []}"#, "{}")));
    assert!(is_optional(&write("identities", r#"{"identities": []}"#, "{}")));
    assert!(is_optional(&write("lua", r#"{"read": "device"}"#, r#"{"a": [{"b": {"lua": "x"}}]}"#)));
    assert!(!is_optional(&write("plain", r#"{"read": "device"}"#, r#"{"a": "lua"}"#)));
}

// ---- the contract ----

fn running() -> String {
    std::fs::read_to_string(suite().join("RUNNING.md")).expect("conformance/RUNNING.md")
}

#[test]
fn running_says_how_a_reader_is_run_on_a_case_and_how_its_result_is_reported() {
    let text = running();
    for needle in ["case folder", "standard output", "exit", "`result`", "`fault`", "passed", "failed", "skipped"] {
        assert!(text.contains(needle), "RUNNING.md does not say {needle}");
    }
}

#[test]
fn running_lists_every_kind_of_fault_a_case_uses() {
    let root = suite();
    let mut kinds = BTreeSet::new();
    for dir in case_dirs(&root) {
        let case: Value = serde_json::from_str(&std::fs::read_to_string(root.join(&dir).join("case.json")).unwrap()).unwrap();
        if let Some(k) = case.get("fault").and_then(|f| f.get("kind")).and_then(Value::as_str) {
            kinds.insert(k.to_string());
        }
    }
    assert!(kinds.len() >= 8, "{kinds:?}");
    let text = running();
    for kind in kinds {
        assert!(text.contains(&format!("`{kind}`")), "the kind `{kind}` is not in RUNNING.md");
    }
}

#[test]
fn running_says_the_optional_cases_are_skipped_and_a_claim_without_them_says_so() {
    let text = running();
    assert!(text.contains("optional.json"), "the list of optional cases");
    assert!(text.contains("without the optional cases"), "the claim of a reader that skips them");
}

#[test]
fn running_writes_the_rule_of_the_suite_version_and_what_a_claim_names() {
    let text = running();
    for needle in ["VERSION", "SHA256SUMS", "minor", "patch", "never changed", "version of the format", "version of the suite"] {
        assert!(text.contains(needle), "RUNNING.md does not say {needle}");
    }
}
