//! A key a block sets to null drops the profile's key (R23): in `identity` and
//! `programs`, the blocks a profile completes, every optional key but `profile`
//! takes `null`; the reader keeps the `null` when it does not resolve, and the
//! resolved description holds no such key. The 0.6 schema admits it at the same places.

use crate::common;

use osd::{FaultKind, Kind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const S6: &str = "https://w3id.org/osd/0.6/device.schema.json";
const P6: &str = "https://w3id.org/osd/0.6/profile.schema.json";

/// The keys of `identity` that take null, each with a value the profile may give.
fn identity_keys() -> Vec<(&'static str, Value)> {
    vec![
        ("request", json!([240, 126, 127, 6, 1, 247])),
        ("to", json!("named_ports")),
        ("script", json!("return true")),
        ("unpack", json!("lsb_8to7")),
        ("reply", json!([240, 126, "any", 6, 2, 67])),
        ("reply_text", json!("Synth")),
        ("family", json!([1, 2])),
        ("member", json!([3, 4])),
        ("device_id", json!({"default": 0, "min": 0, "max": 15})),
        ("firmware", json!({"decode": "raw_bytes", "at": [10, 11]})),
        ("ports", json!(["Synth"])),
        ("usb", json!([{"vendor_id": 1, "product_id": 2}])),
        ("sends", json!({"clock": true})),
    ]
}

/// The keys of `programs` that take null, each with a value the profile may give.
fn programs_keys() -> Vec<(&'static str, Value)> {
    vec![
        ("selects", json!("sound")),
        ("program_change", json!(true)),
        ("requires", json!(["Device mode"])),
        ("banks", json!([{"name": "A", "size": 128}])),
        ("before", json!([[240, 1, 247]])),
        ("after", json!([[240, 2, 247]])),
        ("recall_script", json!("return {}")),
        ("delay_ms", json!(100)),
        ("dump", json!({"kind": "request", "request": [240, 3, 247]})),
        ("notes", json!("Read the manual")),
    ]
}

fn device(blocks: Value) -> Value {
    let mut doc = json!({"$schema": S6, "device": {"name": "Synth", "osc_prefix": "/synth"}});
    for (k, v) in blocks.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn block_with(block: &str, key: &str, value: Value) -> Value {
    device(json!({ block: { key: value } }))
}

fn read_ok(doc: &Value) -> osd::Descriptor {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{doc}: {f}"))
}

fn written(d: &osd::Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn refused_at(doc: &Value, pointer: &str) {
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{doc}: {f}");
}

fn all_keys() -> Vec<(&'static str, &'static str, Value)> {
    let i = identity_keys().into_iter().map(|(k, v)| ("identity", k, v));
    i.chain(programs_keys().into_iter().map(|(k, v)| ("programs", k, v))).collect()
}

fn files(profile: Value, driver: Value) -> BTreeMap<String, String> {
    [("acme/_profile".to_string(), profile.to_string()), ("acme/synth".to_string(), driver.to_string())].into_iter().collect()
}

fn profile_of(block: &str, keys: &[(&str, Value)]) -> Value {
    let mut b = serde_json::Map::new();
    for (k, v) in keys {
        b.insert(k.to_string(), v.clone());
    }
    json!({"$schema": P6, block: b})
}

fn resolved(files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve("acme/synth", files).unwrap_or_else(|f| panic!("{f}"));
    written(&osd::Descriptor::Device(Box::new(r.device)))
}

#[test]
fn a_file_read_without_resolution_keeps_a_null_on_every_optional_key_of_identity_and_programs() {
    for (block, key, _) in all_keys() {
        let mut doc = block_with(block, key, Value::Null);
        doc[block]["profile"] = json!("acme");
        let d = read_ok(&doc);
        assert_eq!(written(&d)[block][key], Value::Null, "{block}.{key}");
        assert!(written(&d)[block].as_object().unwrap().contains_key(key), "{block}.{key} kept");
    }
}

#[test]
fn a_null_key_reads_in_a_block_that_names_no_profile() {
    let doc = block_with("programs", "dump", Value::Null);
    assert_eq!(written(&read_ok(&doc))["programs"], json!({"dump": null}));
}

#[test]
fn the_schema_admits_null_on_exactly_the_keys_of_identity_and_programs_that_take_it() {
    let schemas = common::Schemas::load("0.6");
    for (block, key, value) in all_keys() {
        let doc = block_with(block, key, Value::Null);
        assert_eq!(schemas.check(Kind::Device, &doc), None, "{block}.{key}: null");
        let doc = block_with(block, key, value);
        assert_eq!(schemas.check(Kind::Device, &doc), None, "{block}.{key}: value");
    }
}

#[test]
fn the_schema_refuses_null_on_profile_and_on_a_key_of_a_block_no_profile_completes() {
    let schemas = common::Schemas::load("0.6");
    for block in ["identity", "programs"] {
        assert!(schemas.check(Kind::Device, &block_with(block, "profile", Value::Null)).is_some(), "{block}.profile");
    }
    for (block, key) in [("midi_out", "default_channel"), ("device", "name")] {
        assert!(schemas.check(Kind::Device, &block_with(block, key, Value::Null)).is_some(), "{block}.{key}");
    }
}

#[test]
fn the_schema_refuses_null_in_the_sub_object_of_a_key() {
    let schemas = common::Schemas::load("0.6");
    let cases = [
        ("programs", json!({"dump": {"kind": "request", "request": null}})),
        ("identity", json!({"device_id": {"min": null}})),
        ("identity", json!({"firmware": {"decode": "raw_bytes", "at": null}})),
        ("programs", json!({"banks": [{"name": "A", "size": null}]})),
    ];
    for (block, value) in cases {
        let doc = device(json!({ block: value }));
        assert!(schemas.check(Kind::Device, &doc).is_some(), "{doc}");
    }
}

#[test]
fn null_in_a_sub_object_of_a_key_is_refused() {
    refused_at(&device(json!({"programs": {"dump": {"kind": "request", "request": null}}})), "/programs/dump/request");
    refused_at(&device(json!({"identity": {"device_id": {"min": null}}})), "/identity/device_id/min");
}

#[test]
fn null_on_a_key_of_a_block_no_profile_completes_is_refused() {
    refused_at(&device(json!({"midi_out": {"default_channel": null}})), "/midi_out/default_channel");
    refused_at(&device(json!({"device": {"name": null}})), "/device/name");
}

#[test]
fn null_on_the_profile_a_block_names_is_refused() {
    refused_at(&block_with("identity", "profile", Value::Null), "/identity/profile");
    refused_at(&block_with("programs", "profile", Value::Null), "/programs/profile");
}

#[test]
fn a_null_key_is_absent_from_the_resolved_block_and_the_other_profile_keys_stay() {
    for (block, keys) in [("identity", identity_keys()), ("programs", programs_keys())] {
        for (dropped, _) in &keys {
            let driver = device(json!({ block: {"profile": "acme", *dropped: null} }));
            let doc = resolved(&files(profile_of(block, &keys), driver));
            let got = doc[block].as_object().unwrap();
            assert!(!got.contains_key(*dropped), "{block}.{dropped} is dropped: {doc}");
            for (k, v) in &keys {
                if k != dropped {
                    assert_eq!(got.get(*k), Some(v), "{block}.{k} stays when {dropped} is dropped");
                }
            }
            assert_eq!(got["profile"], json!("acme"));
        }
    }
}

#[test]
fn programs_dump_null_removes_the_dump_of_the_profile_and_identity_ports_null_its_ports() {
    let profile = json!({"$schema": P6,
        "identity": {"request": [240, 126, 127, 6, 1, 247], "ports": ["Synth"]},
        "programs": {"selects": "sound", "dump": {"kind": "manual"}}});
    let driver = device(json!({"identity": {"profile": "acme", "ports": null}, "programs": {"profile": "acme", "dump": null}}));
    let doc = resolved(&files(profile, driver));
    assert_eq!(doc["identity"], json!({"profile": "acme", "request": [240, 126, 127, 6, 1, 247]}));
    assert_eq!(doc["programs"], json!({"profile": "acme", "selects": "sound"}));
}

#[test]
fn a_null_on_a_key_the_profile_does_not_give_is_accepted_and_the_key_is_absent() {
    let profile = json!({"$schema": P6, "identity": {"reply_text": "Synth"}, "programs": {"selects": "sound"}});
    let driver = device(json!({"identity": {"profile": "acme", "ports": null}, "programs": {"profile": "acme", "dump": null}}));
    let doc = resolved(&files(profile, driver));
    assert_eq!(doc["identity"], json!({"profile": "acme", "reply_text": "Synth"}));
    assert_eq!(doc["programs"], json!({"profile": "acme", "selects": "sound"}));
}

#[test]
fn a_null_on_the_profile_key_of_a_resolved_file_is_refused() {
    let profile = json!({"$schema": P6, "identity": {"reply_text": "Synth"}});
    let driver = device(json!({"identity": {"profile": null}}));
    let f = osd::resolve("acme/synth", &files(profile, driver)).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/identity/profile"), "{f}");
}
