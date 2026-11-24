//! A reader runs the conformance suite as `conformance/RUNNING.md` says: the word `conformance`
//! of `osd` is the adapter of the reference reader, a toy reader proves the contract needs
//! nothing of `osd`, and one runner judges both.

use crate::common::conformance_runner::{cases_of, judge, kind_folder, run_case, run_cases, versions, Reader};
use crate::common::{osd, repo};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

fn adapter() -> Reader {
    Reader { program: PathBuf::from(env!("CARGO_BIN_EXE_osd")), args: vec!["conformance".to_string()] }
}

fn case_json(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap()
}

fn suite() -> PathBuf {
    repo().join("conformance")
}

// ---- the comparison ----

#[test]
fn the_comparison_accepts_objects_in_another_key_order_and_numbers_by_value_and_ignores_extra_output_keys() {
    let case = json!({"_about": "x", "read": "device", "result": {"a": 1, "b": [1, 2]}});
    let got = json!({"extra": true, "result": {"b": [1, 2], "a": 1.0}});
    assert_eq!(judge(&case, &got), Ok(()));
}

#[test]
fn the_comparison_reports_a_wrong_value_as_failed() {
    let case = json!({"read": "device", "result": {"a": 1, "b": [1, 2]}});
    assert!(judge(&case, &json!({"result": {"a": 2, "b": [1, 2]}})).is_err());
    assert!(judge(&case, &json!({"result": {"a": 1, "b": [2, 1]}})).is_err(), "arrays are compared in order");
    assert!(judge(&json!({"read": "device", "result": "Abc"}), &json!({"result": "abc"})).is_err(), "strings are exact");
    assert!(judge(&case, &json!({})).is_err(), "a missing key");
}

#[test]
fn the_comparison_reports_a_wrong_pointer_and_a_wrong_fault_kind_as_failed() {
    let case = json!({"read": "device", "fault": {"kind": "BadValue", "pointer": "/a"}});
    assert_eq!(judge(&case, &json!({"fault": {"kind": "BadValue", "pointer": "/a"}})), Ok(()));
    assert!(judge(&case, &json!({"fault": {"kind": "BadValue", "pointer": "/b"}})).is_err());
    assert!(judge(&case, &json!({"fault": {"kind": "Missing", "pointer": "/a"}})).is_err());
    assert!(judge(&case, &json!({"result": {}})).is_err(), "a result where a fault is expected");
}

#[test]
fn the_comparison_of_a_resolve_case_reads_the_file_of_the_fault() {
    let case = json!({"resolve": "acme/a", "fault": {"kind": "LinkMissing", "file": "acme/a", "pointer": "/midi_of"}});
    assert_eq!(judge(&case, &json!({"fault": {"kind": "LinkMissing", "file": "acme/a", "pointer": "/midi_of"}})), Ok(()));
    assert!(judge(&case, &json!({"fault": {"kind": "LinkMissing", "file": "acme/b", "pointer": "/midi_of"}})).is_err());
}

#[test]
fn the_comparison_leaves_aside_the_about_the_selector_and_the_schema_keys() {
    let case = json!({
        "_about": "x", "_note": "y", "read": "device", "schema_pointer": "/p", "schema_accepts": true,
        "schema_refused": [{"file": "a", "pointer": "/p"}], "result": 1
    });
    assert_eq!(judge(&case, &json!({"result": 1})), Ok(()));
}

#[test]
fn the_comparison_reads_every_expectation_key_of_the_case() {
    let case = json!({"read": "device", "result": {}, "roles": [{"pointer": "/p"}], "identities": [1]});
    assert!(judge(&case, &json!({"result": {}, "identities": [1]})).is_err());
    assert!(judge(&case, &json!({"result": {}, "roles": [{"pointer": "/p"}], "identities": [2]})).is_err());
    assert_eq!(judge(&case, &json!({"result": {}, "roles": [{"pointer": "/p"}], "identities": [1]})), Ok(()));
}

// ---- the toy reader ----

