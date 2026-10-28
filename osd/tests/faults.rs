//! Every refusal of a read is the fault `osd/docs/INTERFACE.md` names, at the
//! JSON pointer of the value refused, and in the file it was read from.

use osd::{FaultKind, Kind};
use std::collections::BTreeMap;

const DEVICE: &str = r#"{"device": {"name": "Test", "osc_prefix": "/test"}}"#;

/// `DEVICE` with one change: `path` (keys from the root) set to `value`.
fn device_with(path: &[&str], value: serde_json::Value) -> String {
    let mut doc: serde_json::Value = serde_json::from_str(DEVICE).unwrap();
    let mut at = &mut doc;
    for k in &path[..path.len() - 1] {
        at = at.as_object_mut().unwrap().entry(k.to_string()).or_insert(serde_json::json!({}));
    }
    at[path[path.len() - 1]] = value;
    doc.to_string()
}

fn fault(text: &str, kind: Kind) -> osd::Fault {
    osd::read(text, kind).expect_err("refused")
}

#[track_caller]
fn refused(text: &str, kind: Kind, fault_kind: FaultKind, pointer: &str) {
    let f = fault(text, kind);
    assert_eq!((f.kind, f.pointer.as_str()), (fault_kind, pointer), "{f}");
}

#[test]
fn a_minimal_device_reads() {
    assert!(matches!(osd::read(DEVICE, Kind::Device), Ok(osd::Descriptor::Device(_))));
}

#[test]
fn text_that_is_not_json_is_a_syntax_fault() {
    refused("{\"device\": ", Kind::Device, FaultKind::Syntax, "");
}

