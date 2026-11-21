//! OSD 0.6, a draft: read like 0.5 at its opening, except that `capabilities`
//! forbids a repeated entry; `osd::convert_to` carries a 0.4 or 0.5 text to it;
//! `osd::convert` stays on the latest published version, 0.5. The schemas and the
//! conformance cases of 0.6 mirror those of 0.5; its checksums are written when
//! the version is published.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::path::Path;

const KINDS: [(Kind, &str); 5] = [
    (Kind::Device, "device"),
    (Kind::Profile, "profile"),
    (Kind::Sound, "sound"),
    (Kind::Fingerprint, "fingerprint"),
    (Kind::Firmware, "firmware"),
];

fn address(version: &str, kind: &str) -> String {
    format!("https://w3id.org/osd/{version}/{kind}.schema.json")
}

fn file(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap()
}

/// The 0.5 examples, by kind.
fn examples_05() -> Vec<(Kind, String)> {
    vec![
        (Kind::Device, file("examples/0.5/synth.device.json")),
        (Kind::Profile, file("examples/0.5/acme.profile.json")),
        (Kind::Sound, file("examples/0.5/warm-pad.sound.json")),
        (Kind::Fingerprint, file("examples/0.5/minilogue.fingerprint.json")),
        (Kind::Firmware, file("examples/0.5/tribe-0.3-beta.firmware.json")),
    ]
}

/// Documents of 0.4, by kind.
fn texts_04() -> Vec<(Kind, String)> {
    vec![
        (Kind::Device, file("examples/devices/test_synth.json")),
        (Kind::Sound, file("examples/warm-pad.sound.json")),
        (Kind::Fingerprint, file("examples/minilogue.fingerprint.json")),
        (Kind::Firmware, file("examples/tribe-0.3-beta.firmware.json")),
    ]
}

fn name(kind: Kind) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).unwrap().1
}

/// The text with its 0.5 `$schema` address turned into the 0.6 one; a firmware, whose
/// `device` and `based_on` become `targets` in 0.6, goes through the conversion.
fn as_06(text: &str, kind: Kind) -> String {
    if kind == Kind::Firmware {
        return osd::convert_to(text, kind, Version::V0_6).unwrap();
    }
    text.replace(&address("0.5", name(kind)), &address("0.6", name(kind)))
}

fn value(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn without_schema(text: &str) -> Value {
    let mut v = value(text);
    v.as_object_mut().unwrap().remove("$schema");
    v
}

/// A device of version `version` whose `capabilities` is `caps` and which holds
/// Lua (a `transform`), so that `["lua"]` is the derived list.
fn lua_device(version: &str, caps: Value) -> String {
    json!({
        "$schema": address(version, "device"),
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "transform": "x * 2"}]},
        "capabilities": caps
    })
    .to_string()
}

#[test]
fn the_reader_lists_the_0_6_draft_among_its_versions_and_keeps_0_5_published() {
    assert_eq!(Version::V0_6.name(), "0.6");
    assert!(osd::VERSIONS.contains(&Version::V0_6));
    assert_eq!(osd::PUBLISHED, Version::V0_5);
    assert_ne!(Some(&osd::PUBLISHED), osd::VERSIONS.last());
}

#[test]
fn a_0_6_address_is_the_w3id_one_of_its_kind() {
    for (kind, n) in KINDS {
        let text = as_06(&examples_05().into_iter().find(|(k, _)| *k == kind).unwrap().1, kind);
        assert!(osd::read(&text, kind).is_ok(), "{n}");
        assert_eq!(osd::declared_kind(&text), Some(kind), "{n}");
        let d = osd::read_as(&text, kind, Version::V0_6).unwrap_or_else(|f| panic!("{n}: {f}"));
        assert_eq!(value(&osd::write(&d))["$schema"], address("0.6", n), "{n}");
        // Read as another version, it is refused at `$schema`.
        let f = osd::read_as(&text, kind, Version::V0_5).unwrap_err();
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{n}");
    }
}

#[test]
fn a_0_6_address_on_the_old_host_is_refused() {
    let text = file("examples/0.5/synth.device.json")
        .replace("https://w3id.org/osd/0.5/", "https://synthhub.roomi-fields.com/osd/0.6/");
    let f = osd::read(&text, Kind::Device).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnreadVersion, "/$schema"));
}

