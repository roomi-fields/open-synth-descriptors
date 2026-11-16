//! `osd::convert` carries the `_` notes of a 0.4 `midi_in` to the device
//! object, under `_midi_in` followed by the whole source key.

use osd::Kind;
use serde_json::{json, Value};

const CC: &str = "/cc/{num} {value} {channel}";

fn convert(doc: Value) -> Value {
    let out = osd::convert(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
    osd::read_as(&out, Kind::Device, osd::Version::V0_5).unwrap_or_else(|f| panic!("the result does not read: {f}"));
    serde_json::from_str(&out).unwrap()
}

fn device(midi_in: Value) -> Value {
    json!({"device": {"name": "T", "osc_prefix": "/t"}, "midi_in": midi_in})
}

#[test]
fn a_note_of_midi_in_is_carried_to_the_device_under_midi_in_and_its_key() {
    let out = convert(device(json!({"cc": CC, "_notes": "source X"})));
    assert_eq!(out["midi_in"], json!(["cc"]));
    assert_eq!(out["_midi_in_notes"], json!("source X"));
}

#[test]
fn the_destination_key_is_midi_in_followed_by_the_whole_source_key() {
    let out = convert(device(json!({"cc": CC, "_source": "a", "_": "b", "_midi_in": "c"})));
    assert_eq!(out["_midi_in_source"], json!("a"));
    assert_eq!(out["_midi_in_"], json!("b"));
    assert_eq!(out["_midi_in_midi_in"], json!("c"));
    assert_eq!(out["midi_in"], json!(["cc"]));
}

#[test]
fn a_note_keeps_its_json_value_whatever_its_type() {
    let out = convert(device(json!({"_a": {"url": "u", "n": [1, 2]}, "_b": 3, "_c": null, "_d": [true]})));
    assert_eq!(out["_midi_in_a"], json!({"url": "u", "n": [1, 2]}));
    assert_eq!(out["_midi_in_b"], json!(3));
    assert_eq!(out["_midi_in_c"], Value::Null);
    assert_eq!(out["_midi_in_d"], json!([true]));
    assert_eq!(out["midi_in"], json!([]));
}

#[test]
fn a_midi_in_without_notes_converts_without_any_midi_in_key() {
    let out = convert(device(json!({"cc": CC})));
    assert_eq!(out["midi_in"], json!(["cc"]));
    let keys: Vec<&String> = out.as_object().unwrap().keys().filter(|k| k.starts_with("_midi_in")).collect();
    assert!(keys.is_empty(), "{keys:?}");
}

#[test]
fn a_taken_destination_is_not_overwritten_and_the_note_takes_the_first_free_suffix() {
    let out = convert(json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "midi_in": {"_notes": "from midi_in"},
        "_midi_in_notes": "on the device",
    }));
    assert_eq!(out["_midi_in_notes"], json!("on the device"));
    assert_eq!(out["_midi_in_notes_2"], json!("from midi_in"));

    let out = convert(json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "midi_in": {"_notes": "n"},
        "_midi_in_notes": "one",
        "_midi_in_notes_2": "two",
    }));
    assert_eq!(out["_midi_in_notes"], json!("one"));
    assert_eq!(out["_midi_in_notes_2"], json!("two"));
    assert_eq!(out["_midi_in_notes_3"], json!("n"));
}

#[test]
fn a_converted_device_with_midi_in_notes_converts_to_itself() {
    let once = osd::convert(&device(json!({"cc": CC, "_notes": "x"})).to_string(), Kind::Device).unwrap();
    assert_eq!(osd::convert(&once, Kind::Device).as_deref(), Ok(once.as_str()));
}
