//! The word `build` of the program, and its refusals.

use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_osd-reference");
pub const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../schemas/0.6");
const KINDS: [&str; 6] = ["device", "profile", "sound", "fingerprint", "firmware", "assembly"];

pub fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("osd-reference runs")
}

#[test]
fn build_writes_one_markdown_page_per_kind() {
    let out = tempfile::tempdir().unwrap();
    let res = run(&["build", SCHEMAS, out.path().to_str().unwrap()]);
    assert_eq!(res.status.code(), Some(0), "{}", String::from_utf8_lossy(&res.stderr));
    let mut names: Vec<String> = std::fs::read_dir(out.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    let mut want: Vec<String> = KINDS.iter().map(|k| format!("{k}.md")).collect();
    want.sort();
    assert_eq!(names, want);
    let device = std::fs::read_to_string(out.path().join("device.md")).unwrap();
    assert!(device.contains("undescribed"));
}

#[test]
fn build_gives_the_same_pages_twice() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    run(&["build", SCHEMAS, a.path().to_str().unwrap()]);
    run(&["build", SCHEMAS, b.path().to_str().unwrap()]);
    for k in KINDS {
        let f = format!("{k}.md");
        assert_eq!(std::fs::read(a.path().join(&f)).unwrap(), std::fs::read(b.path().join(&f)).unwrap());
    }
}

#[test]
fn build_refuses_a_folder_without_schema() {
    let empty = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let res = run(&["build", empty.path().to_str().unwrap(), out.path().to_str().unwrap()]);
    assert_eq!(res.status.code(), Some(1));
}

#[test]
fn build_refuses_a_file_that_is_not_json_naming_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("broken.schema.json"), "{ not json").unwrap();
    let out = tempfile::tempdir().unwrap();
    let res = run(&["build", dir.path().to_str().unwrap(), out.path().to_str().unwrap()]);
    assert_eq!(res.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&res.stderr).contains("broken.schema.json"));
}

#[test]
fn build_refuses_a_reference_no_file_answers_naming_file_and_pointer() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("lonely.schema.json"),
        r##"{"properties":{"a":{"$ref":"#/$defs/missing"}}}"##,
    )
    .unwrap();
    let out = tempfile::tempdir().unwrap();
    let res = run(&["build", dir.path().to_str().unwrap(), out.path().to_str().unwrap()]);
    assert_eq!(res.status.code(), Some(1));
    let err = String::from_utf8_lossy(&res.stderr);
    assert!(err.contains("lonely.schema.json") && err.contains("#/$defs/missing"), "{err}");
}

#[test]
fn a_missing_argument_exits_2() {
    assert_eq!(run(&[]).status.code(), Some(2));
    assert_eq!(run(&["build", SCHEMAS]).status.code(), Some(2));
    assert_eq!(run(&["count"]).status.code(), Some(2));
    assert_eq!(run(&["undescribed"]).status.code(), Some(2));
}
