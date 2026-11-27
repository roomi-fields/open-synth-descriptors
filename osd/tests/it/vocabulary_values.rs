//! The third type of term of the vocabulary of roles, `value`: a choice of a selector. Its
//! `applies_to` names one or more settings of the same state; a setting still applies to parts
//! only. A value is neither a part nor a setting of a role.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const STATE_SCHEMA: &str = "https://w3id.org/osd/vocabulary/2/vocabulary.schema.json";

fn term(name: &str, kind: &str) -> Value {
    json!({
        "name": name,
        "type": kind,
        "definition": format!("The {name} of a sound."),
        "words": [],
        "shared_by": [{"maker": "Moog", "device": "Minimoog"}, {"maker": "Roland", "device": "Juno-106"}]
    })
}

fn applying(name: &str, kind: &str, applies_to: &[&str]) -> Value {
    let mut t = term(name, kind);
    t["applies_to"] = json!(applies_to);
    t
}

/// A part, its selector, a part, its selector (indices 0 to 3).
fn base() -> Vec<Value> {
    vec![
        term("filter", "part"),
        applying("filter-type", "setting", &["filter"]),
        term("oscillator", "part"),
        applying("waveform", "setting", &["oscillator"]),
    ]
}

fn state_of(terms: Vec<Value>) -> Value {
    json!({"$schema": STATE_SCHEMA, "state": "2", "terms": terms})
}

fn with(extra: Vec<Value>) -> Value {
    let mut t = base();
    t.extend(extra);
    state_of(t)
}

fn read(doc: &Value) -> Result<osd::Vocabulary, osd::Fault> {
    osd::read_vocabulary(&doc.to_string())
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).err().unwrap_or_else(|| panic!("accepted: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

#[test]
fn a_value_that_applies_to_one_setting_or_several_is_read_in_any_order() {
    let values = vec![applying("low-pass", "value", &["filter-type"]), applying("sine", "value", &["waveform"])];
    assert!(read(&with(values)).map_err(|f| f.to_string()).is_ok());
    let both = applying("off", "value", &["filter-type", "waveform"]);
    assert!(read(&with(vec![both])).map_err(|f| f.to_string()).is_ok());
    // A value declared before the setting it applies to.
    let mut t = base();
    t.insert(0, applying("sine", "value", &["waveform"]));
    assert!(read(&state_of(t)).map_err(|f| f.to_string()).is_ok());
    // A source stands for shared_by.
    let mut sourced = applying("square", "value", &["waveform"]);
    sourced.as_object_mut().unwrap().remove("shared_by");
    sourced["source"] = json!([{"document": "SFZ", "table": "sample", "name": "*square"}]);
    assert!(read(&with(vec![sourced])).map_err(|f| f.to_string()).is_ok());
}

#[test]
fn a_value_without_applies_to_is_missing_it() {
    refused(&with(vec![term("sine", "value")]), FaultKind::Missing, "/terms/4/applies_to");
}

#[test]
fn a_value_with_an_empty_applies_to_is_refused_at_it() {
    refused(&with(vec![applying("sine", "value", &[])]), FaultKind::BadValue, "/terms/4/applies_to");
}

#[test]
fn a_value_applies_to_settings_of_the_state_and_to_nothing_else() {
    // A part.
    refused(&with(vec![applying("sine", "value", &["oscillator"])]), FaultKind::BadValue, "/terms/4/applies_to/0");
    refused(&with(vec![applying("sine", "value", &["waveform", "filter"])]), FaultKind::BadValue, "/terms/4/applies_to/1");
    // A name the state lacks.
    refused(&with(vec![applying("sine", "value", &["waveform", "ghost"])]), FaultKind::BadValue, "/terms/4/applies_to/1");
    // Another value, and itself.
    let other = applying("sine", "value", &["waveform"]);
    refused(&with(vec![other, applying("square", "value", &["sine"])]), FaultKind::BadValue, "/terms/5/applies_to/0");
    refused(&with(vec![applying("sine", "value", &["sine"])]), FaultKind::BadValue, "/terms/4/applies_to/0");
}

#[test]
fn a_value_neither_stands_under_a_part_nor_opens_a_path() {
    let mut t = applying("sine", "value", &["waveform"]);
    t["stands_under"] = json!(["oscillator"]);
    refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/stands_under");
    for at_top in [true, false] {
        let mut t = applying("sine", "value", &["waveform"]);
        t["at_top"] = json!(at_top);
        refused(&with(vec![t]), FaultKind::BadValue, "/terms/4/at_top");
    }
}

#[test]
fn a_setting_still_applies_to_parts_only_and_not_to_a_value() {
    let value = applying("sine", "value", &["waveform"]);
    let setting = applying("depth", "setting", &["sine"]);
    refused(&with(vec![setting, value.clone()]), FaultKind::BadValue, "/terms/4/applies_to/0");
    let setting = applying("depth", "setting", &["filter", "sine"]);
    refused(&with(vec![value, setting]), FaultKind::BadValue, "/terms/5/applies_to/1");
}

#[test]
fn a_part_does_not_stand_under_a_value() {
    let value = applying("sine", "value", &["waveform"]);
    let mut part = term("envelope", "part");
    part["stands_under"] = json!(["sine"]);
    refused(&with(vec![value, part]), FaultKind::BadValue, "/terms/5/stands_under/0");
}

#[test]
fn a_value_needs_a_source_or_a_shared_by() {
    let mut t = applying("sine", "value", &["waveform"]);
    t.as_object_mut().unwrap().remove("shared_by");
    refused(&with(vec![t]), FaultKind::Missing, "/terms/4/source");
}

// The schema of state 2.

fn schema_paths(doc: &Value) -> Vec<String> {
    let path = common::repo().join("schemas/vocabulary/2/vocabulary.schema.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let schema: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(schema["$id"], STATE_SCHEMA);
    let validator = jsonschema::options().build(&schema).unwrap();
    validator.iter_errors(doc).map(|e| e.instance_path().to_string()).collect()
}

#[test]
fn the_schema_of_state_2_accepts_values_and_the_shapes_the_reader_accepts() {
    let values = vec![applying("low-pass", "value", &["filter-type"]), applying("off", "value", &["filter-type", "waveform"])];
    assert_eq!(schema_paths(&with(values)), Vec::<String>::new());
    // What a name refers to is the reader's to judge.
    let wrong = applying("sine", "value", &["oscillator", "ghost"]);
    assert_eq!(schema_paths(&with(vec![wrong])), Vec::<String>::new());
}

#[test]
fn the_schema_of_state_2_refuses_each_shape_of_a_value_the_reader_refuses() {
    let at = |t: Value| schema_paths(&with(vec![t]));
    assert!(at(term("sine", "value")).contains(&"/terms/4".to_string()), "a value without applies_to");
    assert!(at(applying("sine", "value", &[])).contains(&"/terms/4/applies_to".to_string()));
    let mut t = applying("sine", "value", &["waveform"]);
    t["stands_under"] = json!(["oscillator"]);
    assert!(!at(t).is_empty(), "a value with stands_under");
    for at_top in [true, false] {
        let mut t = applying("sine", "value", &["waveform"]);
        t["at_top"] = json!(at_top);
        assert!(!at(t).is_empty(), "a value with at_top {at_top}");
    }
}

#[test]
fn the_schema_of_state_2_keeps_the_rules_of_parts_and_settings() {
    assert!(!schema_paths(&with(vec![applying("mixer", "part", &["filter"])])).is_empty(), "a part with applies_to");
    assert!(!schema_paths(&with(vec![term("depth", "setting")])).is_empty(), "a setting without applies_to");
    assert!(!schema_paths(&with(vec![term("depth", "knob")])).is_empty(), "an unknown type");
    let mut t = term("sine", "value");
    t["applies_to"] = json!(["waveform"]);
    t.as_object_mut().unwrap().remove("shared_by");
    assert!(!schema_paths(&with(vec![t])).is_empty(), "neither a source nor a shared_by");
}

// A value in a role.

/// A device of 0.6 whose one control has this role, written with state 2.
fn device_with(role: Value) -> String {
    json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "role_vocabulary": "2",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "role": role}]}
    })
    .to_string()
}