/// A toy reader: a few lines of shell, nothing of `osd`. It writes a read case's descriptor back
/// as the result, and converts a 0.5 descriptor by naming the 0.6 schema.
const TOY: &str = r#"#!/bin/sh
case_dir="$1"
if grep -q '"read" *:' "$case_dir/case.json"; then
  printf '{"result": %s}\n' "$(cat "$case_dir/descriptor.json")"
elif grep -q '"convert" *:' "$case_dir/case.json"; then
  printf '{"result": %s}\n' "$(sed 's|osd/0\.5|osd/0.6|' "$case_dir/descriptor.json")"
else
  echo "the toy reads only read and convert cases" >&2
  exit 2
fi
"#;

fn toy(dir: &Path) -> Reader {
    let path = dir.join("toy.sh");
    std::fs::write(&path, TOY).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    Reader { program: path, args: Vec::new() }
}

/// A 0.6 reading case whose result is its descriptor as written.
fn read_case_written_back() -> PathBuf {
    cases_of(&suite().join("0.6"))
        .into_iter()
        .filter(|d| kind_folder(d) == "read")
        .find(|d| {
            let case = case_json(d);
            let descriptor: Value = serde_json::from_str(&std::fs::read_to_string(d.join("descriptor.json")).unwrap()).unwrap();
            case.get("result") == Some(&descriptor) && expectation_count(&case) == 1
        })
        .expect("a 0.6 reading case that writes its descriptor back")
}

fn expectation_count(case: &Value) -> usize {
    crate::common::conformance_runner::expectation_keys(case).len()
}

#[test]
fn a_toy_reader_that_uses_nothing_of_osd_passes_a_read_case_through_the_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let reader = toy(tmp.path());
    let dir = read_case_written_back();
    let got = run_case(&reader, &dir).unwrap();
    assert_eq!(judge(&case_json(&dir), &got), Ok(()), "{}", dir.display());
}

#[test]
fn a_toy_reader_passes_a_conversion_case_through_the_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let reader = toy(tmp.path());
    let dir = suite().join("0.6/convert/fingerprint-named-device-converts");
    let got = run_case(&reader, &dir).unwrap();
    assert_eq!(judge(&case_json(&dir), &got), Ok(()));
}

#[test]
fn the_runner_fails_a_toy_reader_on_a_case_it_does_not_read() {
    let tmp = tempfile::tempdir().unwrap();
    let reader = toy(tmp.path());
    let dir = suite().join("0.6/vocabulary").join(std::fs::read_dir(suite().join("0.6/vocabulary")).unwrap().next().unwrap().unwrap().file_name());
    assert!(run_case(&reader, &dir).is_err());
}

#[test]
fn the_report_names_the_format_and_the_suite_versions_with_the_counts_of_passed_failed_and_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let reader = toy(tmp.path());
    let root = suite();
    let read = read_case_written_back();
    let convert = root.join("0.6/convert/fingerprint-named-device-converts");
    let unread = root.join("0.6/judge_needs").join(std::fs::read_dir(root.join("0.6/judge_needs")).unwrap().next().unwrap().unwrap().file_name());
    let skipped = root.join("0.6/vocabulary").join(std::fs::read_dir(root.join("0.6/vocabulary")).unwrap().next().unwrap().unwrap().file_name());
    let optional: BTreeSet<String> = [skipped.strip_prefix(&root).unwrap().to_str().unwrap().to_string()].into();
    let report = run_cases(&reader, &root, "0.6", &[read, convert, unread, skipped], &optional);
    assert_eq!((report.passed, report.failed, report.skipped), (2, 1, 1), "{:?}", report.failures);
    let suite_version = std::fs::read_to_string(root.join("VERSION")).unwrap();
    assert_eq!(report.line(), format!("format 0.6 suite {}: 2 passed, 1 failed, 1 skipped", suite_version.trim_end()));
}

// ---- the adapter of osd ----

