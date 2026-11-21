//! The schema of 0.4 and 0.5 admits `null` on `identity.firmware` only among the direct keys
//! of `identity` and `programs`; 0.6 admits it on every optional one (R23). The reader refuses
//! what the schema of the file's own version refuses.

use osd::{FaultKind, Kind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn schema(v: &str, kind: &str) -> String {
    let host = if v == "0.4" { "https://synthhub.roomi-fields.com/osd" } else { "https://w3id.org/osd" };
    format!("{host}/{v}/{kind}.schema.json")
}

fn device(v: &str, block: &str, key: &str, value: Value) -> Value {
    let mut doc = json!({"device": {"name": "Synth", "osc_prefix": "/synth"}, block: {key: value}});
    stamp(&mut doc, v, "device");
    doc
}

/// A 0.4 file carries no `$schema`; the later versions name theirs.
fn stamp(doc: &mut Value, v: &str, kind: &str) {
    if v != "0.4" {
        doc["$schema"] = json!(schema(v, kind));
    }
}

fn refused_at(text: &str, kind: Kind, pointer: &str) {
    let f = osd::read(text, kind).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{text}: {f}");
}

#[test]
fn a_0_4_or_0_5_device_refuses_null_on_a_direct_key_of_identity_and_programs() {
    let keys = [
        ("identity", "request"),
        ("identity", "device_id"),
        ("identity", "family"),
        ("identity", "ports"),
        ("identity", "usb"),
        ("programs", "dump"),
        ("programs", "notes"),
        ("programs", "banks"),
        ("programs", "delay_ms"),
        ("programs", "program_change"),
    ];
    for v in ["0.4", "0.5"] {
        for (block, key) in keys {
            let doc = device(v, block, key, Value::Null);
            refused_at(&doc.to_string(), Kind::Device, &format!("/{block}/{key}"));
        }
    }
}

#[test]
fn a_0_4_or_0_5_device_reads_null_on_identity_firmware() {
    for v in ["0.4", "0.5"] {
        let doc = device(v, "identity", "firmware", Value::Null);
        osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{v}: {f}"));
    }
}

#[test]
fn a_0_6_device_reads_null_on_every_optional_direct_key_of_identity_and_programs() {
    let identity = ["request", "to", "script", "unpack", "reply", "reply_text", "family", "member", "device_id", "firmware", "ports", "usb", "sends"];
    let programs = ["selects", "program_change", "requires", "banks", "before", "after", "recall_script", "delay_ms", "dump", "notes"];
    for (block, keys) in [("identity", &identity[..]), ("programs", &programs[..])] {
        for key in keys {
            let doc = device("0.6", block, key, Value::Null);
            osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{block}.{key}: {f}"));
        }
    }
}

#[test]
fn a_null_inside_the_value_of_a_key_is_refused_in_every_version() {
    for v in ["0.4", "0.5", "0.6"] {
        let dump = device(v, "programs", "dump", json!({"kind": "request", "request": null}));
        refused_at(&dump.to_string(), Kind::Device, "/programs/dump/request");
        let id = device(v, "identity", "device_id", json!({"min": null}));
        refused_at(&id.to_string(), Kind::Device, "/identity/device_id/min");
    }
}

#[test]
fn a_0_4_or_0_5_profile_refuses_null_on_a_direct_key_of_identity_and_programs() {
    for v in ["0.4", "0.5"] {
        for (block, key) in [("identity", "request"), ("programs", "dump")] {
            let mut doc = json!({ block: {key: null} });
            stamp(&mut doc, v, "profile");
            refused_at(&doc.to_string(), Kind::Profile, &format!("/{block}/{key}"));
        }
    }
}

fn files(profile: Value, driver: Value) -> BTreeMap<String, String> {
    [("acme/_profile".to_string(), profile.to_string()), ("acme/synth".to_string(), driver.to_string())].into_iter().collect()
}

#[test]
fn the_version_of_the_file_that_carries_the_null_decides_the_refusal() {
    // A 0.6 driver that sets a key to null over a 0.5 profile: the driver's null stays read.
    let driver = json!({"$schema": schema("0.6", "device"), "device": {"name": "Synth", "osc_prefix": "/synth"},
        "programs": {"profile": "acme", "dump": null}});
    let profile5 = json!({"$schema": schema("0.5", "profile"), "programs": {"dump": {"kind": "request", "request": [240, 3, 247]}}});
    osd::resolve("acme/synth", &files(profile5, driver)).unwrap_or_else(|f| panic!("{f}"));

    // A 0.5 profile that holds the null itself is refused at the profile's file.
    let driver = json!({"$schema": schema("0.6", "device"), "device": {"name": "Synth", "osc_prefix": "/synth"},
        "programs": {"profile": "acme"}});
    let profile5 = json!({"$schema": schema("0.5", "profile"), "programs": {"dump": null}});
    let f = osd::resolve("acme/synth", &files(profile5, driver)).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/_profile"), "/programs/dump"), "{f}");

    // A 0.5 driver with the null is refused at its own file, over a 0.6 profile.
    let driver = json!({"$schema": schema("0.5", "device"), "device": {"name": "Synth", "osc_prefix": "/synth"},
        "programs": {"profile": "acme", "dump": null}});
    let profile6 = json!({"$schema": schema("0.6", "profile"), "programs": {"dump": {"kind": "request", "request": [240, 3, 247]}}});
    let f = osd::resolve("acme/synth", &files(profile6, driver)).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/synth"), "/programs/dump"), "{f}");
}
