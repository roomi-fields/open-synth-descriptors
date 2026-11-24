//! The complete descriptor of a device: what `resolve` made, without its links. No `midi_of`, no
//! profile name in `identity` or `programs`, the capabilities of the merge; `resolve` itself keeps
//! the name of the profile (23.24 of `spec/0.6.md`).

use crate::common;

use osd::{Capability, Descriptor, FaultKind, Kind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const DEVICE: &str = "https://w3id.org/osd/0.6/device.schema.json";
const PROFILE: &str = "https://w3id.org/osd/0.6/profile.schema.json";
const DEVICE_5: &str = "https://w3id.org/osd/0.5/device.schema.json";

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn text_of(d: &Descriptor) -> String {
    osd::write(d)
}

fn json_of(d: &Descriptor) -> Value {
    serde_json::from_str(&text_of(d)).unwrap()
}

fn complete_of(id: &str, files: &BTreeMap<String, String>) -> Descriptor {
    let r = osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"));
    osd::complete_descriptor(&r)
}

/// The conformance cases of 0.6 that resolve and expect a result, with their catalogue.
fn cases_with_result() -> Vec<(String, Value, BTreeMap<String, String>)> {
    let root = common::repo().join("conformance/0.6/resolve");
    let mut out = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(root).unwrap().map(|e| e.unwrap().path()).collect();
    dirs.sort();
    for dir in dirs {
        let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        if case.get("result").is_none() {
            continue;
        }
        let mut files = BTreeMap::new();
        for maker in std::fs::read_dir(dir.join("files")).unwrap().flatten() {
            for f in std::fs::read_dir(maker.path()).unwrap().flatten() {
                let id = format!("{}/{}", maker.file_name().to_string_lossy(), f.path().file_stem().unwrap().to_string_lossy());
                files.insert(id, std::fs::read_to_string(f.path()).unwrap());
            }
        }
        out.push((dir.file_name().unwrap().to_string_lossy().into_owned(), case, files));
    }
    out
}

fn resolved_of(case: &Value, files: &BTreeMap<String, String>) -> osd::Resolved {
    let got = match case.get("resolve_assembly").and_then(Value::as_str) {
        Some(id) => osd::resolve_assembly(id, case.get("device").and_then(Value::as_str), files),
        None => osd::resolve(case["resolve"].as_str().unwrap(), files),
    };
    got.unwrap_or_else(|f| panic!("{f}"))
}

#[test]
fn the_complete_descriptor_of_each_resolve_case_is_read_and_valid_against_the_schemas_of_0_6() {
    let schemas = common::Schemas::load("0.6");
    let cases = cases_with_result();
    assert_eq!(cases.len(), 14);
    let mut failures = Vec::new();
    for (name, case, files) in &cases {
        let complete = osd::complete_descriptor(&resolved_of(case, files));
        let doc = json_of(&complete);
        if let Some(e) = schemas.check(Kind::Device, &doc) {
            failures.push(format!("{name}: schema: {e}"));
        }
        if let Err(f) = osd::read(&text_of(&complete), Kind::Device) {
            failures.push(format!("{name}: read: {f}"));
        }
        if doc["$schema"] != DEVICE {
            failures.push(format!("{name}: $schema {:?}", doc["$schema"]));
        }
    }
    common::report("resolve cases failed", failures);
}

#[test]
fn the_complete_descriptor_written_and_read_again_gives_the_same_descriptor() {
    let mut failures = Vec::new();
    for (name, case, files) in &cases_with_result() {
        let complete = osd::complete_descriptor(&resolved_of(case, files));
        let text = text_of(&complete);
        match osd::read(&text, Kind::Device) {
            Ok(again) => {
                if again != complete || text_of(&again) != text {
                    failures.push(format!("{name}: the round trip changed the descriptor"));
                }
            }
            Err(f) => failures.push(format!("{name}: {f}")),
        }
    }
    common::report("resolve cases failed", failures);
}

#[test]
fn the_complete_descriptor_has_no_midi_of_no_profile_name_and_no_resolution_bookkeeping() {
    let mut failures = Vec::new();
    for (name, case, files) in &cases_with_result() {
        let doc = json_of(&osd::complete_descriptor(&resolved_of(case, files)));
        for bad in ["midi_of", "inherited", "files"] {
            if doc.get(bad).is_some() {
                failures.push(format!("{name}: {bad} present"));
            }
        }
        for block in ["identity", "programs"] {
            if doc.get(block).and_then(|b| b.get("profile")).is_some() {
                failures.push(format!("{name}: {block}.profile present"));
            }
        }
    }
    common::report("resolve cases failed", failures);
}

#[test]
fn the_name_of_the_profile_leaves_identity_and_programs_and_the_keys_it_brought_stay() {
    let files = catalogue(vec![
        (
            "acme/synth",
            json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"},
                   "identity": {"profile": "acme", "ports": null}, "programs": {"profile": "acme", "dump": null}}),
        ),
        (
            "acme/_profile",
            json!({"$schema": PROFILE, "identity": {"request": [240, 126, 127, 6, 1, 247], "ports": ["Synth"]},
                   "programs": {"selects": "sound", "dump": {"kind": "manual"}}}),
        ),
    ]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    // resolve keeps the name of the profile, as 23.24 says.
    let kept: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device.clone())))).unwrap();
    assert_eq!(kept["identity"]["profile"], "acme");
    assert_eq!(kept["programs"]["profile"], "acme");

    let doc = json_of(&osd::complete_descriptor(&r));
    assert_eq!(doc["identity"], json!({"request": [240, 126, 127, 6, 1, 247]}));
    assert_eq!(doc["programs"], json!({"selects": "sound"}));
}

