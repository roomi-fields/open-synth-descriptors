//! `osd convert`: the text `osd::convert` gives, on the standard output or in a file.

use crate::common::{self, cases, osd, osd_in, repo};
use serde_json::Value;

fn path_of(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
fn convert_prints_the_text_osd_convert_gives_for_each_conformance_case() {
    let mut failures = Vec::new();
    let mut seen = 0;
    for dir in cases(&repo().join("conformance/0.5/convert")) {
        seen += 1;
        let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        let kind = case["convert"].as_str().unwrap();
        let file = path_of(&dir.join("descriptor.json"));
        let text = std::fs::read_to_string(&file).unwrap();
        let run = osd(&["convert", "--kind", kind, &file]);
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        match osd::convert(&text, common::kind_of(kind)) {
            Ok(want) => {
                if run.code != 0 || run.stdout != want {
                    failures.push(format!("{name}: exit {}, stdout differs, stderr {:?}", run.code, run.stderr));
                }
            }
            Err(f) => {
                let line = format!("{file}: {:?} at \"{}\": {}", f.kind, f.pointer, f.rule);
                if run.code != 1 || !run.stderr.lines().any(|l| l == line) || !run.stdout.is_empty() {
                    failures.push(format!("{name}: exit {}, wants {line:?}, stderr {:?}", run.code, run.stderr));
                }
            }
        }
    }
    assert!(seen >= 20, "{seen} cases");
    common::report("cases failed", failures);
}

#[test]
fn convert_returns_a_0_5_descriptor_as_osd_convert_renders_it() {
    for kind in common::KINDS {
        let file = path_of(&common::example_path(kind));
        let text = std::fs::read_to_string(&file).unwrap();
        let run = osd(&["convert", &file]);
        assert_eq!(run.code, 0, "{kind}: {:?}", run.stderr);
        assert_eq!(run.stdout, osd::convert(&text, common::kind_of(kind)).unwrap(), "{kind}");
    }
}

#[test]
fn convert_of_a_0_4_descriptor_names_the_latest_version() {
    let file = path_of(&repo().join("examples/devices/test_synth.json"));
    let text = std::fs::read_to_string(&file).unwrap();
    let run = osd(&["convert", "--kind", "device", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, osd::convert(&text, osd::Kind::Device).unwrap());
    let doc: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(doc["$schema"], "https://w3id.org/osd/0.5/device.schema.json");
}

#[test]
fn convert_writes_the_file_named_by_o_and_prints_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&repo().join("examples/devices/test_synth.json"));
    let out = path_of(&dir.path().join("out.json"));
    let want = osd(&["convert", "--kind", "device", &file]).stdout;
    let run = osd(&["convert", "--kind", "device", &file, "-o", &out]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(!want.is_empty());
    assert_eq!(std::fs::read_to_string(&out).unwrap(), want);
}

#[test]
fn convert_leaves_an_existing_output_file_unless_forced() {
    let dir = tempfile::tempdir().unwrap();
    let file = path_of(&repo().join("examples/devices/test_synth.json"));
    let out = dir.path().join("out.json");
    std::fs::write(&out, "keep me").unwrap();
    let out = path_of(&out);

    let run = osd(&["convert", "--kind", "device", &file, "-o", &out]);
    assert_eq!(run.code, 2);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "keep me");

    let run = osd(&["convert", "--kind", "device", &file, "-o", &out, "--force"]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    let want = osd(&["convert", "--kind", "device", &file]).stdout;
    assert_eq!(std::fs::read_to_string(&out).unwrap(), want);
}

#[test]
fn convert_of_a_refused_text_writes_no_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "{ not json").unwrap();
    let out = dir.path().join("out.json");
    let run = osd(&["convert", "--kind", "device", &path_of(&bad), "-o", &path_of(&out)]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.contains("Syntax at \"\""), "{:?}", run.stderr);
    assert!(!out.exists());
}

#[test]
fn convert_refuses_a_file_without_schema_declaration_and_without_kind() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.json");
    std::fs::write(&plain, r#"{"device": {"name": "Synth", "osc_prefix": "/synth"}}"#).unwrap();
    let run = osd_in(dir.path(), &["convert", &path_of(&plain)]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("--kind"), "{:?}", run.stderr);
    assert_eq!(run.stdout, "");
}
