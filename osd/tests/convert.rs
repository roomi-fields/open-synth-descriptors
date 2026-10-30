//! `osd::convert`: the 0.5 text of a 0.4 descriptor. It adds `$schema` (the
//! first key), changes no value, writes as `write` does, and is a no-op on
//! what is already 0.5.

mod common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

const KINDS: [(Kind, &str); 5] = [
    (Kind::Device, "device"),
    (Kind::Profile, "profile"),
    (Kind::Sound, "sound"),
    (Kind::Fingerprint, "fingerprint"),
    (Kind::Firmware, "firmware"),
];

/// Documents of 0.4, by kind: the texts convert starts from. Those of
/// sound, fingerprint and firmware carry a 0.4 `$schema`; those of device
/// and profile have none.
fn texts_04() -> Vec<(Kind, String)> {
    let file = |p: &str| std::fs::read_to_string(common::repo().join(p)).unwrap();
    vec![
        (Kind::Device, file("examples/devices/test_synth.json")),
        (Kind::Device, r#"{"device": {"osc_prefix": "/z", "name": "Z"}, "_note": "n"}"#.into()),
        (
            Kind::Profile,
            r#"{"identity": {"request": [240, 126, 127, 6, 1, 247], "reply": [240, 126, 0, 6, 2, 67], "reply_text": "acme"}}"#.into(),
        ),
        (Kind::Sound, file("examples/warm-pad.sound.json")),
        (Kind::Fingerprint, file("examples/minilogue.fingerprint.json")),
        (Kind::Fingerprint, file("examples/unknown-device.fingerprint.json")),
        (Kind::Firmware, file("examples/tribe-0.3-beta.firmware.json")),
    ]
}

fn name(kind: Kind) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).unwrap().1
}

fn convert(text: &str, kind: Kind) -> String {
    osd::convert(text, kind).unwrap_or_else(|f| panic!("{f}"))
}

/// The document without its `$schema`.
fn without_schema(text: &str) -> Value {
    let mut v: Value = serde_json::from_str(text).unwrap();
    v.as_object_mut().unwrap().remove("$schema");
    v
}

#[test]
fn criterion_8_every_value_is_unchanged_and_schema_is_the_first_key_in_w3id() {
    for (kind, text) in texts_04() {
        let out = convert(&text, kind);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let first = doc.as_object().unwrap().iter().next().unwrap();
        let want = format!("https://w3id.org/osd/0.5/{}.schema.json", name(kind));
        assert_eq!((first.0.as_str(), first.1.as_str()), ("$schema", Some(want.as_str())), "{kind:?}");
        assert_eq!(without_schema(&out), without_schema_midi_in_listed(&text), "{kind:?}: values changed");
    }
}

/// The document without its `$schema`, its 0.4 `midi_in` object written as
/// the 0.5 list of its types, in the order of the schema (OSD-b91.2.5).
fn without_schema_midi_in_listed(text: &str) -> Value {
    let mut v = without_schema(text);
    if let Some(Value::Object(m)) = v.get("midi_in").cloned() {
        let order = ["note_on", "note_off", "cc", "pitchbend", "aftertouch", "poly_aftertouch", "program_change"];
        let list: Vec<Value> = order.iter().filter(|t| m.contains_key(**t)).map(|t| json!(t)).collect();
        v["midi_in"] = Value::Array(list);
    }
    v
}

#[test]
fn criterion_8_a_0_4_schema_is_replaced_not_doubled() {
    for (kind, text) in texts_04().into_iter().filter(|(_, t)| t.contains("\"$schema\"")) {
        let out = convert(&text, kind);
        assert_eq!(out.matches("\"$schema\"").count(), 1, "{kind:?}");
        assert!(!out.contains("synthhub.roomi-fields.com"), "{kind:?}: a 0.4 address is left");
    }
}

#[test]
fn criterion_8_the_other_keys_and_the_layout_are_those_of_write() {
    for (kind, text) in texts_04() {
        let out = convert(&text, kind);
        let read = osd::read(&out, kind).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(osd::write(&read), out, "{kind:?}");
    }
}

#[test]
fn criterion_8_the_converted_text_reads_as_0_5_not_as_0_4() {
    for (kind, text) in texts_04() {
        let out = convert(&text, kind);
        let v = osd::read_as(&out, kind, osd::Version::V0_5).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(v, osd::read(&out, kind).unwrap(), "{kind:?}");
    }
}