#[test]
fn a_0_5_descriptor_whose_schema_is_changed_to_0_6_reads_to_the_same_values() {
    for (kind, text) in examples_05() {
        if kind == Kind::Firmware {
            continue;
        }
        let twin = as_06(&text, kind);
        assert_ne!(twin, text, "{kind:?}");
        let d = osd::read(&twin, kind).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(without_schema(&osd::write(&d)), without_schema(&text), "{kind:?}");
    }
}

#[test]
fn a_0_6_descriptor_with_an_unknown_field_is_refused() {
    let mut top = value(&as_06(&file("examples/0.5/synth.device.json"), Kind::Device));
    top["colour"] = json!("red");
    let f = osd::read(&top.to_string(), Kind::Device).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/colour"));
    let mut nested = value(&as_06(&file("examples/0.5/synth.device.json"), Kind::Device));
    nested["device"]["colour"] = json!("red");
    let f = osd::read(&nested.to_string(), Kind::Device).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/device/colour"));
    let mut sound = value(&as_06(&file("examples/0.5/warm-pad.sound.json"), Kind::Sound));
    sound["colour"] = json!("red");
    let f = osd::read(&sound.to_string(), Kind::Sound).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/colour"));
}

#[test]
fn the_closed_vocabularies_of_0_5_apply_to_0_6() {
    let mut doc = value(&as_06(&file("examples/0.5/warm-pad.sound.json"), Kind::Sound));
    doc["license"] = json!("none");
    let f = osd::read(&doc.to_string(), Kind::Sound).unwrap_err();
    assert_eq!(f.kind, FaultKind::BadValue, "{f}");
    assert_eq!(f.pointer, "/license");
}

#[test]
fn the_0_6_reader_refuses_a_repeated_capabilities_entry() {
    let f = osd::read(&lua_device("0.6", json!(["lua", "lua"])), Kind::Device).unwrap_err();
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/capabilities"), "{f}");
    // A list without a repeat reads.
    assert!(osd::read(&lua_device("0.6", json!(["lua"])), Kind::Device).is_ok());
}

#[test]
fn the_0_5_reader_still_reads_a_repeated_capabilities_entry_once() {
    let d = osd::read(&lua_device("0.5", json!(["lua", "lua"])), Kind::Device).unwrap();
    let Some(device) = (match d {
        osd::Descriptor::Device(x) => Some(x),
        _ => None,
    }) else {
        panic!("a device")
    };
    assert_eq!(device.capabilities, Some(vec![osd::Capability::Lua]));
}

#[test]
fn the_0_6_device_schema_forbids_a_repeated_capabilities_entry() {
    let schemas = common::Schemas::load("0.6");
    let errors = schemas.errors(Kind::Device, &value(&lua_device("0.6", json!(["lua", "lua"]))));
    assert!(errors.iter().any(|(at, _)| at == "/capabilities"), "{errors:?}");
    assert!(schemas.errors(Kind::Device, &value(&lua_device("0.6", json!(["lua"])))).is_empty());
}

#[test]
fn the_0_6_schemas_are_the_six_of_the_version_with_their_w3id_ids_and_no_checksum_line() {
    let schemas = common::repo().join("schemas/0.6");
    for n in ["device", "profile", "sound", "fingerprint", "firmware", "assembly"] {
        let schema = value(&std::fs::read_to_string(schemas.join(format!("{n}.schema.json"))).unwrap());
        assert_eq!(schema["$id"], address("0.6", n), "{n}");
    }
    let files = std::fs::read_dir(&schemas).unwrap().count();
    assert_eq!(files, 6);
    // The six compile as JSON Schema.
    let _ = common::Schemas::load("0.6");
    let sums = file("schemas/SHA256SUMS");
    assert!(!sums.lines().any(|l| l.contains("0.6/")), "a 0.6 line in SHA256SUMS");
}

/// The schema without what states the role: every key that holds "role", and every
/// conditional or array item that names it.
fn strip_roles(v: &mut Value) {
    match v {
        Value::Object(o) => {
            o.retain(|k, _| !k.contains("role"));
            for key in ["if", "then", "else", "dependentRequired", "dependentSchemas", "allOf"] {
                if o.get(key).is_some_and(|x| x.to_string().contains("role")) {
                    o.remove(key);
                }
            }
            o.values_mut().for_each(strip_roles);
        }
        Value::Array(a) => {
            a.retain(|x| !x.to_string().contains("role_vocabulary"));
            a.iter_mut().for_each(strip_roles);
        }
        _ => {}
    }
}