/// Whether a message calls the term a part or a setting.
fn calls_it_a_part_or_a_setting(rule: &str) -> bool {
    let low = rule.to_lowercase();
    ["is a part", "is a setting", "is the part", "is the setting", "the part low", "the setting low", "the part sine", "the setting sine"]
        .iter()
        .any(|w| low.contains(w))
}

#[test]
fn a_value_used_as_a_part_of_a_role_is_refused_at_the_part_without_being_called_a_setting() {
    for (parts, pointer) in [
        (json!([{"part": "sine"}]), "/cc_params/entries/0/role/parts/0/part"),
        (json!([{"part": "low-pass"}]), "/cc_params/entries/0/role/parts/0/part"),
        (json!([{"part": "filter"}, {"part": "low-pass"}]), "/cc_params/entries/0/role/parts/1/part"),
    ] {
        let doc = device_with(json!({"parts": parts, "setting": "cutoff"}));
        let f = osd::read(&doc, Kind::Device).err().unwrap_or_else(|| panic!("accepted: {doc}"));
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
        assert!(!calls_it_a_part_or_a_setting(&f.rule), "{}", f.rule);
    }
}

#[test]
fn a_value_used_as_the_setting_of_a_role_is_refused_at_the_setting_without_being_called_a_part() {
    for (part, value) in [("filter", "low-pass"), ("oscillator", "sine")] {
        let doc = device_with(json!({"parts": [{"part": part}], "setting": value}));
        let f = osd::read(&doc, Kind::Device).err().unwrap_or_else(|| panic!("accepted: {doc}"));
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/cc_params/entries/0/role/setting"), "{f}");
        assert!(!calls_it_a_part_or_a_setting(&f.rule), "{}", f.rule);
    }
}

#[test]
fn a_value_segment_of_a_param_path_is_refused_at_its_position_without_being_called_a_part_or_a_setting() {
    for (path, pointer) in [
        ("/Filter/LowPass", "/1"),
        ("/Oscillator/Sine", "/1"),
        ("/Oscillator/2/Square", "/2"),
        ("/Sine/Cutoff", "/0"),
        ("/Filter/BandReject/Cutoff", "/1"),
    ] {
        let f = osd::role_from_param_path(path, "2").err().unwrap_or_else(|| panic!("{path} accepted"));
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{path}: {f}");
        assert!(!calls_it_a_part_or_a_setting(&f.rule), "{path}: {}", f.rule);
    }
}
