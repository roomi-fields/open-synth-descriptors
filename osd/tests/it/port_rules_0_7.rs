// @lit spec/0.7.md
//! A port name is a rule from 0.7, and a fingerprint says its system and the two signs of MIDI 2.0
//! (proposal 0004): what the reader and the schema accept and refuse, what an older text keeps, what
//! the converter writes and how a resolution joins a profile of another version.

use crate::common;

use osd::{Capability, Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn address(version: &str, kind: &str) -> String {
    format!("https://w3id.org/osd/{version}/{kind}.schema.json")
}

/// A document of `version` and `kind` holding `blocks`; 0.4 has no `$schema`.
fn doc(version: &str, kind: &str, blocks: Value) -> Value {
    let mut d = match kind {
        "device" => json!({"device": {"name": "Synth", "osc_prefix": "/synth"}}),
        "fingerprint" => json!({"observed": "2026-11-25", "device": {"vendor": "Acme", "model": "Synth", "firmware": "1.0"}}),
        _ => json!({}),
    };
    if version != "0.4" {
        d["$schema"] = json!(address(version, kind));
    }
    for (k, v) in blocks.as_object().unwrap() {
        d[k] = v.clone();
    }
    d
}

fn kind_of(word: &str) -> Kind {
    match word {
        "device" => Kind::Device,
        "profile" => Kind::Profile,
        "fingerprint" => Kind::Fingerprint,
        _ => panic!("{word}"),
    }
}

fn written(d: &Value, kind: &str) -> Value {
    let read = osd::read(&d.to_string(), kind_of(kind)).unwrap_or_else(|f| panic!("{d}: {f}"));
    serde_json::from_str(&osd::write(&read)).unwrap()
}

fn refused(d: &Value, kind: &str) -> (FaultKind, String) {
    let f = osd::read(&d.to_string(), kind_of(kind)).expect_err(&format!("accepted: {d}"));
    (f.kind, f.pointer)
}

fn converted(d: &Value, kind: &str) -> Value {
    let out = osd::convert_to(&d.to_string(), kind_of(kind), Version::V0_7).unwrap_or_else(|f| panic!("{d}: {f}"));
    osd::read_as(&out, kind_of(kind), Version::V0_7).unwrap_or_else(|f| panic!("{out}: {f}"));
    serde_json::from_str(&out).unwrap()
}

const STACKS: [(&str, &str); 7] = [
    ("winmm", "windows"), ("windows-midi-services", "windows"), ("winrt", "windows"), ("alsa-seq", "linux"),
    ("alsa-rawmidi", "linux"), ("jack", "linux"), ("coremidi", "macos"),
];

// ---- reading 0.7

#[test]
fn every_stack_is_accepted_with_its_platform_alone_on_a_rule_and_on_a_fingerprint_by_reader_and_schema() {
    let schemas = common::Schemas::load("0.7");
    for (stack, own) in STACKS {
        for platform in ["windows", "linux", "macos"] {
            let rule = json!({"operator": "contains", "value": "synth", "platform": platform, "stack": stack});
            let cases = [
                ("device", doc("0.7", "device", json!({"identity": {"ports": [rule]}})), "/identity/ports/0/stack"),
                ("profile", doc("0.7", "profile", json!({"identity": {"ports": [rule]}})), "/identity/ports/0/stack"),
                ("fingerprint", doc("0.7", "fingerprint", json!({"platform": platform, "stack": stack})), "/stack"),
            ];
            for (kind, d, pointer) in cases {
                let errors = schemas.errors(kind_of(kind), &d);
                if platform == own {
                    assert_eq!(written(&d, kind), d, "{stack} {platform}");
                    assert!(errors.is_empty(), "{d}: {errors:?}");
                } else {
                    assert_eq!(refused(&d, kind), (FaultKind::BadValue, pointer.to_string()), "{d}");
                    let mut at: Vec<&str> = errors.iter().map(|(p, _)| p.as_str()).collect();
                    at.dedup();
                    assert_eq!(at, [pointer], "{d}: {errors:?}");
                }
            }
        }
    }
}

#[test]
fn a_rule_at_a_later_place_is_refused_at_its_own_index() {
    let d = doc("0.7", "device", json!({"identity": {"ports": [{"operator": "equals", "value": "a"}, {"operator": "equals", "value": ""}]}}));
    assert_eq!(refused(&d, "device"), (FaultKind::BadValue, "/identity/ports/1/value".to_string()));
}

#[test]
fn a_profile_of_0_7_refuses_a_text_name_and_a_product_instance_id_and_a_system_on_the_endpoint_rule() {
    for (identity, kind, pointer) in [
        (json!({"ports": ["Synth"]}), FaultKind::BadValue, "/identity/ports/0"),
        (json!({"product_instance_id": "A"}), FaultKind::UnknownField, "/identity/product_instance_id"),
        (json!({"endpoint_name": {"operator": "equals", "value": "a", "platform": "linux"}}), FaultKind::UnknownField, "/identity/endpoint_name/platform"),
        (json!({"endpoint_name": {"operator": "equals", "value": ""}}), FaultKind::BadValue, "/identity/endpoint_name/value"),
    ] {
        let d = doc("0.7", "profile", json!({"identity": identity}));
        assert_eq!(refused(&d, "profile"), (kind, pointer.to_string()), "{d}");
        assert!(!common::Schemas::load("0.7").errors(Kind::Profile, &d).is_empty(), "the schema accepts {d}");
    }
}

#[test]
fn a_0_7_fingerprint_reads_each_new_field_alone_and_writes_it_back_key_for_key() {
    let schemas = common::Schemas::load("0.7");
    for extra in [
        json!({"platform": "linux"}),
        json!({"endpoint_name": "synth endpoint"}),
        json!({"endpoint_name": ""}),
        json!({"product_instance_id": "Ab 01~"}),
        json!({"product_instance_id": ""}),
        json!({}),
    ] {
        let d = doc("0.7", "fingerprint", extra);
        assert_eq!(written(&d, "fingerprint"), d);
        let errors = schemas.errors(Kind::Fingerprint, &d);
        assert!(errors.is_empty(), "{d}: {errors:?}");
    }
}

// ---- before 0.7

#[test]
fn endpoint_name_is_an_unknown_field_before_0_7_on_a_device_and_a_profile() {
    for version in ["0.4", "0.5", "0.6"] {
        for kind in ["device", "profile"] {
            for value in [json!({"operator": "equals", "value": "synth"}), json!(null), json!("synth")] {
                let d = doc(version, kind, json!({"identity": {"endpoint_name": value}}));
                assert_eq!(refused(&d, kind), (FaultKind::UnknownField, "/identity/endpoint_name".to_string()), "{d}");
            }
        }
    }
}

#[test]
fn the_four_new_fields_of_a_fingerprint_are_unknown_before_0_7() {
    for version in ["0.4", "0.5", "0.6"] {
        for (key, value) in [("platform", "linux"), ("stack", "jack"), ("endpoint_name", "synth"), ("product_instance_id", "A")] {
            let d = doc(version, "fingerprint", json!({key: value}));
            assert_eq!(refused(&d, "fingerprint"), (FaultKind::UnknownField, format!("/{key}")), "{d}");
        }
    }
}

#[test]
fn a_rule_written_as_an_object_is_refused_before_0_7_at_its_entry() {
    for version in ["0.4", "0.5", "0.6"] {
        for kind in ["device", "profile"] {
            let d = doc(version, kind, json!({"identity": {"ports": ["Synth", {"operator": "contains", "value": "synth"}]}}));
            assert_eq!(refused(&d, kind), (FaultKind::BadValue, "/identity/ports/1".to_string()), "{d}");
        }
    }
}

#[test]
fn the_names_of_a_text_before_0_7_are_written_back_unchanged_an_empty_one_included() {
    for version in ["0.4", "0.5", "0.6"] {
        for kind in ["device", "profile"] {
            let d = doc(version, kind, json!({"identity": {"ports": ["Synth", "", "DIN Thru"]}}));
            assert_eq!(written(&d, kind), d);
        }
    }
}

// ---- the converter

#[test]
fn the_chain_from_0_4_and_0_5_writes_each_name_as_a_contains_rule() {
    for version in ["0.4", "0.5", "0.6"] {
        for kind in ["device", "profile"] {
            let d = doc(version, kind, json!({"identity": {"ports": ["B", "", "A", "B"]}}));
            let rule = |v: &str| json!({"operator": "contains", "value": v});
            assert_eq!(converted(&d, kind)["identity"], json!({"ports": [rule("B"), rule("A"), rule("B")]}), "{version} {kind}");
        }
    }
}

#[test]
fn the_converter_writes_no_endpoint_name_and_no_product_instance_id_and_adds_no_identity() {
    let d = doc("0.6", "device", json!({"identity": {"reply_text": "synth"}}));
    assert_eq!(converted(&d, "device")["identity"], json!({"reply_text": "synth"}));
    assert!(converted(&doc("0.6", "device", json!({})), "device").get("identity").is_none());
    for version in ["0.4", "0.5", "0.6"] {
        let d = doc(version, "fingerprint", json!({"ports": {"inputs": ["synth", ""]}}));
        let mut want = d.clone();
        want["$schema"] = json!(address("0.7", "fingerprint"));
        assert_eq!(converted(&d, "fingerprint"), want, "{version}");
    }
}

#[test]
fn converting_converted_rules_again_changes_nothing() {
    let first = converted(&doc("0.6", "device", json!({"identity": {"ports": ["Synth"]}})), "device");
    assert_eq!(converted(&first, "device"), first);
}

// ---- resolution

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn resolved(files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve("acme/synth", files).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap()
}

fn contains(v: &str) -> Value {
    json!({"operator": "contains", "value": v})
}

#[test]
fn a_0_7_programs_profile_converts_the_identity_a_0_6_profile_gave_before_it() {
    let r = resolved(&catalogue(vec![
        ("acme/synth", doc("0.6", "device", json!({"identity": {"profile": "acme"}, "programs": {"profile": "acme/banks"}}))),
        ("acme/_profile", doc("0.6", "profile", json!({"identity": {"ports": ["Synth"]}}))),
        ("acme/_profile.banks", doc("0.7", "profile", json!({"programs": {"banks": [{"name": "A", "size": 8}]}}))),
    ]));
    assert_eq!(r["$schema"], address("0.7", "device"));
    assert_eq!(r["identity"], json!({"ports": [contains("Synth")], "profile": "acme"}));
    assert_eq!(r["programs"], json!({"banks": [{"name": "A", "size": 8}], "profile": "acme/banks"}));
}

#[test]
fn a_device_of_0_4_or_0_5_under_a_0_7_profile_resolves_as_0_7() {
    for version in ["0.4", "0.5"] {
        let r = resolved(&catalogue(vec![
            ("acme/synth", doc(version, "device", json!({"identity": {"profile": "acme", "ports": ["Mine"]}}))),
            ("acme/_profile", doc("0.7", "profile", json!({"identity": {"endpoint_name": {"operator": "equals", "value": "Synth"}}}))),
        ]));
        assert_eq!(r["$schema"], address("0.7", "device"), "{version}");
        assert_eq!(r["identity"], json!({"ports": [contains("Mine")], "endpoint_name": {"operator": "equals", "value": "Synth"}, "profile": "acme"}), "{version}");
    }
}

#[test]
fn a_null_of_a_0_7_device_drops_the_names_of_a_0_6_profile() {
    let r = resolved(&catalogue(vec![
        ("acme/synth", doc("0.7", "device", json!({"identity": {"profile": "acme", "ports": null}}))),
        ("acme/_profile", doc("0.6", "profile", json!({"identity": {"ports": ["Synth"], "reply_text": "acme"}}))),
    ]));
    assert_eq!(r["identity"], json!({"reply_text": "acme", "profile": "acme"}));
}

#[test]
fn the_midi_of_link_is_joined_first_then_a_0_7_profile_makes_the_joined_device_0_7() {
    let base = doc("0.6", "device", json!({"cc_params": {"entries": [{"osc": "/volume", "cc": 7}]}}));
    let r = resolved(&catalogue(vec![
        ("acme/synth", doc("0.6", "device", json!({"midi_of": "acme/base", "identity": {"profile": "acme"}}))),
        ("acme/base", base),
        ("acme/_profile", doc("0.7", "profile", json!({"identity": {"ports": [{"operator": "equals", "value": "Synth"}]}}))),
    ]));
    assert_eq!(r["$schema"], address("0.7", "device"));
    assert_eq!(r["identity"], json!({"ports": [{"operator": "equals", "value": "Synth"}], "profile": "acme"}));
    assert_eq!(r["cc_params"]["entries"][0]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [7]}]));
}

