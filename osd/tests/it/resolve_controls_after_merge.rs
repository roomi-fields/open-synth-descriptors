//! A sound transfer setting's `control` is judged once the document is resolved: it names an
//! entry of the merged tables (the target's, a linked target's, or the file's own).

use osd::FaultKind;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const S6: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn params(entries: Value) -> Value {
    json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": entries})
}

fn base(control: &str) -> Value {
    json!({
        "$schema": S6,
        "device": {"name": "Synth", "osc_prefix": "/synth"},
        "params": params(json!([{"osc": "/p0", "id": 3}, {"osc": "/p1", "id": 4}])),
        "sound_transfers": [{
            "id": "single", "sound_id": "program", "carries": "sound",
            "match_frame": [240, 66, "any", 0, 1, 81, 64, "any", "any", "any", "any", "any", "any", "any", "any", "any", 247],
            "elements": {"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}},
            "places": [{"control": control, "from": 0, "length": 1}]
        }]
    })
}

fn link(to: &str, extra: Value) -> Value {
    let mut d = json!({"$schema": S6, "device": {"name": "Synth", "osc_prefix": "/synth"}, "midi_of": to});
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    d
}

#[test]
fn a_control_naming_an_entry_of_the_target_is_accepted_through_midi_of() {
    let files = catalogue(vec![("acme/a", link("acme/b", json!({}))), ("acme/b", base("/params/entries/1"))]);
    osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn a_control_naming_an_entry_of_a_target_linked_in_turn_is_accepted() {
    let files = catalogue(vec![
        ("acme/a", link("acme/b", json!({}))),
        ("acme/b", link("acme/c", json!({}))),
        ("acme/c", base("/params/entries/1")),
    ]);
    osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn a_control_naming_an_entry_the_target_lacks_is_refused_with_its_pointer() {
    let files = catalogue(vec![("acme/a", link("acme/b", json!({}))), ("acme/b", base("/params/entries/2"))]);
    let f = osd::resolve("acme/a", &files).expect_err("an absent entry is refused");
    assert_eq!(f.kind, FaultKind::BadValue);
    assert_eq!(f.pointer, "/sound_transfers/0/places/0/control");
}

#[test]
fn a_midi_of_file_cannot_carry_sound_transfers_so_the_target_is_the_only_judge_of_its_controls() {
    let mut a = link("acme/b", json!({}));
    a["sound_transfers"] = base("/params/entries/9")["sound_transfers"].clone();
    let files = catalogue(vec![("acme/a", a), ("acme/b", base("/params/entries/1"))]);
    let f = osd::resolve("acme/a", &files).expect_err("a midi_of file adds routes only");
    assert_eq!(f.kind, FaultKind::UnknownField);
    assert_eq!(f.pointer, "/sound_transfers");
}

#[test]
fn a_control_naming_an_entry_absent_down_a_chain_is_refused() {
    let files = catalogue(vec![
        ("acme/a", link("acme/b", json!({}))),
        ("acme/b", link("acme/c", json!({}))),
        ("acme/c", base("/params/entries/5")),
    ]);
    let f = osd::resolve("acme/a", &files).expect_err("an absent entry is refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/sound_transfers/0/places/0/control"));
}
