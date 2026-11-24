//! `osd validate`: one verdict per file, what `osd::read` says, the exit codes.

use crate::common::{self, cases, osd, repo};
use serde_json::Value;

/// A conformance reading case: its descriptor path, its kind, and what it expects.
fn read_case(dir: &std::path::Path) -> (String, String, Value) {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let path = dir.join("descriptor.json").to_string_lossy().into_owned();
    (path, case["read"].as_str().unwrap().to_string(), case)
}

#[test]
fn validate_says_of_each_conformance_reading_case_what_the_case_expects() {
    let mut failures = Vec::new();
    let mut seen = 0;
    for version in ["0.4", "0.5"] {
        for dir in cases(&repo().join("conformance").join(version).join("read")) {
            seen += 1;
            let (path, kind, case) = read_case(&dir);
            let run = osd(&["validate", "--kind", &kind, &path]);
            let name = format!("{version}/{}", dir.file_name().unwrap().to_string_lossy());
            if let Some(f) = case.get("fault") {
                let want = format!("{} at \"{}\"", f["kind"].as_str().unwrap(), f["pointer"].as_str().unwrap());
                let line_ok = run.stderr.lines().any(|l| l.starts_with(&format!("{path}: ")) && l.contains(&want));
                if run.code != 1 || !line_ok {
                    failures.push(format!("{name}: exit {}, wants a line with {want}, stderr {:?}", run.code, run.stderr));
                }
            } else if run.code != 0 || !run.stdout.lines().any(|l| l == format!("{path}: ok")) {
                failures.push(format!("{name}: exit {}, stdout {:?}, stderr {:?}", run.code, run.stdout, run.stderr));
            }
        }
    }
    assert!(seen >= 40, "{seen} cases");
    common::report("cases failed", failures);
}

#[test]
fn validate_prints_the_kind_pointer_and_rule_osd_read_gives() {
    let mut seen = 0;
    for dir in cases(&repo().join("conformance/0.5/read")) {
        let (path, kind, case) = read_case(&dir);
        if case.get("fault").is_none() {
            continue;
        }
        seen += 1;
        let text = std::fs::read_to_string(&path).unwrap();
        let fault = osd::read(&text, common::kind_of(&kind)).unwrap_err();
        let want = format!("{path}: {:?} at \"{}\": {}", fault.kind, fault.pointer, fault.rule);
        let run = osd(&["validate", "--kind", &kind, &path]);
        assert!(run.stderr.lines().any(|l| l == want), "{path}: wants {want:?}, stderr {:?}", run.stderr);
        assert_eq!(run.stdout, "", "{path}: a fault has no verdict on the standard output");
    }
    assert!(seen >= 10, "{seen} fault cases");
}

#[test]
fn validate_writes_an_empty_pointer_between_quotes_for_the_whole_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broken.json");
    std::fs::write(&path, "{ not json").unwrap();
    let path = path.to_string_lossy().into_owned();
    let run = osd(&["validate", "--kind", "device", &path]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.lines().any(|l| l.starts_with(&format!("{path}: Syntax at \"\": "))), "{:?}", run.stderr);
}

#[test]
fn validate_reads_the_kind_of_a_file_from_its_schema_declaration() {
    for kind in common::KINDS {
        let path = common::example_path(kind).to_string_lossy().into_owned();
        let run = osd(&["validate", &path]);
        assert_eq!((run.code, run.stdout.as_str()), (0, format!("{path}: ok\n").as_str()), "{kind}: {:?}", run.stderr);
    }
}

#[test]
fn validate_refuses_a_file_without_schema_declaration_and_without_kind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plain.json");
    std::fs::write(&path, r#"{"device": {"name": "Synth", "osc_prefix": "/synth"}}"#).unwrap();
    let path = path.to_string_lossy().into_owned();

    let run = osd(&["validate", &path]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("--kind"), "{:?}", run.stderr);
    assert!(!run.stdout.contains(&format!("{path}: ok")));

    let run = osd(&["validate", "--kind", "device", &path]);
    assert_eq!((run.code, run.stdout.as_str()), (0, format!("{path}: ok\n").as_str()), "{:?}", run.stderr);
}

#[test]
fn validate_refuses_an_unknown_kind_naming_the_option() {
    let path = common::example_path("device").to_string_lossy().into_owned();
    let run = osd(&["validate", "--kind", "gadget", &path]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("--kind"), "{:?}", run.stderr);
}

#[test]
fn validate_gives_one_verdict_per_file_and_exits_1_when_one_has_a_fault() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "[]").unwrap();
    let bad = bad.to_string_lossy().into_owned();
    let good = common::example_path("device").to_string_lossy().into_owned();
    let also = common::example_path("sound").to_string_lossy().into_owned();

    let run = osd(&["validate", &good, &also]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, format!("{good}: ok\n{also}: ok\n"));

    let run = osd(&["validate", "--kind", "device", &good, &bad]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stdout, format!("{good}: ok\n"));
    assert!(run.stderr.lines().any(|l| l.starts_with(&format!("{bad}: "))), "{:?}", run.stderr);
}

#[test]
fn validate_exits_2_without_a_verdict_for_a_path_that_is_not_a_readable_file() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.json").to_string_lossy().into_owned();
    let folder = dir.path().to_string_lossy().into_owned();
    for path in [&missing, &folder] {
        let run = osd(&["validate", "--kind", "device", path]);
        assert_eq!(run.code, 2, "{path}");
        assert!(!run.stdout.contains(&format!("{path}: ok")), "{path}");
        assert!(!run.stderr.contains(": ok"), "{path}");
    }
}

#[test]
fn validate_without_a_file_is_a_misuse() {
    let run = osd(&["validate"]);
    assert_eq!(run.code, 2);
}
