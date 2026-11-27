//! `osd` and a control placed twice in one sound transfer, at places that differ: each word that
//! judges, renders or writes a descriptor prints on the error output one line per such place, to be
//! confirmed, the standard output and the exit code unchanged.

use crate::common::osd;
use serde_json::{json, Value};

/// A device of 0.7 whose one transfer places its control at `places`.
fn device(places: Value) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "contributed", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]}]},
        "sound_transfers": [{"id": "single", "carries": "sound", "match_frame": [240, 66, "any", "any", 247],
            "elements": {"data": {"from": 2, "length": 2}}, "places": places}],
    })
}

fn place(from: u8) -> Value {
    json!({"control": "/cc_params/entries/0", "from": from, "length": 1})
}

fn with_file(doc: &Value) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("d.json").to_string_lossy().into_owned();
    std::fs::write(&file, doc.to_string()).unwrap();
    (dir, file)
}

#[test]
fn validate_prints_one_line_per_place_to_be_confirmed_says_ok_and_exits_0() {
    let (_dir, file) = with_file(&device(json!([place(0), place(1)])));
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stdout, format!("{file}: ok\n"));
    let lines: Vec<&str> = run.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{:?}", run.stderr);
    for (m, line) in lines.iter().enumerate() {
        let at = format!("{file}: warning at \"/sound_transfers/0/places/{m}/control\": ");
        assert!(line.starts_with(&at) && line.ends_with("to be confirmed"), "{line}");
    }
}

#[test]
fn validate_says_nothing_of_identical_places() {
    let (_dir, file) = with_file(&device(json!([place(0), place(0)])));
    let run = osd(&["validate", &file]);
    assert_eq!(run.code, 0, "{:?}", run.stderr);
    assert_eq!(run.stderr, "");
}