#[test]
fn the_adapter_passes_every_case_of_every_published_version_with_the_comparison_of_the_runner() {
    let root = suite();
    let mut failures = Vec::new();
    let mut kinds: BTreeSet<String> = BTreeSet::new();
    let mut count = 0;
    for version in versions(&root) {
        for dir in cases_of(&root.join(&version)) {
            let case = case_json(&dir);
            let kind = if case.get("resolve_assembly").is_some() { "resolve_assembly".to_string() } else { kind_folder(&dir) };
            kinds.insert(kind);
            count += 1;
            if let Err(why) = run_case(&adapter(), &dir).and_then(|got| judge(&case, &got)) {
                failures.push(format!("{}: {why}", dir.strip_prefix(&root).unwrap().display()));
            }
        }
    }
    assert!(count > 400, "{count} cases found");
    for kind in ["read", "convert", "resolve", "resolve_assembly", "coding", "judge_needs", "vocabulary"] {
        assert!(kinds.contains(kind), "no case of kind {kind} run: {kinds:?}");
    }
    crate::common::report("cases the adapter does not pass", failures);
}

#[test]
fn the_adapter_writes_a_fault_for_a_case_that_expects_one_and_exits_0() {
    let dir = suite().join("0.6/convert/control-mixing-cc-and-nrpn-refused");
    let run = osd(&["conformance", dir.to_str().unwrap()]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let got: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(got["fault"]["kind"], "NotConvertible");
    assert_eq!(got["fault"]["pointer"], "/cc_params/entries/0");
    assert!(got.get("result").is_none(), "{got}");
}

#[test]
fn the_adapter_refuses_a_folder_that_is_not_a_case_with_exit_2_and_no_result() {
    let tmp = tempfile::tempdir().unwrap();
    let empty = tmp.path().join("0.6/read/none");
    std::fs::create_dir_all(&empty).unwrap();
    let missing = tmp.path().join("absent");
    for path in [&empty, &missing, &tmp.path().to_path_buf()] {
        let run = osd(&["conformance", path.to_str().unwrap()]);
        assert_eq!(run.code, 2, "{}: {}", path.display(), run.stdout);
        assert_eq!(run.stdout, "", "{}", path.display());
        assert!(!run.stderr.trim().is_empty(), "{}", path.display());
    }
}

#[test]
fn the_adapter_refuses_a_case_of_a_kind_it_does_not_know_with_exit_2_and_no_result() {
    let tmp = tempfile::tempdir().unwrap();
    for (folder, text) in [
        ("0.6/frobnicate/selector", r#"{"_about": "x", "frobnicate": "device", "result": {}}"#),
        ("0.6/frobnicate/none", r#"{"_about": "x", "result": {}}"#),
    ] {
        let dir = tmp.path().join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("case.json"), text).unwrap();
        let run = osd(&["conformance", dir.to_str().unwrap()]);
        assert_eq!(run.code, 2, "{folder}: {}", run.stdout);
        assert_eq!(run.stdout, "", "{folder}");
        assert!(!run.stderr.trim().is_empty(), "{folder}");
    }
}

#[test]
fn the_adapter_takes_one_case_folder_as_its_only_argument() {
    let dir = suite().join("0.6/convert/control-mixing-cc-and-nrpn-refused");
    let one = dir.to_str().unwrap();
    for args in [vec!["conformance"], vec!["conformance", one, one]] {
        let run = osd(&args);
        assert_eq!(run.code, 2, "{args:?}");
        assert_eq!(run.stdout, "");
    }
}

#[test]
fn the_interface_writes_the_adapter_word_with_its_exit_code_2_and_the_help_says_it() {
    let text = std::fs::read_to_string(repo().join("osd-cli/docs/INTERFACE.md")).unwrap();
    let row = text.lines().find(|l| l.starts_with("| `conformance`")).expect("a row for the word conformance in INTERFACE.md");
    assert!(row.contains("exit 2"), "{row}");
    let help = osd(&["conformance", "--help"]);
    assert_eq!(help.code, 0, "{}", help.stderr);
    assert!(help.stdout.contains("conformance"), "{}", help.stdout);
    assert!(help.stdout.contains("2"), "{}", help.stdout);
}