/// A key's schema without the `null` it admits: out of `type`, `enum` and `anyOf`.
/// 0.6 admits `null` on the direct keys of `identity` and `programs` (a profile's key is dropped by it).
fn strip_null(schema: &mut Value) {
    let Some(o) = schema.as_object_mut() else { return };
    if let Some(Value::Array(types)) = o.get_mut("type") {
        types.retain(|t| t != "null");
        if types.len() == 1 {
            let only = types.remove(0);
            o.insert("type".into(), only);
        }
    }
    if let Some(Value::Array(e)) = o.get_mut("enum") {
        e.retain(|t| !t.is_null());
    }
    if let Some(Value::Array(any)) = o.get_mut("anyOf") {
        any.retain(|t| t != &json!({"type": "null"}));
        if any.len() == 1 {
            let only = any.remove(0);
            o.remove("anyOf");
            if let Value::Object(m) = only {
                o.extend(m);
            }
        }
    }
}

fn strip_block_nulls(doc: &mut Value) {
    if let Some(props) = doc["properties"]["programs"]["properties"].as_object_mut() {
        props.values_mut().for_each(strip_null);
    }
    if let Some(props) = doc["$defs"]["identity"]["properties"].as_object_mut() {
        props.values_mut().for_each(strip_null);
    }
}