#[test]
fn a_field_the_version_does_not_define_is_refused_where_it_is() {
    refused(&device_with(&["colour"], "red".into()), Kind::Device, FaultKind::UnknownField, "/colour");
    refused(&device_with(&["device", "colour"], "red".into()), Kind::Device, FaultKind::UnknownField, "/device/colour");
    let entries = serde_json::json!({"entries": [{"osc": "/a", "cc": 1}, {"osc": "/b", "ccc": 2}]});
    refused(&device_with(&["cc_params"], entries), Kind::Device, FaultKind::UnknownField, "/cc_params/entries/1/ccc");
    refused(r#"{"identity": {"reply": []}, "midi_out": {}}"#, Kind::Profile, FaultKind::UnknownField, "/midi_out");
}

#[test]
fn a_note_for_people_is_any_key_starting_with_an_underscore() {
    let text = device_with(&["cc_params"], serde_json::json!({"entries": [{"osc": "/a", "_why": {"any": [1]}}]}));
    let osd::Descriptor::Device(d) = osd::read(&text, Kind::Device).unwrap() else { panic!() };
    assert_eq!(d.cc_params.unwrap().entries[0].remarks["_why"], serde_json::json!({"any": [1]}));
}

#[test]
fn a_value_outside_its_type_is_refused() {
    refused(&device_with(&["device", "name"], 3.into()), Kind::Device, FaultKind::BadValue, "/device/name");
    refused(&device_with(&["midi_out", "default_channel"], 16.into()), Kind::Device, FaultKind::BadValue, "/midi_out/default_channel");
    refused(&device_with(&["midi_out", "note_offset"], 1.5.into()), Kind::Device, FaultKind::BadValue, "/midi_out/note_offset");
    refused(&device_with(&["device", "osc_prefix"], "test".into()), Kind::Device, FaultKind::BadValue, "/device/osc_prefix");
    let frame = serde_json::json!({"get_frame": [240, "value", 247], "set_frame": [], "value_type": "u7", "entries": []});
    refused(&device_with(&["params"], frame), Kind::Device, FaultKind::BadValue, "/params/get_frame/1");
    let range = serde_json::json!({"entries": [{"osc": "/a", "range": [0, 1, 2]}]});
    refused(&device_with(&["cc_params"], range), Kind::Device, FaultKind::BadValue, "/cc_params/entries/0/range");
    refused(&device_with(&["_sources"], serde_json::json!([{"url": "x"}])), Kind::Device, FaultKind::Missing, "/_sources/0/type");
    refused(&device_with(&["_coverage"], "all".into()), Kind::Device, FaultKind::BadValue, "/_coverage");
}

#[test]
fn a_value_outside_its_closed_list_is_refused() {
    refused(&device_with(&["device", "kind"], "plugin".into()), Kind::Device, FaultKind::BadValue, "/device/kind");
    let dump = serde_json::json!({"dump": {"kind": "sometimes"}});
    refused(&device_with(&["programs"], dump), Kind::Device, FaultKind::BadValue, "/programs/dump/kind");
}

#[test]
fn a_missing_required_field_is_refused() {
    refused(r#"{"midi_out": {}}"#, Kind::Device, FaultKind::Missing, "/device");
    refused(r#"{"device": {"name": "Test"}}"#, Kind::Device, FaultKind::Missing, "/device/osc_prefix");
    let trigger = serde_json::json!({"entries": [{"osc": "/kick", "sound": "kick"}]});
    refused(&device_with(&["triggers"], trigger), Kind::Device, FaultKind::Missing, "/triggers/entries/0/note_range");
}

#[test]
fn a_json_pointer_escapes_its_keys() {
    refused(&device_with(&["device", "a/b~c"], 1.into()), Kind::Device, FaultKind::UnknownField, "/device/a~1b~0c");
}

#[test]
fn a_version_it_does_not_read_is_refused() {
    let sound = |schema: &str| format!(r#"{{"$schema": "{schema}", "name": "Pad", "kind": "clip", "content": {{}}}}"#);
    assert!(osd::read(&sound("https://synthhub.roomi-fields.com/osd/0.4/sound.schema.json"), Kind::Sound).is_ok());
    refused(&sound("https://synthhub.roomi-fields.com/osd/0.3/sound.schema.json"), Kind::Sound, FaultKind::UnreadVersion, "/$schema");
    refused(&sound("https://example.com/other.json"), Kind::Sound, FaultKind::BadValue, "/$schema");
    assert_eq!(osd::VERSIONS, &[osd::Version::V0_4]);
}

#[test]
fn the_other_kinds_are_checked_as_strictly() {
    refused(r#"{"name": "Pad", "kind": "sound", "content": {}}"#, Kind::Sound, FaultKind::Missing, "/device");
    let long = "x".repeat(81);
    refused(&format!(r#"{{"name": "{long}", "kind": "clip", "content": {{}}}}"#), Kind::Sound, FaultKind::BadValue, "/name");
    refused(r#"{"observed": "2026-10-27", "identity_reply": "F0 7e F7"}"#, Kind::Fingerprint, FaultKind::BadValue, "/identity_reply");
    refused(r#"{"observed": "2026-10-27", "device": {"firmware": "2"}}"#, Kind::Fingerprint, FaultKind::Missing, "/device/descriptor");
    let firmware = r#"{"name": "JJOS", "version": "1", "device": {"vendor": "Akai", "model": "MPC"}, "author": []}"#;
    refused(firmware, Kind::Firmware, FaultKind::BadValue, "/author");
}

#[test]
fn a_fault_met_while_resolving_names_its_file() {
    let mut files = BTreeMap::new();
    files.insert("acme/one".to_string(), r#"{"device": {"name": "One", "osc_prefix": "/one"}, "midi_of": "acme/two"}"#.to_string());
    files.insert("acme/two".to_string(), r#"{"device": {"name": "One", "osc_prefix": "/one", "colour": 1}}"#.to_string());
    let f = osd::resolve("acme/one", &files).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::UnknownField, Some("acme/two"), "/device/colour"), "{f}");
    assert!(f.to_string().starts_with("acme/two: /device/colour: "), "{f}");
}
