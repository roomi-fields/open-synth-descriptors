//! `osd` and a source not carried as credit (an entry of a note `_sources_<n>` from 0.7): each word
//! that judges or renders a descriptor prints one line per such entry on the error output, the
//! standard output and the exit code unchanged, and `osd conformance` writes it as
//! `UncreditedSource`.

use crate::common::{osd, repo, Run};
use serde_json::{json, Value};

fn address(version: &str, kind: &str) -> String {
    format!("https://w3id.org/osd/{version}/{kind}.schema.json")
}

/// A descriptor of `kind` in `version` holding `body` and the keys of `extra`.
fn descriptor(version: &str, kind: &str, extra: Value) -> Value {
    let target = json!({"descriptor": "korg/minilogue", "vendor": "KORG", "model": "minilogue"});
    let mut doc = match kind {
        "profile" => json!({"identity": {"reply_text": "acme"}}),
        "sound" => json!({"name": "Warm pad", "kind": "sound", "device": target, "content": {"settings": {"/amp/attack": [90]}}}),
        "fingerprint" => json!({"observed": "2026-11-22", "device": {"descriptor": "korg/minilogue", "firmware": "2.01"}}),
        "firmware" => json!({"name": "Mod", "version": "2.0", "kind": "mod", "targets": [{"device": target}]}),
        "assembly" => json!({"stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": []}),
        other => panic!("{other} is not a kind of this file"),
    };
    doc["$schema"] = json!(address(version, kind));
    for (k, v) in extra.as_object().expect("an object") {
        doc[k] = v.clone();
    }
    doc
}

fn with_file(doc: &Value) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("d.json").to_string_lossy().into_owned();
    std::fs::write(&file, doc.to_string()).unwrap();
    (dir, file)
}

/// The line of a source not carried as credit, `asked` being the file the word was given.
fn lines(asked: &str, pointers: &[&str]) -> Vec<String> {
    pointers.iter().map(|p| format!("{asked}: warning at \"{p}\": the source is kept as a note and not carried as credit")).collect()
}

fn error_lines(run: &Run) -> Vec<&str> {
    run.stderr.lines().collect()
}

#[test]
fn validate_prints_one_line_per_source_a_sound_keeps_in_a_note_says_ok_and_exits_0() {
    let doc = descriptor("0.7", "sound", json!({"_sources_2": ["see the manual", {"type": "guess"}]}));
    let (_dir, file) = with_file(&doc);
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, format!("{file}: ok\n"));
    assert_eq!(error_lines(&run), lines(&file, &["/_sources_2/0", "/_sources_2/1"]));
}

#[test]
fn validate_prints_the_line_for_each_kind_that_keeps_a_source_in_a_note_a_note_that_is_no_list_at_the_note() {
    for kind in ["profile", "fingerprint", "firmware", "assembly"] {
        let doc = descriptor("0.7", kind, json!({"_sources_2": ["a"], "_sources_3": "b"}));
        let (_dir, file) = with_file(&doc);
        let run = osd(&["validate", &file]);
        assert_eq!(run.code, 0, "{kind}: {:?}", run.stderr);
        assert_eq!(run.stdout, format!("{file}: ok\n"), "{kind}");
        assert_eq!(error_lines(&run), lines(&file, &["/_sources_2/0", "/_sources_3"]), "{kind}");
    }
}

#[test]
fn validate_prints_the_line_of_the_assembly_a_sound_was_made_on_after_the_sounds_own() {
    let made_on = json!({"stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": [], "_sources_2": ["inner"]});
    let doc = descriptor("0.7", "sound", json!({"_sources_2": ["outer"], "made_on": made_on}));
    let (_dir, file) = with_file(&doc);
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(error_lines(&run), lines(&file, &["/_sources_2/0", "/made_on/_sources_2/0"]));
}

#[test]
fn convert_prints_the_line_for_each_source_the_text_it_renders_keeps_in_a_note() {
    let doc = descriptor("0.6", "sound", json!({"_sources": [{"type": "vendor-doc"}, "see the manual", {"type": "guess"}]}));
    let (dir, file) = with_file(&doc);
    let want = osd::convert_to(&doc.to_string(), osd::Kind::Sound, osd::Version::V0_7).unwrap();
    let run = osd(&["convert", "--to", "0.7", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, want);
    assert_eq!(error_lines(&run), lines(&file, &["/_sources_2/0", "/_sources_2/1"]));
    let out = dir.path().join("out.json").to_string_lossy().into_owned();
    let run = osd(&["convert", "--to", "0.7", &file, "-o", &out]);
    assert_eq!((run.code, run.stdout.as_str()), (0, ""), "{:?}", run.stderr);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), want);
    assert_eq!(error_lines(&run), lines(&file, &["/_sources_2/0", "/_sources_2/1"]));
}

#[test]
fn a_note_before_0_7_and_a_source_carried_as_credit_get_no_line() {
    let old = descriptor("0.6", "sound", json!({"_sources_2": ["a"]}));
    let credited = descriptor("0.7", "sound", json!({"sources": [{"type": "vendor-doc"}]}));
    for doc in [old, credited] {
        let (_dir, file) = with_file(&doc);
        let run = osd(&["validate", &file]);
        assert_eq!((run.code, run.stderr.as_str()), (0, ""), "{doc}");
    }
}

#[test]
fn the_adapter_writes_a_source_not_carried_as_credit_as_uncredited_source_at_its_pointer() {
    let dir = repo().join("conformance/0.7/read/sound-sources-not-carried-warned");
    let run = osd(&["conformance", dir.to_str().unwrap()]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let got: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("{e}: {}", run.stdout));
    let want = json!([
        {"kind": "UncreditedSource", "pointer": "/_sources_2/0"},
        {"kind": "UncreditedSource", "pointer": "/_sources_2/1"},
    ]);
    assert_eq!(got["warnings"], want);
}