#[test]
fn a_0_6_schema_differs_from_its_0_5_twin_only_by_its_address_the_unique_capabilities_the_role_and_the_default_value() {
    for (_, n) in KINDS {
        let read = |v: &str| std::fs::read_to_string(common::repo().join(format!("schemas/{v}/{n}.schema.json"))).unwrap();
        let mut now = value(&read("0.6"));
        let mut before = value(&read("0.5"));
        if n == "device" {
            assert_eq!(now["properties"]["capabilities"]["uniqueItems"], json!(true));
            now["properties"]["capabilities"].as_object_mut().unwrap().remove("uniqueItems");
        }
        // The role (`role` on an entry, `role_vocabulary`, and what requires it) is 0.6's own.
        strip_roles(&mut now);
        // The default value of a control (`default` on a CC entry and on a param entry) is 0.6's own.
        if n == "device" {
            for entry in ["cc_entry", "param_entry"] {
                let props = now["$defs"][entry]["properties"].as_object_mut().unwrap();
                assert!(props.remove("default").is_some(), "{entry} has no default");
                // The precision fields of a control (MIDI 2.0) are 0.6's own too.
                for field in ["priority", "step_count", "transmit", "recognize"] {
                    assert!(props.remove(field).is_some(), "{entry} has no {field}");
                }
            }
            let props = now["$defs"]["cc_entry"]["properties"].as_object_mut().unwrap();
            assert!(props.remove("values").is_some(), "cc_entry has no values");
            // The further ways of a control (`also`, and their definition) are 0.6's own.
            assert!(props.remove("also").is_some(), "cc_entry has no also");
            assert!(now["$defs"].as_object_mut().unwrap().remove("cc_way").is_some(), "no def cc_way");
            // A file with `midi_of` takes the frame and the value type of `params` from its target:
            // 0.6 requires only `entries` there, and asks the three keys of a file without `midi_of`.
            let params = now["properties"]["params"].as_object_mut().unwrap();
            assert_eq!(params["required"], json!(["entries"]));
            params.insert("required".into(), json!(["get_frame", "set_frame", "value_type", "entries"]));
            let text = params["description"].as_str().unwrap();
            assert!(text.contains("midi_of"));
            params.insert("description".into(), json!("Parameters read and written by SysEx through one shared message shape."));
            let all_of = now["allOf"].as_array_mut().unwrap();
            assert_eq!(all_of.len(), 2, "allOf holds the lua rule and the params rule");
            assert!(all_of[1]["if"]["not"]["required"] == json!(["midi_of"]));
            all_of.pop();
            // The `values` of a param entry stays, its description saying the range check is 0.6's.
            let values = now["$defs"]["param_entry"]["properties"]["values"].as_object_mut().unwrap();
            assert!(values["description"].as_str().unwrap().contains("range"));
            values.insert("description".into(), json!("Named values."));
            // The sound transfers (the elements of a message that carries a sound) are 0.6's own.
            assert!(now["properties"].as_object_mut().unwrap().remove("sound_transfers").is_some(), "device has no sound_transfers");
            // What a device receives (`resources`, and their definition) is 0.6's own.
            assert!(now["properties"].as_object_mut().unwrap().remove("resources").is_some(), "device has no resources");
            assert!(now["$defs"].as_object_mut().unwrap().remove("resource").is_some(), "no def resource");
            // Whether a bank can be written, and the reply timeout of a dump, are 0.6's own.
            assert!(now["$defs"]["bank"]["properties"].as_object_mut().unwrap().remove("writable").is_some(), "bank has no writable");
            let dump = now["properties"]["programs"]["properties"]["dump"]["properties"].as_object_mut().unwrap();
            assert!(dump.remove("reply_timeout_ms").is_some(), "dump has no reply_timeout_ms");
            // What a virtual instrument stands for, and how true it is, are 0.6's own.
            let device = now["$defs"]["device"].as_object_mut().unwrap();
            for key in ["dependentRequired", "dependentSchemas"] {
                assert!(device.remove(key).is_some(), "device has no {key}");
            }
            let props = device["properties"].as_object_mut().unwrap();
            for field in ["stands_for", "fidelity"] {
                assert!(props.remove(field).is_some(), "device has no {field}");
            }
            let defs = now["$defs"].as_object_mut().unwrap();
            for def in ["sound_transfer", "sound_elements", "sound_element", "sound_data_element"] {
                assert!(defs.remove(def).is_some(), "no def {def}");
            }
            // The settings of a sound (their place in its data) are 0.6's own: their definitions begin with `sound_`.
            let more: Vec<String> = defs.keys().filter(|k| k.starts_with("sound_")).cloned().collect();
            assert!(more.iter().any(|k| k.starts_with("sound_setting")), "no def sound_setting…: {more:?}");
            for def in more {
                defs.remove(&def);
            }
        }
        // The targets of a firmware (`targets`, and their definitions) replace its `device` and `based_on`.
        if n == "firmware" {
            let drop = |v: &mut Value, fields: &[&str]| {
                for f in fields {
                    assert!(v["properties"].as_object_mut().unwrap().remove(*f).is_some(), "firmware has no {f}");
                }
                let req = v["required"].as_array_mut().unwrap();
                req.retain(|r| !fields.contains(&r.as_str().unwrap()));
                v.as_object_mut().unwrap().remove("$defs");
            };
            drop(&mut now, &["targets"]);
            drop(&mut before, &["device", "based_on"]);
        }
        // The identities other tools give a sound (`external_ids`) are 0.6's own.
        if n == "sound" {
            assert!(now["properties"].as_object_mut().unwrap().remove("external_ids").is_some(), "sound has no external_ids");
        }
        // The `null` of a direct key of `identity` and `programs` is 0.6's own.
        if n == "device" {
            strip_block_nulls(&mut now);
            strip_block_nulls(&mut before);
        }
        // Compare once the version names are the same everywhere.
        let norm = |v: &mut Value| {
            *v = value(&v.to_string().replace("osd/0.6/", "osd/0.5/").replace("0.6", "0.5"));
        };
        norm(&mut now);
        norm(&mut before);
        assert_eq!(now, before, "{n}");
    }
}

#[test]
fn convert_to_0_6_carries_a_0_5_text_with_its_schema_changed_and_its_values_kept() {
    for (kind, text) in examples_05() {
        // A firmware descriptor's `device` and `based_on` become targets (firmware_targets).
        if kind == Kind::Firmware {
            continue;
        }
        let out = osd::convert_to(&text, kind, Version::V0_6).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(value(&out)["$schema"], address("0.6", name(kind)), "{kind:?}");
        assert_eq!(without_schema(&out), without_schema(&text), "{kind:?}");
        assert!(osd::read_as(&out, kind, Version::V0_6).is_ok(), "{kind:?}");
    }
}

#[test]
fn convert_to_0_6_carries_a_0_4_text_through_0_5() {
    for (kind, text) in texts_04() {
        if kind == Kind::Firmware {
            continue;
        }
        let out = osd::convert_to(&text, kind, Version::V0_6).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        let via_05 = osd::convert(&text, kind).unwrap();
        assert_eq!(value(&out)["$schema"], address("0.6", name(kind)), "{kind:?}");
        assert_eq!(without_schema(&out), without_schema(&via_05), "{kind:?}");
        assert_eq!(out, as_06(&via_05, kind), "{kind:?}");
    }
}