#[test]
fn a_0_6_profile_of_a_midi_of_pair_joined_in_0_7_is_converted() {
    let r = resolved(&catalogue(vec![
        ("acme/synth", doc("0.6", "device", json!({"midi_of": "acme/base", "identity": {"profile": "acme"}}))),
        ("acme/base", doc("0.7", "device", json!({}))),
        ("acme/_profile", doc("0.6", "profile", json!({"identity": {"ports": ["Synth"]}}))),
    ]));
    assert_eq!(r["$schema"], address("0.7", "device"));
    assert_eq!(r["identity"], json!({"ports": [contains("Synth")], "profile": "acme"}));
}

#[test]
fn an_older_profile_that_does_not_convert_is_refused_in_its_file() {
    let files = catalogue(vec![
        ("acme/synth", doc("0.7", "device", json!({"programs": {"profile": "acme"}}))),
        ("acme/_profile", doc("0.4", "profile", json!({"programs": {"before": [[240, "{checksum}", 247]]}}))),
    ]);
    let alone = osd::convert_to(&files["acme/_profile"], Kind::Profile, Version::V0_7).expect_err("the profile alone does not convert");
    assert_eq!((alone.kind, alone.pointer.as_str()), (FaultKind::NotConvertible, "/programs/before/0/1"), "{alone}");
    let f = osd::resolve("acme/synth", &files).expect_err("refused");
    assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::NotConvertible, Some("acme/_profile"), "/programs/before/0/1"), "{f}");
}