#[test]
fn the_routes_of_the_midi_of_target_stay_and_the_link_leaves() {
    let route = |osc: &str| json!({"osc": osc, "script": "return {}"});
    let files = catalogue(vec![
        (
            "acme/a",
            json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "midi_of": "acme/b",
                   "custom_commands": [route("/own")], "capabilities": ["lua"]}),
        ),
        (
            "acme/b",
            json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"},
                   "capabilities": ["lua"], "custom_commands": [route("/target")]}),
        ),
    ]);
    let doc = json_of(&complete_of("acme/a", &files));
    assert!(doc.get("midi_of").is_none());
    let oscs: Vec<&str> = doc["custom_commands"].as_array().unwrap().iter().map(|c| c["osc"].as_str().unwrap()).collect();
    assert_eq!(oscs, vec!["/target", "/own"]);
}

#[test]
fn the_complete_descriptor_of_a_device_without_link_is_that_device() {
    let files = catalogue(vec![(
        "acme/synth",
        json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"},
               "identity": {"request": [240, 126, 127, 6, 1, 247]}, "programs": {"selects": "sound"}}),
    )]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    let complete = osd::complete_descriptor(&r);
    assert_eq!(complete, Descriptor::Device(Box::new(r.device.clone())));
    assert_eq!(json_of(&complete), serde_json::from_str::<Value>(&files["acme/synth"]).unwrap());
}

#[test]
fn the_capabilities_a_profile_brings_are_in_the_complete_descriptor_and_read_accepts_the_list() {
    let own = json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "identity": {"profile": "acme"}});
    let files = catalogue(vec![
        ("acme/synth", own.clone()),
        (
            "acme/_profile",
            json!({"$schema": PROFILE, "identity": {"reply": [240, 126, "any", 6, 2], "script": "return reply[5] == 2"}}),
        ),
    ]);
    let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(r.inherited, vec![Capability::Lua]);
    let complete = osd::complete_descriptor(&r);
    let doc = json_of(&complete);
    assert_eq!(doc["capabilities"], json!(["lua"]));
    assert!(doc["identity"].get("profile").is_none());
    assert!(osd::read(&text_of(&complete), Kind::Device).is_ok_and(|d| d == complete));

    // The file's own list, with lua and no field that holds Lua, is refused.
    let mut refused = own;
    refused["capabilities"] = json!(["lua"]);
    let fault = osd::read(&refused.to_string(), Kind::Device).unwrap_err();
    assert_eq!((fault.kind, fault.pointer.as_str()), (FaultKind::BadValue, "/capabilities"));
}

#[test]
fn a_file_that_links_a_target_of_an_older_version_gives_a_0_6_descriptor() {
    let files = catalogue(vec![
        ("acme/a", json!({"$schema": DEVICE, "device": {"name": "Synth", "osc_prefix": "/synth"}, "midi_of": "acme/b"})),
        (
            "acme/b",
            json!({"$schema": DEVICE_5, "device": {"name": "Synth", "osc_prefix": "/synth"},
                   "commands": [{"osc": "/c", "frame": [240, 247]}]}),
        ),
    ]);
    let complete = complete_of("acme/a", &files);
    let doc = json_of(&complete);
    assert_eq!(doc["$schema"], DEVICE);
    assert!(doc.get("midi_of").is_none());
    assert_eq!(complete.kind(), Kind::Device);
    assert!(osd::read(&text_of(&complete), Kind::Device).is_ok());
    assert_eq!(common::Schemas::load("0.6").check(Kind::Device, &doc), None);
}

#[test]
fn resolve_keeps_the_name_of_the_profile_and_its_link() {
    let (_, case, files) = cases_with_result().into_iter().find(|(n, _, _)| n == "profile-null-drops-key").unwrap();
    let r = resolved_of(&case, &files);
    let doc: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap();
    assert_eq!(doc, case["result"]);
}