#[test]
fn criterion_9_a_text_already_0_5_comes_back_identical() {
    for (kind, text) in texts_04() {
        let once = convert(&text, kind);
        assert_eq!(convert(&once, kind), once, "{kind:?}");
        // A 0.5 text not in the layout of `write` is returned as it is.
        let compact = serde_json::from_str::<Value>(&once).unwrap().to_string();
        assert_ne!(compact, once);
        assert_eq!(convert(&compact, kind), compact, "{kind:?}: compact 0.5 text");
    }
}

#[test]
fn criterion_9_converting_twice_gives_the_text_of_converting_once() {
    for (kind, text) in texts_04() {
        let once = convert(&text, kind);
        assert_eq!(convert(&convert(&text, kind), kind), once, "{kind:?}");
    }
}

#[test]
fn criterion_10_a_0_4_text_that_cannot_be_read_gives_the_fault_of_read() {
    let cases: [(Kind, Value, FaultKind, &str); 4] = [
        (Kind::Device, json!({"device": {"name": "T", "osc_prefix": "/t"}, "colour": "red"}), FaultKind::UnknownField, "/colour"),
        (Kind::Device, json!({"device": {"name": "T", "osc_prefix": "/t", "colour": 1}}), FaultKind::UnknownField, "/device/colour"),
        (Kind::Device, json!({"device": {"name": "T", "osc_prefix": "/t"}, "midi_out": {"default_channel": 1.5}}), FaultKind::BadValue, "/midi_out/default_channel"),
        (Kind::Device, json!({"device": {"name": "T"}}), FaultKind::Missing, "/device/osc_prefix"),
    ];
    for (kind, doc, want_kind, pointer) in cases {
        let text = doc.to_string();
        let f = osd::convert(&text, kind).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (want_kind, pointer), "{f}");
        assert_eq!(f, osd::read(&text, kind).unwrap_err());
    }
    let f = osd::convert("{\"device\": ", Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Syntax, ""), "{f}");
}

#[test]
fn criterion_10_a_sound_with_a_wrong_value_names_its_field() {
    let mut v: Value = serde_json::from_str(&texts_04().into_iter().find(|(k, _)| *k == Kind::Sound).unwrap().1).unwrap();
    v["license"] = json!(3);
    let f = osd::convert(&v.to_string(), Kind::Sound).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/license"), "{f}");
}

#[test]
fn criterion_10_no_0_4_field_is_refused_as_not_convertible() {
    for (kind, text) in texts_04() {
        assert!(osd::convert(&text, kind).is_ok(), "{kind:?}");
    }
}

#[test]
fn criterion_10_a_text_naming_an_unknown_version_is_an_unread_version_fault() {
    let text = json!({"$schema": "https://w3id.org/osd/0.9/firmware.schema.json", "name": "x"}).to_string();
    let f = osd::convert(&text, Kind::Firmware).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnreadVersion, "/$schema"), "{f}");
}

#[test]
fn criterion_11_a_converted_descriptor_validates_against_the_0_5_schema_of_its_kind() {
    let schemas = common::Schemas::load("0.5");
    for (kind, text) in texts_04() {
        let doc: Value = serde_json::from_str(&convert(&text, kind)).unwrap();
        assert_eq!(schemas.check(kind, &doc), None, "{kind:?}");
    }
}

#[test]
fn criterion_11_a_0_5_descriptor_without_schema_does_not_validate() {
    let schemas = common::Schemas::load("0.5");
    for (kind, text) in texts_04() {
        let doc = without_schema(&convert(&text, kind));
        let errors = schemas.errors(kind, &doc);
        assert!(!errors.is_empty(), "{kind:?}: valid without `$schema`");
    }
}

#[test]
fn criterion_1_the_0_5_schemas_have_the_w3id_address() {
    for (_, kind) in KINDS {
        let path = common::repo().join(format!("schemas/0.5/{kind}.schema.json"));
        let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(schema["$id"], format!("https://w3id.org/osd/0.5/{kind}.schema.json"), "{kind}");
    }
}

#[test]
fn criterion_2_every_0_5_schema_requires_schema_and_fixes_its_value() {
    for (_, kind) in KINDS {
        let path = common::repo().join(format!("schemas/0.5/{kind}.schema.json"));
        let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let required: Vec<&str> = schema["required"].as_array().unwrap().iter().map(|r| r.as_str().unwrap()).collect();
        assert!(required.contains(&"$schema"), "{kind}: {required:?}");
        assert_eq!(schema["properties"]["$schema"]["const"], schema["$id"], "{kind}");
    }
}
