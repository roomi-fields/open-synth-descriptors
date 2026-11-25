//! OSD 0.7, a candidate build. `osd::convert_to` carries 0.4, 0.5 and 0.6 to it by an ordered
//! chain of rules; a descriptor that has no control, no params entry and no source reads the same
//! in 0.6 and 0.7 but for its `$schema`; `PUBLISHED` stays 0.6.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const KINDS: [(Kind, &str); 6] = [
    (Kind::Device, "device"),
    (Kind::Profile, "profile"),
    (Kind::Sound, "sound"),
    (Kind::Fingerprint, "fingerprint"),
    (Kind::Firmware, "firmware"),
    (Kind::Assembly, "assembly"),
];

fn address(version: &str, kind: &str) -> String {
    format!("https://w3id.org/osd/{version}/{kind}.schema.json")
}

fn file(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap()
}

fn name(kind: Kind) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).unwrap().1
}

/// The 0.6 examples, by kind.
fn examples_06() -> Vec<(Kind, String)> {
    vec![
        (Kind::Device, file("examples/0.6/minilogue.device.json")),
        (Kind::Sound, file("examples/0.6/warm-pad.sound.json")),
        (Kind::Fingerprint, file("examples/0.6/minilogue.fingerprint.json")),
        (Kind::Firmware, file("examples/0.6/tribe-0.3-beta.firmware.json")),
        (Kind::Assembly, file("examples/0.6/tribe-on-fm1.assembly.json")),
    ]
}

/// The text of a 0.6 example as 0.7 says it: the converter's text.
fn as_07(text: &str, kind: Kind) -> String {
    assert!(text.contains(&address("0.6", name(kind))), "the example names 0.6");
    osd::convert_to(text, kind, Version::V0_7).unwrap_or_else(|f| panic!("{kind:?}: {f}"))
}

/// A 0.6 document as the converter writes it for 0.7 where the controls are concerned: the `cc`
/// and the `range` of an entry of `cc_params` are one way, the `id` of an entry of `params` too.
fn with_ways(want: &mut Value) {
    for (block, flat, kind) in [("cc_params", "cc", "cc"), ("params", "id", "sysex")] {
        let list = want.get_mut(block).and_then(|b| b.get_mut("entries")).and_then(Value::as_array_mut);
        for entry in list.into_iter().flatten().filter_map(Value::as_object_mut) {
            let mut way = if let Some(n) = entry.remove(flat) {
                json!({"ctrlType": kind, "ctrlIndex": [n]})
            } else if let (Some(msb), Some(lsb)) = (entry.remove("nrpn_msb"), entry.remove("nrpn_lsb")) {
                json!({"ctrlType": "nrpn", "ctrlIndex": [msb, lsb]})
            } else {
                continue;
            };
            for moved in ["value_bits", "range"] {
                if let Some(v) = entry.remove(moved) {
                    way[moved] = v;
                }
            }
            entry.insert("ways".into(), json!([way]));
        }
    }
}

fn schema_of(text: &str) -> String {
    let v: Value = serde_json::from_str(text).unwrap();
    v["$schema"].as_str().unwrap().to_string()
}

#[test]
fn the_versions_read_include_0_7_named_0_7_and_the_published_one_stays_0_6() {
    assert_eq!(Version::V0_7.name(), "0.7");
    assert!(osd::VERSIONS.contains(&Version::V0_7));
    assert!(Version::V0_6 < Version::V0_7);
    assert_eq!(osd::PUBLISHED, Version::V0_6);
}

#[test]
fn a_descriptor_of_each_kind_naming_0_7_reads_as_0_7() {
    for (kind, text) in examples_06() {
        let d = osd::read(&as_07(&text, kind), kind).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(d.schema(), Some(Version::V0_7), "{kind:?}");
    }
}

#[test]
fn a_0_7_text_reads_back_through_read_as_and_writes_back_naming_0_7() {
    for (kind, text) in examples_06() {
        let text = as_07(&text, kind);
        let d = osd::read_as(&text, kind, Version::V0_7).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        let back = osd::write(&d);
        assert_eq!(schema_of(&back), address("0.7", name(kind)), "{kind:?}");
        let again = osd::read_as(&back, kind, Version::V0_7).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        assert_eq!(again, d, "{kind:?}");
    }
}

#[test]
fn a_0_7_text_read_as_0_6_is_refused_at_its_schema() {
    let (kind, text) = examples_06().remove(0);
    let f = osd::read_as(&as_07(&text, kind), kind, Version::V0_6).expect_err("0.7 is not 0.6");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"));
}

#[test]
fn the_kind_a_0_7_address_declares_is_found() {
    for (kind, text) in examples_06() {
        assert_eq!(osd::declared_kind(&as_07(&text, kind)), Some(kind));
    }
}

