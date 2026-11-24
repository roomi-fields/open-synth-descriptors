//! A sound's `kind` admits an extension `x-<registrant>-<term>` in 0.6, as a
//! device's `selects` does; 0.4 and 0.5 stay closed. The 0.6 fingerprint case
//! is a 0.6 case that carries a whole fingerprint.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

fn sound(version: &str, kind: &str, with_device: bool) -> Value {
    let mut doc = json!({
        "$schema": format!("https://w3id.org/osd/{version}/sound.schema.json"),
        "name": "Track",
        "kind": kind,
        "content": { "settings": { "/amp/attack": [90] } }
    });
    if with_device {
        doc["device"] = json!({ "descriptor": "acme/synth", "vendor": "Acme", "model": "Synth" });
    }
    doc
}

#[test]
fn a_sound_kind_extension_is_valid_read_with_its_value_and_written_as_it_is() {
    let schemas = common::Schemas::load("0.6");
    for kind in ["x-acme-track", "x-a1-two-words", "x-osd-track"] {
        let doc = sound("0.6", kind, true);
        assert_eq!(schemas.check(Kind::Sound, &doc), None, "{kind}");
        let read = osd::read(&doc.to_string(), Kind::Sound).unwrap_or_else(|f| panic!("{kind}: {f}"));
        let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
        assert_eq!(back, doc, "{kind}");
        assert_eq!(back["kind"], kind);
    }
}

#[test]
fn a_sound_kind_outside_the_list_and_the_extension_form_is_refused_at_the_value() {
    let schemas = common::Schemas::load("0.6");
    for kind in ["track", "x-acme", "x-Acme-track", "x--track", "X-acme-track", "x-acme-Track", "x-acme-", ""] {
        let doc = sound("0.6", kind, true);
        assert!(schemas.check(Kind::Sound, &doc).is_some(), "schema accepts {kind:?}");
        let fault = osd::read(&doc.to_string(), Kind::Sound).expect_err(kind);
        assert_eq!(fault.kind, FaultKind::BadValue, "{kind:?}");
        assert_eq!(fault.pointer, "/kind", "{kind:?}");
    }
}

#[test]
fn a_sound_of_an_extension_kind_needs_its_device_like_any_kind_but_clip() {
    let schemas = common::Schemas::load("0.6");
    let doc = sound("0.6", "x-acme-track", false);
    assert!(schemas.check(Kind::Sound, &doc).is_some(), "the schema accepts an extension kind without a device");
    let fault = osd::read(&doc.to_string(), Kind::Sound).expect_err("no device");
    assert_eq!(fault.pointer, "/device");
    // A clip stays the one kind without a device.
    let clip = sound("0.6", "clip", false);
    assert_eq!(schemas.check(Kind::Sound, &clip), None);
    osd::read(&clip.to_string(), Kind::Sound).unwrap();
}

#[test]
fn the_closed_sound_kinds_of_0_4_and_0_5_refuse_an_extension() {
    // The reader reads 0.5 and 0.6; the 0.4 schema is checked alone.
    for version in ["0.4", "0.5"] {
        let schemas = common::Schemas::load(version);
        let doc = sound(version, "x-acme-track", true);
        assert!(schemas.check(Kind::Sound, &doc).is_some(), "{version} schema accepts an extension kind");
    }
    let doc = sound("0.5", "x-acme-track", true);
    let fault = osd::read(&doc.to_string(), Kind::Sound).expect_err("0.5");
    assert_eq!(fault.kind, FaultKind::BadValue);
    assert_eq!(fault.pointer, "/kind");
}

#[test]
fn a_conformance_case_of_0_6_shows_a_sound_kind_extension_and_another_refuses_a_kind_outside_the_list() {
    let dir = common::repo().join("conformance/0.6/read");
    let mut accepted = 0;
    let mut refused = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let (Ok(case), Ok(desc)) = (std::fs::read_to_string(path.join("case.json")), std::fs::read_to_string(path.join("descriptor.json"))) else {
            continue;
        };
        let (case, desc): (Value, Value) = (serde_json::from_str(&case).unwrap(), serde_json::from_str(&desc).unwrap());
        if case["read"] != "sound" {
            continue;
        }
        let Some(kind) = desc["kind"].as_str() else { continue };
        if kind.starts_with("x-") && case.get("result").is_some() {
            assert_eq!(case["result"]["kind"], kind, "{}", path.display());
            accepted += 1;
        }
        if !["sound", "kit", "sequence", "project", "bank", "clip", "performance", "scene", "setlist"].contains(&kind)
            && !kind.starts_with("x-")
            && case["fault"]["kind"] == "BadValue"
            && case["fault"]["pointer"] == "/kind"
        {
            refused += 1;
        }
    }
    assert!(accepted >= 1, "no 0.6 sound case reads an extension kind");
    assert!(refused >= 1, "no 0.6 sound case refuses a kind outside the list");
}

#[test]
fn the_fingerprint_case_of_0_6_is_a_0_6_fingerprint_with_ports_replies_usb_identity_and_controls() {
    let dir = common::repo().join("conformance/0.6/read/fingerprint");
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert!(!case["_about"].as_str().unwrap().contains("0.5"), "the case still calls itself a 0.5 case");
    assert_eq!(doc["$schema"], "https://w3id.org/osd/0.6/fingerprint.schema.json");
    assert!(doc["ports"].is_object(), "ports");
    assert!(doc["identity_reply"].is_string(), "identity_reply");
    assert!(doc["maker_replies"].is_array(), "maker_replies");
    assert!(doc["usb"].is_object(), "USB identity");
    assert!(doc["controls"].as_array().is_some_and(|c| !c.is_empty()), "controls");
    let read = osd::read(&text, Kind::Fingerprint).unwrap_or_else(|f| panic!("{f}"));
    let back: Value = serde_json::from_str(&osd::write(&read)).unwrap();
    assert_eq!(back, doc);
    assert_eq!(common::Schemas::load("0.6").check(Kind::Fingerprint, &doc), None);
    assert_eq!(case["result"], doc);
}
