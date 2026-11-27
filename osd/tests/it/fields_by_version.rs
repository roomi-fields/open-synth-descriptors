//! A descriptor holds the fields of the version it names (section 5.3): `title`, `description`,
//! `value_description` and `notes` of a control, `title` and `description` of a params entry, a
//! command and a trigger are defined from 0.7, so a text before 0.7 that carries one is refused
//! as an unknown field at its pointer, and the same text stamped 0.7 is read.

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

fn stamp(version: &str, mut doc: Value) -> Value {
    // 0.7 says the message of a control as a way: the flat `cc` and `id` of the older forms
    // become the way of their entry.
    if version == "0.7" {
        for (block, flat, kind) in [("cc_params", "cc", "cc"), ("params", "id", "sysex")] {
            let list = doc.get_mut(block).and_then(|b| b.get_mut("entries")).and_then(Value::as_array_mut);
            for entry in list.into_iter().flatten().filter_map(Value::as_object_mut) {
                if let Some(n) = entry.remove(flat) {
                    let way = if kind == "sysex" { json!({"ctrlType": kind, "id": n}) } else { json!({"ctrlType": kind, "ctrlIndex": [n]}) };
                    entry.insert("ways".into(), json!([way]));
                }
            }
        }
    }
    // 0.4 has no `$schema`.
    if version != "0.4" {
        doc["$schema"] = json!(format!("https://w3id.org/osd/{version}/device.schema.json"));
    }
    doc
}

fn base() -> Value {
    json!({"device": {"name": "Synth", "osc_prefix": "/synth"}})
}

fn with(block: &str, value: Value) -> Value {
    let mut doc = base();
    doc[block] = value;
    doc
}

fn control(extra: Value) -> Value {
    let mut entry = json!({"osc": "/filter/cutoff", "cc": 74});
    entry.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    with("cc_params", json!({"channel": 0, "entries": [entry]}))
}

fn params_entry(extra: Value) -> Value {
    let mut entry = json!({"osc": "/osc1/shape", "id": 5});
    entry.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    with("params", json!({
        "get_frame": [240, 66, 247],
        "set_frame": [240, 67, 247],
        "value_type": "u7",
        "entries": [entry],
    }))
}

fn command(extra: Value) -> Value {
    let mut c = json!({"osc": "/dump"});
    c.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    with("commands", json!([c]))
}

fn trigger(extra: Value) -> Value {
    let mut t = json!({"osc": "/kick", "sound": "Kick", "channel": 9, "channel_mode": "fixed", "note_range": [36, 36]});
    t.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    with("triggers", json!({"entries": [t]}))
}

/// Each 0.7 field of the table: a text that carries it and its pointer.
fn fields_from_0_7() -> Vec<(&'static str, Value, &'static str)> {
    vec![
        ("control title", control(json!({"title": "Cutoff"})), "/cc_params/entries/0/title"),
        ("control description", control(json!({"description": "The cutoff."})), "/cc_params/entries/0/description"),
        ("control value_description", control(json!({"value_description": "0 is closed."})), "/cc_params/entries/0/value_description"),
        ("control notes", control(json!({"notes": "A note."})), "/cc_params/entries/0/notes"),
        ("params title", params_entry(json!({"title": "Shape"})), "/params/entries/0/title"),
        ("params description", params_entry(json!({"description": "The shape."})), "/params/entries/0/description"),
        ("command title", command(json!({"title": "Dump"})), "/commands/0/title"),
        ("command description", command(json!({"description": "Asks for a dump."})), "/commands/0/description"),
        ("trigger title", trigger(json!({"title": "Kick"})), "/triggers/entries/0/title"),
        ("trigger description", trigger(json!({"description": "The bass drum."})), "/triggers/entries/0/description"),
    ]
}

fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

#[test]
fn a_field_defined_from_0_7_is_refused_before_0_7_as_an_unknown_field_at_its_pointer() {
    for version in ["0.4", "0.5", "0.6"] {
        for (name, doc, pointer) in fields_from_0_7() {
            let f = read(&stamp(version, doc)).expect_err(&format!("{version} {name}"));
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, pointer), "{version} {name}");
        }
    }
}

#[test]
fn the_same_text_stamped_0_7_is_read() {
    for (name, doc, _) in fields_from_0_7() {
        read(&stamp("0.7", doc)).unwrap_or_else(|f| panic!("{name}: {f}"));
    }
}

#[test]
fn the_underscore_notes_of_an_entry_are_still_read_before_0_7() {
    for version in ["0.4", "0.5", "0.6"] {
        for doc in [
            control(json!({"_name": "Cutoff", "_description": "The cutoff.", "_usage": "Sweep it.", "_notes": "A note."})),
            params_entry(json!({"_name": "Shape", "_description": "The shape.", "_notes": "A note."})),
            trigger(json!({"_name": "Kick", "_description": "The bass drum."})),
        ] {
            read(&stamp(version, doc)).unwrap_or_else(|f| panic!("{version}: {f}"));
        }
    }
}

#[test]
fn the_notes_of_a_command_are_still_read_before_0_7() {
    for version in ["0.4", "0.5", "0.6"] {
        read(&stamp(version, command(json!({"notes": "Sent after a reset."})))).unwrap_or_else(|f| panic!("{version}: {f}"));
    }
}

#[test]
fn the_first_field_from_0_7_of_a_second_entry_is_refused_at_its_own_index() {
    let doc = with("cc_params", json!({"channel": 0, "entries": [
        {"osc": "/a", "cc": 1},
        {"osc": "/b", "cc": 2, "title": "B"},
    ]}));
    let f = read(&stamp("0.6", doc)).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/cc_params/entries/1/title"));
}