#[test]
fn converting_a_0_6_text_to_0_7_changes_its_schema_its_sources_and_its_controls_and_nothing_else() {
    for (kind, text) in examples_06() {
        let out = osd::convert_to(&text, kind, Version::V0_7).unwrap_or_else(|f| panic!("{kind:?}: {f}"));
        let mut want: Value = serde_json::from_str(&text).unwrap();
        want["$schema"] = json!(address("0.7", name(kind)));
        // The other changes: `_sources` is `sources`, the controls are ways.
        if let Some(sources) = want.as_object_mut().and_then(|o| o.remove("_sources")) {
            want["sources"] = sources;
        }
        with_ways(&mut want);
        let got: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(got, want, "{kind:?}");
        assert_eq!(osd::read(&out, kind).unwrap().schema(), Some(Version::V0_7), "{kind:?}");
    }
}

#[test]
fn the_converted_text_keeps_schema_as_its_first_key() {
    let (kind, text) = examples_06().remove(0);
    let out = osd::convert_to(&text, kind, Version::V0_7).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v.as_object().unwrap().keys().next().map(String::as_str), Some("$schema"));
}

#[test]
fn a_0_5_and_a_0_4_text_reach_0_7_by_the_chain() {
    let fives = [
        (Kind::Device, "examples/0.5/synth.device.json"),
        (Kind::Profile, "examples/0.5/acme.profile.json"),
        (Kind::Sound, "examples/0.5/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.5/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.5/tribe-0.3-beta.firmware.json"),
    ];
    for (kind, path) in fives {
        let via_06 = osd::convert_to(&file(path), kind, Version::V0_6).unwrap();
        let direct = osd::convert_to(&file(path), kind, Version::V0_7).unwrap_or_else(|f| panic!("{path}: {f}"));
        let mut want: Value = serde_json::from_str(&via_06).unwrap();
        want["$schema"] = json!(address("0.7", name(kind)));
        if let Some(sources) = want.as_object_mut().and_then(|o| o.remove("_sources")) {
            want["sources"] = sources;
        }
        // 0.7 removes these fields; the 0.6 text keeps them.
        with_ways(&mut want);
        if let Some(o) = want.get_mut("device").and_then(Value::as_object_mut) {
            o.remove("revision");
        }
        // 0.7 says the maker in `identity.manufacturerId`, unless a `midi_of` file without identity keeps a note.
        if want.get("midi_of").is_none() || want.get("identity").is_some() {
            let moved = want.get_mut("device").and_then(Value::as_object_mut).and_then(|o| o.remove("manufacturer_id"));
            if let Some(m) = moved {
                if want.get("identity").is_none() {
                    want["identity"] = json!({});
                }
                want["identity"]["manufacturerId"] = m;
            }
        } else if let Some(o) = want.get_mut("device").and_then(Value::as_object_mut) {
            if let Some(m) = o.remove("manufacturer_id") {
                o.insert("_manufacturer_id".into(), m);
            }
        }
        if let Some(o) = want.get_mut("sysex").and_then(Value::as_object_mut) {
            o.remove("footer");
        }
        if let Some(o) = want.get_mut("midi_out").and_then(Value::as_object_mut) {
            o.remove("note_offset");
        }
        assert_eq!(serde_json::from_str::<Value>(&direct).unwrap(), want, "{path}");
    }
    let fours = [
        (Kind::Device, "examples/0.4/test_synth.json"),
        (Kind::Sound, "examples/0.4/warm-pad.sound.json"),
        (Kind::Fingerprint, "examples/0.4/minilogue.fingerprint.json"),
        (Kind::Firmware, "examples/0.4/tribe-0.3-beta.firmware.json"),
    ];
    for (kind, path) in fours {
        let out = osd::convert_to(&file(path), kind, Version::V0_7).unwrap_or_else(|f| panic!("{path}: {f}"));
        assert_eq!(schema_of(&out), address("0.7", name(kind)), "{path}");
        assert_eq!(osd::read(&out, kind).unwrap().schema(), Some(Version::V0_7), "{path}");
    }
}

#[test]
fn a_text_already_in_0_7_comes_back_as_it_is_and_converts_to_itself() {
    for (kind, text) in examples_06() {
        let out = osd::convert_to(&as_07(&text, kind), kind, Version::V0_7).unwrap();
        let again = osd::convert_to(&out, kind, Version::V0_7).unwrap();
        assert_eq!(again, out, "{kind:?}");
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), serde_json::from_str::<Value>(&as_07(&text, kind)).unwrap());
    }
}

#[test]
fn conversion_goes_forward_only_so_a_0_7_text_is_not_carried_back_to_0_6() {
    let (kind, text) = examples_06().remove(0);
    let f = osd::convert_to(&as_07(&text, kind), kind, Version::V0_6).expect_err("backward");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/$schema"));
}

#[test]
fn convert_still_writes_the_published_version_0_6() {
    let text = file("examples/0.5/synth.device.json");
    let out = osd::convert(&text, Kind::Device).unwrap();
    assert_eq!(schema_of(&out), address("0.6", "device"));
}

fn transfers_device(version: &str, control: &str) -> Value {
    json!({
        "$schema": address(version, "device"),
        "device": {"name": "T", "osc_prefix": "/t"},
        "sound_transfers": [{
            "id": "single", "sound_id": "program", "carries": "sound",
            "match_frame": [240, 66, "any", 0, 1, 81, 64, "any", "any", "any", "any", "any", "any", "any", "any", "any", 247],
            "elements": {"data": {"from": 11, "length": 4}, "name": {"from": 7, "length": 4}},
            "places": [{"control": control, "from": 0, "length": 1}]
        }]
    })
}