#[test]
fn convert_to_0_5_of_a_0_4_text_is_what_convert_gives() {
    for (kind, text) in texts_04() {
        assert_eq!(osd::convert_to(&text, kind, Version::V0_5).unwrap(), osd::convert(&text, kind).unwrap(), "{kind:?}");
    }
}

#[test]
fn convert_to_0_6_writes_a_repeated_capabilities_entry_once() {
    let out = osd::convert_to(&lua_device("0.5", json!(["lua", "lua"])), Kind::Device, Version::V0_6).unwrap();
    assert_eq!(value(&out)["capabilities"], json!(["lua"]));
    assert_eq!(value(&out)["$schema"], address("0.6", "device"));
    assert!(osd::read(&out, Kind::Device).is_ok());
}

#[test]
fn convert_to_the_version_of_the_text_returns_it_as_it_is() {
    for (kind, text) in examples_05() {
        let twin = as_06(&text, kind);
        let compact = value(&twin).to_string();
        assert_eq!(osd::convert_to(&compact, kind, Version::V0_6).unwrap(), compact, "{kind:?}");
        assert_eq!(osd::convert_to(&text, kind, Version::V0_5).unwrap(), osd::convert(&text, kind).unwrap(), "{kind:?}");
    }
}

#[test]
fn convert_to_the_same_version_writes_a_repeated_capabilities_entry_once() {
    let out = osd::convert_to(&lua_device("0.5", json!(["lua", "lua"])), Kind::Device, Version::V0_5).unwrap();
    assert_eq!(value(&out)["capabilities"], json!(["lua"]));
}

#[test]
fn convert_to_an_older_version_than_the_source_is_refused_at_the_schema() {
    for (kind, text) in examples_05() {
        let twin = as_06(&text, kind);
        let f = osd::convert_to(&twin, kind, Version::V0_5).unwrap_err();
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{kind:?}: {f}");
        let f = osd::convert_to(&twin, kind, Version::V0_4).unwrap_err();
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{kind:?}: {f}");
    }
}

#[test]
fn convert_to_0_4_is_refused_at_the_schema() {
    for (kind, text) in examples_05().into_iter().chain(texts_04()) {
        let f = osd::convert_to(&text, kind, Version::V0_4).unwrap_err();
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{kind:?}: {f}");
    }
}

#[test]
fn convert_given_a_0_6_text_is_refused_at_the_schema() {
    for (kind, text) in examples_05() {
        let f = osd::convert(&as_06(&text, kind), kind).unwrap_err();
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"), "{kind:?}: {f}");
    }
}

#[test]
fn convert_keeps_the_latest_published_version_as_its_target() {
    for (kind, text) in texts_04() {
        let out = osd::convert(&text, kind).unwrap();
        assert_eq!(value(&out)["$schema"], address(osd::PUBLISHED.name(), name(kind)), "{kind:?}");
    }
}

/// The cases of a folder, by name.
fn case_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

/// A case file with the 0.5 address turned into the 0.6 one, `_about` left out.
fn twin_of(text: &str) -> Value {
    let mut v = value(&text.replace("https://w3id.org/osd/0.5/", "https://w3id.org/osd/0.6/"));
    if let Some(o) = v.as_object_mut() {
        o.remove("_about");
    }
    v
}

#[test]
fn every_reading_case_of_0_5_has_its_0_6_twin_with_the_same_expected_result() {
    let root = common::repo().join("conformance");
    let before = case_names(&root.join("0.5/read"));
    let now = case_names(&root.join("0.6/read"));
    assert!(before.iter().all(|n| now.contains(n)), "the cases of 0.6 hold those of 0.5");
    let mut failures = Vec::new();
    for n in &before {
        // A firmware case names targets in 0.6: it is the conversion of its 0.5 twin (firmware_targets).
        let kind = |v: &str| value(&std::fs::read_to_string(root.join(v).join("read").join(n).join("case.json")).unwrap())["read"].clone();
        if kind("0.5") == json!("firmware") {
            continue;
        }
        for f in ["descriptor.json", "case.json"] {
            let a = std::fs::read_to_string(root.join("0.5/read").join(n).join(f)).unwrap();
            let b = std::fs::read_to_string(root.join("0.6/read").join(n).join(f)).unwrap();
            if twin_of(&a) != twin_of(&b) {
                failures.push(format!("{n}/{f}"));
            }
        }
    }
    common::report("0.6 cases differ from their 0.5 twin", failures);
}
