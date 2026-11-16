//! Resolving 0.5 devices: the declared `capabilities` are checked on each
//! file's own text; what only a profile or a `midi_of` target brings is
//! reported as inherited, beside the capabilities of the merge.

use osd::{Capability, FaultKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const DEVICE: &str = "https://w3id.org/osd/0.5/device.schema.json";
const PROFILE: &str = "https://w3id.org/osd/0.5/profile.schema.json";

const IDENTITY_SCRIPT: &str = "return reply[5] == 2";

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

/// A device taking the identity of profile `acme`, plus `part` at its top level.
fn device(part: Value) -> Value {
    let mut doc = json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "identity": {"profile": "acme"}});
    for (k, v) in part.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

/// Profile `acme` whose identity block holds a script.
fn script_profile() -> Value {
    json!({"$schema": PROFILE, "identity": {"reply": [240, 126, "any", 6, 2], "script": IDENTITY_SCRIPT}})
}

fn own_script() -> Value {
    json!({"programs": {"recall_script": "return {{0xC0, ctx.program}}"}})
}

#[test]
fn a_device_without_script_or_lua_takes_a_script_profile_and_inherits_lua() {
    let files = catalogue(vec![("acme/synth", device(json!({}))), ("acme/_profile", script_profile())]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.capabilities, vec![Capability::Lua]);
    assert_eq!(r.inherited, vec![Capability::Lua]);
}

#[test]
fn the_resolved_device_keeps_the_capabilities_it_declared_and_none_when_it_declared_none() {
    let files = catalogue(vec![("acme/synth", device(json!({}))), ("acme/_profile", script_profile())]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.device.capabilities, None);

    let files = catalogue(vec![
        ("acme/synth", device(json!({"capabilities": ["lua"], "programs": own_script()["programs"].clone()}))),
        ("acme/_profile", script_profile()),
    ]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.device.capabilities, Some(vec![Capability::Lua]));
}

#[test]
fn a_device_declaring_lua_without_a_script_of_its_own_is_refused_at_its_own_reading() {
    let files = catalogue(vec![("acme/synth", device(json!({"capabilities": ["lua"]}))), ("acme/_profile", script_profile())]);
    let f = osd::resolve("acme/synth", &files).expect_err("declared lua, no own script");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/synth"), "/capabilities"));
}

#[test]
fn a_device_with_its_own_script_must_declare_lua_even_under_a_profile() {
    let files = catalogue(vec![("acme/synth", device(own_script())), ("acme/_profile", script_profile())]);
    let f = osd::resolve("acme/synth", &files).expect_err("own script, lua not declared");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::BadValue, Some("acme/synth"), "/capabilities"));

    let mut with_lua = device(own_script());
    with_lua["capabilities"] = json!(["lua"]);
    let plain = json!({"$schema": PROFILE, "identity": {"reply": [240, 126, "any", 6, 2]}});
    let files = catalogue(vec![("acme/synth", with_lua), ("acme/_profile", plain)]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.capabilities, vec![Capability::Lua]);
    assert!(r.inherited.is_empty());
}

#[test]
fn a_capability_of_the_own_text_is_never_inherited_when_the_profile_also_carries_a_script() {
    let mut doc = device(own_script());
    doc["capabilities"] = json!(["lua"]);
    let files = catalogue(vec![("acme/synth", doc), ("acme/_profile", script_profile())]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.capabilities, vec![Capability::Lua]);
    assert_eq!(r.inherited, Vec::<Capability>::new());
}

#[test]
fn a_device_with_its_own_script_and_no_link_inherits_nothing() {
    let doc = json!({
        "$schema": DEVICE,
        "device": {"name": "Synth", "osc_prefix": "/synth"},
        "capabilities": ["lua"],
        "programs": own_script()["programs"].clone()
    });
    let files = catalogue(vec![("acme/synth", doc)]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.capabilities, vec![Capability::Lua]);
    assert_eq!(r.inherited, Vec::<Capability>::new());
}

#[test]
fn a_device_whose_profile_brings_no_lua_inherits_nothing() {
    let plain = json!({"$schema": PROFILE, "identity": {"reply": [240, 126, "any", 6, 2]}});
    let files = catalogue(vec![("acme/synth", device(json!({}))), ("acme/_profile", plain)]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert!(r.capabilities.is_empty());
    assert!(r.inherited.is_empty());
}

#[test]
fn a_capability_only_the_midi_of_target_brings_is_inherited() {
    let target = json!({
        "$schema": DEVICE,
        "device": {"name": "Synth", "osc_prefix": "/synth"},
        "capabilities": ["lua"],
        "programs": own_script()["programs"].clone()
    });
    let child = json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth", "revision": "2"}, "midi_of": "acme/fw-1"});
    let files = catalogue(vec![("acme/fw-1", target), ("acme/fw-2", child)]);
    let r = osd::resolve("acme/fw-2", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.capabilities, vec![Capability::Lua]);
    assert_eq!(r.inherited, vec![Capability::Lua]);
}
