//! The version a descriptor is read in: chosen by its `$schema` when it has
//! one, by the caller (`read_as`) when it has none, never guessed. A
//! 0.5 descriptor carries `$schema`, and `read` refuses one without.

mod common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};

const W3ID: &str = "https://w3id.org/osd/";
const HOST: &str = "https://synthhub.roomi-fields.com/osd/";

const KINDS: [(Kind, &str); 5] = [
    (Kind::Device, "device"),
    (Kind::Profile, "profile"),
    (Kind::Sound, "sound"),
    (Kind::Fingerprint, "fingerprint"),
    (Kind::Firmware, "firmware"),
];

/// A valid descriptor of `kind` without `$schema`, as 0.4 reads it.
fn bare(kind: Kind) -> Value {
    let from_file = |path: &str| -> Value {
        let mut v: Value = serde_json::from_str(&std::fs::read_to_string(common::repo().join(path)).unwrap()).unwrap();
        v.as_object_mut().unwrap().remove("$schema");
        v
    };
    match kind {
        Kind::Device => json!({"device": {"name": "Test", "osc_prefix": "/test"}}),
        Kind::Profile => json!({"identity": {"request": [240, 126, 127, 6, 1, 247], "reply": [240, 126, "{dev}", 6, 2, 67], "reply_text": "acme"}}),
        Kind::Sound => from_file("examples/warm-pad.sound.json"),
        Kind::Fingerprint => from_file("examples/minilogue.fingerprint.json"),
        Kind::Firmware => from_file("examples/tribe-0.3-beta.firmware.json"),
    }
}

/// `bare(kind)` with `$schema` set to `url`, as its first key.
fn with_schema(kind: Kind, url: Value) -> String {
    let mut out = serde_json::Map::new();
    out.insert("$schema".into(), url);
    out.extend(bare(kind).as_object().unwrap().clone());
    Value::Object(out).to_string()
}

fn w3id(version: &str, name: &str) -> Value {
    json!(format!("{W3ID}{version}/{name}.schema.json"))
}

/// The `$schema` of the text `write` gives back for what `text` reads as.
fn written_schema(text: &str, kind: Kind) -> Option<String> {
    let d = osd::read(text, kind).unwrap_or_else(|f| panic!("{f}"));
    let v: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    v.get("$schema").map(|s| s.as_str().unwrap().to_string())
}

#[test]
fn version_0_5_exists_and_is_read_by_the_crate() {
    assert_eq!(Version::V0_5.name(), "0.5");
    assert!(osd::VERSIONS.contains(&Version::V0_5));
    assert!(osd::VERSIONS.contains(&Version::V0_4));
    assert!(Version::V0_4 < Version::V0_5);
}

#[test]
fn criterion_7_a_w3id_schema_of_0_5_gives_version_0_5_for_each_kind() {
    for (kind, name) in KINDS {
        let text = with_schema(kind, w3id("0.5", name));
        let want = format!("{W3ID}0.5/{name}.schema.json");
        assert_eq!(written_schema(&text, kind), Some(want), "{name}");
    }
}

#[test]
fn criterion_7_a_synthhub_schema_of_0_4_gives_version_0_4() {
    // The kinds whose 0.4 schema holds `$schema`.
    for (kind, name) in [(Kind::Sound, "sound"), (Kind::Fingerprint, "fingerprint"), (Kind::Firmware, "firmware")] {
        let text = with_schema(kind, json!(format!("{HOST}0.4/{name}.schema.json")));
        let want = format!("{HOST}0.4/{name}.schema.json");
        assert_eq!(written_schema(&text, kind), Some(want), "{name}");
    }
}

#[test]
fn criterion_7_a_synthhub_schema_of_0_4_on_a_device_or_a_profile_is_an_unknown_field_at_the_schema() {
    // The 0.4 schemas of these kinds hold no `$schema`: the field is unknown there.
    for (kind, name) in [(Kind::Device, "device"), (Kind::Profile, "profile")] {
        let text = with_schema(kind, json!(format!("{HOST}0.4/{name}.schema.json")));
        let f = osd::read(&text, kind).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/$schema"), "{name}: {f}");
    }
}

