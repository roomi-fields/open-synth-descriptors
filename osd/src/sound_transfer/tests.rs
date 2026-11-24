//! The judgement of the `control` of each setting: by the pattern alone for a text read on its
//! own that holds `midi_of`; by the existence of the entry in the tables for a merged document,
//! whether or not it keeps `midi_of`.

use super::check_controls;
use crate::FaultKind;
use serde_json::{json, Value};

fn document(midi_of: bool, controls: &[&str]) -> Value {
    let settings: Vec<Value> = controls.iter().map(|c| json!({"control": c, "from": 0, "length": 1})).collect();
    let mut d = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1}]},
        "params": {"entries": [{"osc": "/p0", "id": 3}, {"osc": "/p1", "id": 4}]},
        "sound_transfers": [{"id": "single", "places": settings}]
    });
    if midi_of {
        d["midi_of"] = json!("acme/b");
    }
    d
}

#[test]
fn a_merged_document_that_keeps_midi_of_refuses_a_control_naming_an_absent_entry() {
    let f = check_controls(&document(true, &["/params/entries/2"]), true).unwrap_err();
    assert_eq!(f.kind, FaultKind::BadValue);
    assert_eq!(f.pointer, "/sound_transfers/0/places/0/control");
    assert!(f.rule.contains("/params/entries/2"), "{}", f.rule);
}

#[test]
fn a_merged_document_that_keeps_midi_of_refuses_an_absent_cc_params_entry_after_a_present_one() {
    let f = check_controls(&document(true, &["/params/entries/0", "/cc_params/entries/1"]), true).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/sound_transfers/0/places/1/control"));
    assert!(f.rule.contains("/cc_params/entries/1"), "{}", f.rule);
}

#[test]
fn a_merged_document_that_keeps_midi_of_accepts_controls_naming_present_entries() {
    assert!(check_controls(&document(true, &["/cc_params/entries/0", "/params/entries/1"]), true).is_ok());
}

#[test]
fn a_merged_document_without_midi_of_still_refuses_an_absent_entry() {
    assert!(check_controls(&document(false, &["/params/entries/2"]), true).is_err());
}

#[test]
fn a_text_read_alone_that_holds_midi_of_is_judged_on_the_pattern_only() {
    assert!(check_controls(&document(true, &["/params/entries/9"]), false).is_ok());
    let f = check_controls(&document(true, &["/params/entries/x"]), false).unwrap_err();
    assert_eq!(f.kind, FaultKind::BadValue);
}

#[test]
fn a_text_read_alone_without_midi_of_refuses_an_absent_entry() {
    assert!(check_controls(&document(false, &["/params/entries/2"]), false).is_err());
}