fn fault_of(doc: &Value) -> (FaultKind, String) {
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("refused");
    (f.kind, f.pointer)
}

#[test]
fn a_0_7_device_whose_sound_transfers_name_a_control_outside_the_pattern_is_refused_like_a_0_6_one() {
    for bad in ["/nowhere", "/cc_params/entries/x", "cc_params/entries/0"] {
        let six = fault_of(&transfers_device("0.6", bad));
        assert_eq!(six, (FaultKind::BadValue, "/sound_transfers/0/places/0/control".to_string()), "{bad}");
        assert_eq!(fault_of(&transfers_device("0.7", bad)), six, "{bad}");
    }
}

#[test]
fn a_0_7_device_whose_sound_transfers_name_an_absent_entry_is_refused_like_a_0_6_one() {
    let six = fault_of(&transfers_device("0.6", "/cc_params/entries/4"));
    assert_eq!(six, (FaultKind::BadValue, "/sound_transfers/0/places/0/control".to_string()));
    assert_eq!(fault_of(&transfers_device("0.7", "/cc_params/entries/4")), six);
}

#[test]
fn a_0_7_device_with_sound_transfers_naming_an_existing_control_is_accepted() {
    let mut doc = transfers_device("0.7", "/cc_params/entries/0");
    doc["cc_params"] = json!({"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]});
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"));
}

#[test]
fn a_0_7_sound_transfers_control_is_judged_after_the_merge_of_a_midi_of_resolution() {
    let catalogue = |a: Value, b: Value| -> BTreeMap<String, String> {
        [("acme/a".to_string(), a.to_string()), ("acme/b".to_string(), b.to_string())].into()
    };
    let link = json!({"$schema": address("0.7", "device"), "device": {"name": "T", "osc_prefix": "/t"}, "midi_of": "acme/b"});
    let mut ok = transfers_device("0.7", "/cc_params/entries/0");
    ok["cc_params"] = json!({"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]});
    osd::resolve("acme/a", &catalogue(link.clone(), ok)).unwrap_or_else(|f| panic!("{f}"));
    let mut absent = transfers_device("0.7", "/cc_params/entries/3");
    absent["cc_params"] = json!({"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]});
    let f = osd::resolve("acme/a", &catalogue(link, absent)).expect_err("absent entry");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/sound_transfers/0/places/0/control"));
}

#[test]
fn a_0_7_assembly_reads_and_a_sound_made_on_an_assembly_of_0_7_is_not_refused() {
    let assembly = file("examples/0.6/tribe-on-fm1.assembly.json");
    let out = as_07(&assembly, Kind::Assembly);
    let d = osd::read(&out, Kind::Assembly).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(d.schema(), Some(Version::V0_7));
}

#[test]
fn an_assembly_naming_0_5_is_still_refused_at_its_schema() {
    let text = json!({"$schema": address("0.5", "assembly"), "stock": {"firmware": "acme/stock", "version": "1.0"}, "modules": []});
    assert!(osd::read(&text.to_string(), Kind::Assembly).is_err());
}

#[test]
fn a_0_7_file_and_a_0_6_target_resolve_into_a_0_7_document_in_either_order() {
    let device = |version: &str, extra: Value| -> Value {
        let mut d = json!({"$schema": address(version, "device"), "device": {"name": "Synth", "osc_prefix": "/synth"}});
        for (k, v) in extra.as_object().unwrap() {
            d[k] = v.clone();
        }
        d
    };
    for (file_v, target_v) in [("0.7", "0.6"), ("0.6", "0.7"), ("0.7", "0.7")] {
        let files: BTreeMap<String, String> = [
            ("acme/a".to_string(), device(file_v, json!({"midi_of": "acme/b"})).to_string()),
            ("acme/b".to_string(), device(target_v, json!({"cc_params": {"entries": [if target_v == "0.7" { json!({"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}) } else { json!({"osc": "/x", "cc": 1}) }]}})).to_string()),
        ]
        .into();
        let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{file_v}/{target_v}: {f}"));
        assert_eq!(r.device.schema, Some(Version::V0_7), "{file_v}/{target_v}");
        let doc: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap();
        assert_eq!(doc["cc_params"]["entries"][0]["osc"], "/x", "{file_v}/{target_v}");
    }
}

#[test]
fn a_0_7_file_still_adds_what_a_0_6_midi_of_file_may_add() {
    let files: BTreeMap<String, String> = [
        ("acme/a".to_string(), json!({"$schema": address("0.7", "device"), "device": {"name": "S", "osc_prefix": "/s"}, "midi_of": "acme/b", "capabilities": []}).to_string()),
        ("acme/b".to_string(), json!({"$schema": address("0.6", "device"), "device": {"name": "S", "osc_prefix": "/s"}}).to_string()),
    ]
    .into();
    osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
}