#[test]
fn criterion_7_an_unknown_version_in_a_schema_is_unread_version_at_the_schema() {
    for (kind, name) in KINDS {
        for base in [W3ID, HOST] {
            let text = with_schema(kind, json!(format!("{base}0.9/{name}.schema.json")));
            let f = osd::read(&text, kind).expect_err("refused");
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnreadVersion, "/$schema"), "{name} {base}: {f}");
        }
    }
}

#[test]
fn criterion_7_a_schema_of_another_kind_is_a_bad_value_at_the_schema() {
    for (kind, name) in KINDS {
        let other = KINDS.iter().map(|(_, n)| *n).find(|n| *n != name).unwrap();
        let text = with_schema(kind, w3id("0.5", other));
        let f = osd::read(&text, kind).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{name}: {f}");
    }
}

#[test]
fn criterion_7_a_schema_that_is_not_an_osd_url_is_a_bad_value_at_the_schema() {
    for (kind, _) in KINDS {
        for url in [json!("https://example.org/osd/0.5/device.schema.json"), json!(5), json!(null)] {
            let f = osd::read(&with_schema(kind, url.clone()), kind).expect_err("refused");
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{url}: {f}");
        }
    }
}

#[test]
fn criterion_2_a_file_without_schema_is_read_as_0_4() {
    for (kind, name) in KINDS {
        let text = bare(kind).to_string();
        let d = osd::read(&text, kind).unwrap_or_else(|f| panic!("{name}: {f}"));
        let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
        assert_eq!(back, bare(kind), "{name}: written back unchanged, with no 0.5 `$schema`");
    }
}

#[test]
fn criterion_6_a_0_5_descriptor_without_schema_is_missing_at_the_schema() {
    for (kind, name) in KINDS {
        let f = osd::read_as(&bare(kind).to_string(), kind, Version::V0_5).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Missing, "/$schema"), "{name}: {f}");
    }
}

#[test]
fn criterion_6_the_caller_chooses_0_4_for_a_file_without_schema() {
    for (kind, name) in KINDS {
        let text = bare(kind).to_string();
        let a = osd::read_as(&text, kind, Version::V0_4).unwrap_or_else(|f| panic!("{name}: {f}"));
        assert_eq!(a, osd::read(&text, kind).unwrap(), "{name}");
    }
}

#[test]
fn criterion_6_the_caller_reads_in_0_5_a_descriptor_that_carries_its_schema() {
    for (kind, name) in KINDS {
        let text = with_schema(kind, w3id("0.5", name));
        let a = osd::read_as(&text, kind, Version::V0_5).unwrap_or_else(|f| panic!("{name}: {f}"));
        assert_eq!(a, osd::read(&text, kind).unwrap(), "{name}");
    }
}

#[test]
fn criterion_2_a_0_5_descriptor_refuses_what_the_format_does_not_define_where_it_is() {
    let mut v: Value = serde_json::from_str(&with_schema(Kind::Device, w3id("0.5", "device"))).unwrap();
    v["colour"] = json!("red");
    let f = osd::read(&v.to_string(), Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/colour"), "{f}");
    let mut v: Value = serde_json::from_str(&with_schema(Kind::Sound, w3id("0.5", "sound"))).unwrap();
    v["license"] = json!(3);
    let f = osd::read(&v.to_string(), Kind::Sound).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/license"), "{f}");
}

#[test]
fn criterion_2_text_that_is_not_json_is_a_syntax_fault_in_either_version() {
    for v in [Version::V0_4, Version::V0_5] {
        let f = osd::read_as("{\"$schema\": ", Kind::Device, v).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Syntax, ""), "{f}");
    }
}