/// A 0.7 profile `acme` holding one port rule, which makes the resolution that takes it 0.7.
fn profile_0_7() -> Value {
    doc("0.7", "profile", json!({"identity": {"ports": [{"operator": "equals", "value": "Synth"}]}}))
}

#[test]
fn a_device_carried_to_0_7_by_a_profile_loses_the_lua_only_its_removed_field_required() {
    for (table, key, number) in [("cc_params", "cc", 1), ("params", "id", 3)] {
        let mut entries = json!({"entries": [{"osc": "/x", "transform_reverse": "x // 2"}]});
        entries["entries"][0][key] = json!(number);
        if table == "params" {
            entries["get_frame"] = json!([240, 247]);
            entries["set_frame"] = json!([240, 247]);
            entries["value_type"] = json!("u7");
        }
        let mut d = doc("0.6", "device", json!({"capabilities": ["lua"], "identity": {"profile": "acme"}}));
        d[table] = entries;
        let files = catalogue(vec![("acme/synth", d), ("acme/_profile", profile_0_7())]);
        let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{table}: {f}"));
        assert_eq!(r.device.capabilities, None, "{table}");
        assert!(r.capabilities.is_empty() && r.inherited.is_empty(), "{table}: {:?} {:?}", r.capabilities, r.inherited);
        let out = resolved(&files);
        assert_eq!(out["$schema"], address("0.7", "device"), "{table}");
        assert!(out.get("capabilities").is_none() && out[table]["entries"][0].get("transform_reverse").is_none(), "{table}: {out}");
        osd::read(&out.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{out}: {f}"));
    }
}

#[test]
fn a_device_carried_to_0_7_by_a_profile_keeps_the_lua_a_remaining_field_requires() {
    let r = resolved(&catalogue(vec![
        ("acme/synth", doc("0.6", "device", json!({"capabilities": ["lua"], "identity": {"profile": "acme"},
            "cc_params": {"entries": [{"osc": "/x", "cc": 1, "transform": "x * 2", "transform_reverse": "x // 2"}]}}))),
        ("acme/_profile", profile_0_7()),
    ]));
    assert_eq!(r["$schema"], address("0.7", "device"));
    assert_eq!(r["capabilities"], json!(["lua"]));
}

#[test]
fn an_older_device_whose_midi_of_target_brings_a_script_resolves_under_a_0_7_profile_and_inherits_lua() {
    for version in ["0.5", "0.6"] {
        let script = json!([{"osc": "/c1", "script": "return {}"}]);
        let files = catalogue(vec![
            ("acme/synth", doc(version, "device", json!({"midi_of": "acme/base", "identity": {"profile": "acme"}}))),
            ("acme/base", doc(version, "device", json!({"capabilities": ["lua"], "custom_commands": script.clone()}))),
            ("acme/_profile", profile_0_7()),
        ]);
        let r = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{version}: {f}"));
        assert_eq!(r.device.capabilities, None, "{version}");
        assert_eq!(r.capabilities, vec![Capability::Lua], "{version}");
        assert_eq!(r.inherited, vec![Capability::Lua], "{version}");
        let out = resolved(&files);
        assert_eq!(out["$schema"], address("0.7", "device"), "{version}");
        assert_eq!(out["midi_of"], "acme/base", "{version}");
        assert_eq!(out["custom_commands"], script, "{version}");
    }
}

#[test]
fn an_older_device_that_does_not_convert_under_a_0_7_profile_is_refused_in_its_file() {
    for (version, part, pointer) in [
        // A mix with an nrpn_lsb_range and no NRPN: no way holds it (spec/0.7.md 21.3).
        (
            "0.5",
            json!({"cc_params": {"entries": [{"osc": "/x", "cc": 93, "rpn_msb": 0, "rpn_lsb": 2, "nrpn_lsb_range": [0, 15]}]}}),
            "/cc_params/entries/0",
        ),
        ("0.4", json!({"commands": [{"osc": "/x", "frame": [240, "{checksum}", 247]}]}), "/commands/0/frame/1"),
    ] {
        let mut d = doc(version, "device", part);
        d["identity"] = json!({"profile": "acme"});
        let files = catalogue(vec![("acme/synth", d), ("acme/_profile", profile_0_7())]);
        let f = osd::resolve("acme/synth", &files).expect_err("refused");
        assert_eq!((f.kind, f.file.as_deref(), f.pointer.as_str()), (FaultKind::NotConvertible, Some("acme/synth"), pointer), "{version}: {f}");
    }
}

#[test]
fn the_rule_that_joins_profiles_of_another_version_cites_every_case_that_holds_it() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let rule: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with("### 23.22 ")).skip(1).take_while(|l| !l.starts_with("##")).collect();
    let cited: Vec<&str> = rule.iter().filter(|l| l.starts_with("Cases:")).flat_map(|l| l.split('`').skip(1).step_by(2)).collect();
    let missing: Vec<&str> = [
        "0.7/resolve/profile-0-6-port-names-under-a-0-7-device",
        "0.7/resolve/profile-0-7-under-a-0-6-device",
        "0.7/resolve/profile-0-6-extending-a-0-7-profile",
        "0.7/resolve/identity-from-a-0-7-profile-programs-from-a-0-6-one",
        "0.7/resolve/profile-0-7-drops-the-lua-only-a-removed-field-required",
    ]
    .into_iter()
    .filter(|c| !cited.contains(c))
    .collect();
    assert!(missing.is_empty(), "23.22 of spec/0.7.md does not cite {missing:?}");
}

#[test]
fn a_resolution_with_no_0_7_file_stays_in_the_version_of_its_device_with_plain_names() {
    for (device, profile) in [("0.6", "0.6"), ("0.5", "0.6"), ("0.6", "0.5")] {
        let r = resolved(&catalogue(vec![
            ("acme/synth", doc(device, "device", json!({"identity": {"profile": "acme"}}))),
            ("acme/_profile", doc(profile, "profile", json!({"identity": {"ports": ["", "Synth"]}}))),
        ]));
        assert_eq!(r["$schema"], address(device, "device"));
        assert_eq!(r["identity"], json!({"ports": ["", "Synth"], "profile": "acme"}), "{device} {profile}");
    }
}
