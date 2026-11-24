//! `tools/validate.py` on the resolution cases of the conformance suite: a case says in its
//! `case.json` which of its files the schema must refuse (`schema_refused`, `{file, pointer}`
//! each), and the script checks both ways. Medium: it runs the script, as the Check workflow does.

use crate::common;

use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const VALID: &str = r#"{ "device": { "name": "Synth", "osc_prefix": "/synth" } }"#;
const NULL_REQUEST: &str = r#"{ "device": { "name": "Synth", "osc_prefix": "/synth" }, "identity": { "profile": "acme", "request": null } }"#;
const NULL_REQUEST_AND_DUMP: &str = r#"{ "device": { "name": "Synth", "osc_prefix": "/synth" }, "identity": { "profile": "acme", "request": null }, "programs": { "dump": null } }"#;
const PROFILE: &str = r#"{ "_about": "x" }"#;

struct Run {
    code: Option<i32>,
    out: String,
}

/// A case folder holding `files` (catalogue ids) and a `case.json` with `extra` keys.
fn case(files: &[(&str, &str)], extra: Value) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let mut case = json!({ "_about": "test", "resolve": "acme/synth" });
    case.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    std::fs::write(dir.path().join("case.json"), case.to_string()).unwrap();
    for (id, text) in files {
        let path = dir.path().join("files").join(format!("{id}.json"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn validate(kind: &str, folders: &[&Path]) -> Run {
    let script = common::repo().join("tools/validate.py");
    let mut last = None;
    for python in ["python3", "python"] {
        let mut cmd = Command::new(python);
        cmd.arg(&script).arg(kind).env("OSD_VERSION", "0.4");
        for f in folders {
            cmd.arg(f.join("files"));
        }
        match cmd.output() {
            Ok(o) => {
                let out = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                return Run { code: o.status.code(), out };
            }
            Err(e) => last = Some(e),
        }
    }
    panic!("no python to run validate.py: {last:?}");
}

fn refused(file: &str, pointer: &str) -> Value {
    json!({ "schema_refused": [{ "file": file, "pointer": pointer }] })
}

#[test]
fn a_file_the_case_says_the_schema_refuses_is_expected_refused_and_the_pass_succeeds() {
    let dir = case(&[("acme/synth", NULL_REQUEST)], refused("acme/synth", "/identity/request"));
    let run = validate("device", &[dir.path()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn the_count_tells_valid_files_from_files_refused_as_expected() {
    let dir = case(&[("acme/synth", NULL_REQUEST), ("acme/other", VALID)], refused("acme/synth", "/identity/request"));
    let run = validate("device", &[dir.path()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
    let last = run.out.lines().last().unwrap();
    assert!(last.contains("refused"), "the count names the refused files: {last}");
    assert!(last.contains("valid"), "the count names the valid files: {last}");
}

#[test]
fn a_case_without_schema_refused_still_expects_every_file_valid() {
    let ok = case(&[("acme/synth", VALID)], json!({}));
    assert_eq!(validate("device", &[ok.path()]).code, Some(0));
    let bad = case(&[("acme/synth", NULL_REQUEST)], json!({}));
    assert_eq!(validate("device", &[bad.path()]).code, Some(1));
}

#[test]
fn a_file_expected_refused_that_is_valid_fails_naming_the_file_and_the_pointer() {
    let dir = case(&[("acme/synth", VALID)], refused("acme/synth", "/identity/request"));
    let run = validate("device", &[dir.path()]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("synth.json"), "{}", run.out);
    assert!(run.out.contains("/identity/request"), "{}", run.out);
}

#[test]
fn a_file_refused_at_another_pointer_than_the_one_said_fails_naming_the_file_and_the_pointer() {
    let dir = case(&[("acme/synth", NULL_REQUEST)], refused("acme/synth", "/programs/dump"));
    let run = validate("device", &[dir.path()]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("synth.json"), "{}", run.out);
    assert!(run.out.contains("/programs/dump"), "{}", run.out);
}

#[test]
fn a_file_with_more_than_one_error_fails_even_when_one_is_at_the_pointer_said() {
    let dir = case(&[("acme/synth", NULL_REQUEST_AND_DUMP)], refused("acme/synth", "/identity/request"));
    let run = validate("device", &[dir.path()]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("synth.json"), "{}", run.out);
}

#[test]
fn a_file_refused_that_the_case_does_not_list_fails() {
    let dir = case(&[("acme/synth", VALID), ("acme/other", NULL_REQUEST)], refused("acme/synth", "/identity/request"));
    let run = validate("device", &[dir.path()]);
    assert_eq!(run.code, Some(1), "{}", run.out);
    assert!(run.out.contains("other.json"), "{}", run.out);
}

#[test]
fn a_listed_file_the_pass_does_not_select_is_left_alone() {
    // acme/synth is a device: the profile pass reads only the `_profile*` files.
    let dir = case(&[("acme/synth", NULL_REQUEST), ("acme/_profile", PROFILE)], refused("acme/synth", "/identity/request"));
    let run = validate("profile", &[dir.path()]);
    assert_eq!(run.code, Some(0), "{}", run.out);
}

#[test]
fn the_cases_of_the_repository_pass_both_the_device_and_the_profile_pass() {
    let cases: Vec<_> = std::fs::read_dir(common::repo().join("conformance/0.4/resolve")).unwrap().map(|e| e.unwrap().path()).collect();
    let folders: Vec<&Path> = cases.iter().map(|p| p.as_path()).collect();
    for kind in ["device", "profile"] {
        let run = validate(kind, &folders);
        assert_eq!(run.code, Some(0), "{kind}: {}", run.out);
    }
}

#[test]
fn the_two_null_block_cases_of_0_4_say_where_the_schema_refuses_their_file() {
    for (name, pointer) in [("identity-request-null-refused", "/identity/request"), ("programs-dump-null-refused", "/programs/dump")] {
        let path = common::repo().join("conformance/0.4/resolve").join(name).join("case.json");
        let case: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(case["schema_refused"], json!([{ "file": "acme/synth", "pointer": pointer }]), "{name}");
        assert!(case.get("fault").is_some(), "{name} keeps its fault");
    }
}
