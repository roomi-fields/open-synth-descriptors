//! A control or a `params` entry with no message is kept, to be completed (`spec/0.7.md`): in 0.7
//! by an empty `ways`, its `range`, `transmit`, `recognize` and `nrpn_lsb_range` on the entry;
//! in 0.4 to 0.6 by the same flat fields, written back as given and carried to 0.7 unchanged.

// @lit spec/0.7.md schemas/0.4/** schemas/0.5/** schemas/0.6/** schemas/0.7/**

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const FLAT_VERSIONS: [Version; 3] = [Version::V0_4, Version::V0_5, Version::V0_6];

fn address(version: Version) -> String {
    format!("https://w3id.org/osd/{}/device.schema.json", version.name())
}

/// A device of `version` holding the blocks of `body`; a 0.4 text states no `$schema`.
fn device(version: Version, mut body: Value) -> Value {
    body["device"] = json!({"name": "T", "osc_prefix": "/t"});
    if version != Version::V0_4 {
        body["$schema"] = json!(address(version));
    }
    body
}

fn cc_device(version: Version, entries: Vec<Value>) -> Value {
    device(version, json!({"cc_params": {"channel": 0, "entries": entries}}))
}

fn params_device(version: Version, entries: Vec<Value>) -> Value {
    let (get, set) = match version {
        Version::V0_4 => (json!([240, "{p}", 247]), json!([240, "{p}", "{value}", 247])),
        _ => (json!([240, {"arg": "p"}, 247]), json!([240, {"arg": "p"}, {"arg": "value"}, 247])),
    };
    device(version, json!({"params": {"get_frame": get, "set_frame": set, "value_type": "u7", "entries": entries}}))
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn written(doc: &Value) -> Value {
    let d = read(doc).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    serde_json::from_str(&osd::write(&d)).unwrap()
}

#[track_caller]
fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = match read(doc) {
        Err(f) => f,
        Ok(_) => panic!("read, but should be refused: {doc}"),
    };
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}\n{doc}");
}

fn with(mut base: Value, key: &str, value: Value) -> Value {
    base[key] = value;
    base
}

/// Each field an entry without a way may carry, with a value of it.
fn cc_fields() -> [(&'static str, Value); 4] {
    [("range", json!([0, 100])), ("transmit", json!("absolute")), ("recognize", json!("none")), ("nrpn_lsb_range", json!([0, 15]))]
}

fn params_fields() -> [(&'static str, Value); 3] {
    [("range", json!([-64, 63])), ("transmit", json!("relative")), ("recognize", json!("both"))]
}

fn all_of(osc: &str, fields: &[(&str, Value)]) -> Value {
    fields.iter().fold(json!({"osc": osc}), |e, (k, v)| with(e, k, v.clone()))
}

// ---- 0.7: an empty `ways`

#[test]
fn a_0_7_control_with_an_empty_ways_reads_and_writes_each_of_its_fields_back_as_written() {
    let full = with(with(all_of("/full", &cc_fields()), "ways", json!([])), "title", json!("Bank"));
    let full = with(with(full, "scale", json!([0, 10])), "polarity", json!("unipolar"));
    let mut entries = vec![json!({"osc": "/bare", "ways": []}), full];
    entries.extend(cc_fields().into_iter().map(|(k, v)| with(json!({"osc": format!("/only_{k}"), "ways": []}), k, v)));
    let doc = cc_device(Version::V0_7, entries);
    assert_eq!(written(&doc), doc);
}

#[test]
fn a_0_7_params_entry_with_an_empty_ways_reads_and_writes_each_of_its_fields_back_as_written() {
    let full = with(with(all_of("/full", &params_fields()), "ways", json!([])), "title", json!("Shape"));
    let mut entries = vec![json!({"osc": "/bare", "ways": []}), full];
    entries.extend(params_fields().into_iter().map(|(k, v)| with(json!({"osc": format!("/only_{k}"), "ways": []}), k, v)));
    let doc = params_device(Version::V0_7, entries);
    assert_eq!(written(&doc), doc);
}

#[test]
fn a_0_7_entry_without_a_way_claims_no_message() {
    let entry = |osc: &str| json!({"osc": osc, "ways": [], "range": [0, 127], "nrpn_lsb_range": [0, 15]});
    let cc = json!({"osc": "/c", "ways": [{"ctrlType": "cc", "ctrlIndex": [0]}]});
    let nrpn = json!({"osc": "/n", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [0, 0], "value_bits": 14}]});
    let doc = cc_device(Version::V0_7, vec![entry("/a"), entry("/b"), cc, nrpn]);
    assert_eq!(written(&doc), doc);
}

#[test]
fn a_0_7_entry_without_the_key_ways_is_still_refused_as_missing() {
    refused(&cc_device(Version::V0_7, vec![json!({"osc": "/a"})]), FaultKind::Missing, "/cc_params/entries/0/ways");
    refused(&params_device(Version::V0_7, vec![json!({"osc": "/a"})]), FaultKind::Missing, "/params/entries/0/ways");
}

#[test]
fn a_0_7_control_with_a_way_refuses_the_fields_of_a_control_without_one_as_unknown_at_the_field() {
    let entry = json!({"osc": "/a", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14}]});
    for (key, value) in cc_fields() {
        let doc = cc_device(Version::V0_7, vec![json!({"osc": "/first", "ways": []}), with(entry.clone(), key, value)]);
        refused(&doc, FaultKind::UnknownField, &format!("/cc_params/entries/1/{key}"));
    }
}

#[test]
fn a_0_7_params_entry_with_a_way_refuses_range_transmit_and_recognize_as_unknown_at_the_field() {
    let entry = json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "ctrlIndex": [1]}]});
    for (key, value) in params_fields() {
        let doc = params_device(Version::V0_7, vec![with(entry.clone(), key, value)]);
        refused(&doc, FaultKind::UnknownField, &format!("/params/entries/0/{key}"));
    }
}

#[test]
fn a_0_7_params_entry_never_carries_a_block_of_nrpn_numbers() {
    let doc = params_device(Version::V0_7, vec![json!({"osc": "/a", "ways": [], "nrpn_lsb_range": [0, 15]})]);
    refused(&doc, FaultKind::UnknownField, "/params/entries/0/nrpn_lsb_range");
}

#[test]
fn a_0_7_entry_without_a_way_still_refuses_the_flat_numbers_of_a_message() {
    for (key, value) in [("cc", json!(1)), ("nrpn_msb", json!(1)), ("value_bits", json!(14)), ("also", json!([]))] {
        let doc = cc_device(Version::V0_7, vec![with(json!({"osc": "/a", "ways": []}), key, value)]);
        refused(&doc, FaultKind::UnknownField, &format!("/cc_params/entries/0/{key}"));
    }
    let doc = params_device(Version::V0_7, vec![json!({"osc": "/a", "ways": [], "pr": 1})]);
    refused(&doc, FaultKind::UnknownField, "/params/entries/0/pr");
}

// ---- 0.7: the reference scale of an entry without a way

/// A `cc_params` and a `params` device holding one entry without a way, with `more` on it, and the
/// pointer of that entry.
fn without_a_way(more: Value) -> [(Value, &'static str); 2] {
    let mut entry = json!({"osc": "/a", "ways": []});
    for (k, v) in more.as_object().unwrap() {
        entry[k.as_str()] = v.clone();
    }
    [
        (cc_device(Version::V0_7, vec![entry.clone()]), "/cc_params/entries/0"),
        (params_device(Version::V0_7, vec![entry]), "/params/entries/0"),
    ]
}

/// The three things written on the reference scale, each at a value, with the pointer of the value.
fn on_the_scale(value: i64) -> [(Value, &'static str); 3] {
    [
        (json!({"default": value}), "default"),
        (json!({"values": [{"value": value, "title": "x"}]}), "values/0/value"),
        (json!({"readings": [{"state": "valid", "firmware": "unknown", "range": [value, value]}]}), "readings/0/range"),
    ]
}

fn joined(mut a: Value, b: &Value) -> Value {
    for (k, v) in b.as_object().unwrap() {
        a[k.as_str()] = v.clone();
    }
    a
}

/// `default`, a named value and a reading at `inside` are read on an entry holding `scales`, and
/// at `outside` are refused.
#[track_caller]
fn assert_the_scale(scales: Value, inside: &[i64], outside: &[i64]) {
    for value in inside {
        for (field, _) in on_the_scale(*value) {
            for (doc, _) in without_a_way(joined(scales.clone(), &field)) {
                assert_eq!(written(&doc), doc);
            }
        }
    }
    for value in outside {
        for (field, at) in on_the_scale(*value) {
            for (doc, entry) in without_a_way(joined(scales.clone(), &field)) {
                refused(&doc, FaultKind::BadValue, &format!("{entry}/{at}"));
            }
        }
    }
}

#[test]
fn the_range_of_an_entry_without_a_way_is_its_reference_scale_when_it_gives_no_scale() {
    assert_the_scale(json!({"range": [-10, 200]}), &[-10, 0, 200], &[-11, 201]);
}

#[test]
fn the_scale_of_an_entry_without_a_way_is_its_reference_scale_before_its_range() {
    assert_the_scale(json!({"scale": [0, 1000], "range": [0, 10]}), &[0, 500, 1000], &[-1, 1001]);
    assert_the_scale(json!({"scale": [0, 10], "range": [0, 1000]}), &[10], &[11, 500]);
    assert_the_scale(json!({"scale": [-5, 5]}), &[-5, 5], &[-6, 6]);
}

#[test]
fn an_entry_without_a_way_scale_or_range_compares_its_values_to_nothing() {
    assert_the_scale(json!({}), &[-100000, 0, 128, 100000], &[]);
}

// ---- the schema of 0.7

#[test]
fn the_0_7_schema_accepts_an_empty_ways_with_the_fields_of_the_entry() {
    let schemas = common::Schemas::load("0.7");
    let cc = cc_device(Version::V0_7, vec![json!({"osc": "/bare", "ways": []}), with(all_of("/a", &cc_fields()), "ways", json!([]))]);
    let params = params_device(Version::V0_7, vec![json!({"osc": "/bare", "ways": []}), with(all_of("/a", &params_fields()), "ways", json!([]))]);
    assert_eq!(schemas.check(Kind::Device, &cc), None);
    assert_eq!(schemas.check(Kind::Device, &params), None);
}

#[test]
fn the_0_7_schema_refuses_the_fields_of_an_entry_without_a_way_beside_a_way_and_an_absent_ways() {
    let schemas = common::Schemas::load("0.7");
    let valid = |doc: &Value| schemas.check(Kind::Device, doc).is_none();
    let cc = json!({"osc": "/a", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]});
    let sysex = json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "ctrlIndex": [1]}]});
    // The entry with a way validates, and so does the one without: each refusal is of the field alone.
    assert!(valid(&cc_device(Version::V0_7, vec![cc.clone()])));
    assert!(valid(&cc_device(Version::V0_7, vec![json!({"osc": "/a", "ways": []})])));
    assert!(valid(&params_device(Version::V0_7, vec![json!({"osc": "/a", "ways": []})])));
    for (key, value) in cc_fields() {
        assert!(!valid(&cc_device(Version::V0_7, vec![with(cc.clone(), key, value.clone())])), "cc_params {key} beside a way");
        assert!(!valid(&cc_device(Version::V0_7, vec![with(json!({"osc": "/a"}), key, value)])), "cc_params {key} without the key ways");
    }
    for (key, value) in params_fields() {
        assert!(!valid(&params_device(Version::V0_7, vec![with(sysex.clone(), key, value)])), "params {key} beside a way");
    }
    assert!(!valid(&params_device(Version::V0_7, vec![json!({"osc": "/a", "ways": [], "nrpn_lsb_range": [0, 1]})])));
    assert!(!valid(&cc_device(Version::V0_7, vec![json!({"osc": "/a"})])));
}

// ---- 0.4 to 0.6: read and written back

/// The fields of `all` that `version` admits on an entry: before 0.6 there is no direction.
fn admitted(version: Version, all: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    all.iter().filter(|(k, _)| version >= Version::V0_6 || !["transmit", "recognize"].contains(k)).cloned().collect()
}

/// Entries without a message: bare, with every field the version admits, and with each alone.
fn flat_entries(version: Version, all: &[(&'static str, Value)]) -> Vec<Value> {
    let fields = admitted(version, all);
    let mut entries = vec![json!({"osc": "/bare"}), all_of("/full", &fields)];
    entries.extend(fields.into_iter().map(|(k, v)| with(json!({"osc": format!("/only_{k}")}), k, v)));
    entries
}

#[test]
fn a_control_without_a_message_before_0_7_is_written_back_with_its_fields_none_lost_or_added() {
    for version in FLAT_VERSIONS {
        let doc = cc_device(version, flat_entries(version, &cc_fields()));
        assert_eq!(written(&doc), doc, "{}", version.name());
    }
}

#[test]
fn a_params_entry_without_a_number_before_0_7_is_written_back_with_its_fields_none_lost_or_added() {
    for version in FLAT_VERSIONS {
        let doc = params_device(version, flat_entries(version, &params_fields()));
        assert_eq!(written(&doc), doc, "{}", version.name());
    }
}

#[test]
fn a_direction_on_an_entry_without_a_message_is_still_no_field_before_0_6() {
    for version in [Version::V0_4, Version::V0_5] {
        for key in ["transmit", "recognize"] {
            let doc = cc_device(version, vec![json!({"osc": "/a", key: "both"})]);
            refused(&doc, FaultKind::UnknownField, &format!("/cc_params/entries/0/{key}"));
            let doc = params_device(version, vec![json!({"osc": "/a", key: "both"})]);
            refused(&doc, FaultKind::UnknownField, &format!("/params/entries/0/{key}"));
        }
    }
}

#[test]
fn a_default_or_a_named_value_outside_the_range_of_an_entry_without_a_message_is_still_refused_in_0_6() {
    let entry = json!({"osc": "/a", "range": [0, 10], "default": 11});
    refused(&cc_device(Version::V0_6, vec![entry.clone()]), FaultKind::BadValue, "/cc_params/entries/0/default");
    refused(&params_device(Version::V0_6, vec![entry]), FaultKind::BadValue, "/params/entries/0/default");
    let entry = json!({"osc": "/a", "range": [0, 10], "values": {"Max": 11}});
    refused(&cc_device(Version::V0_6, vec![entry.clone()]), FaultKind::BadValue, "/cc_params/entries/0/values/Max");
    refused(&params_device(Version::V0_6, vec![entry]), FaultKind::BadValue, "/params/entries/0/values/Max");
}

// ---- the converter to 0.7

/// The 0.7 document of `doc`, checked to read as 0.7, to validate against the 0.7 schema, to be
/// written back as it is and to convert to itself.
#[track_caller]
fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{doc}"));
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["$schema"], address(Version::V0_7), "{doc}");
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &value), None, "{out}");
    assert_eq!(written(&value), value, "{out}");
    assert_eq!(osd::convert_to(&out, Kind::Device, Version::V0_7).as_deref(), Ok(out.as_str()), "converting the result changes it");
    value
}

#[test]
fn a_control_without_a_message_converts_to_an_empty_ways_with_its_fields_unchanged_on_the_entry() {
    for version in FLAT_VERSIONS {
        let entries = flat_entries(version, &cc_fields());
        let want: Vec<Value> = entries.iter().map(|e| with(e.clone(), "ways", json!([]))).collect();
        let out = converted(&cc_device(version, entries));
        assert_eq!(out["cc_params"], json!({"channel": 0, "entries": want}), "{}", version.name());
    }
}

#[test]
fn a_params_entry_without_a_number_converts_to_an_empty_ways_with_its_fields_unchanged_on_the_entry() {
    for version in FLAT_VERSIONS {
        let entries = flat_entries(version, &params_fields());
        let want: Vec<Value> = entries.iter().map(|e| with(e.clone(), "ways", json!([]))).collect();
        let out = converted(&params_device(version, entries));
        assert_eq!(out["params"]["entries"], json!(want), "{}", version.name());
    }
}

#[test]
fn the_other_fields_of_a_control_without_a_message_convert_as_those_of_any_control_and_no_scale_is_written() {
    let entry = json!({
        "osc": "/a", "range": [0, 100], "transmit": "absolute", "recognize": "none", "default": 5, "step_count": 3,
        "values": {"Off": 0, "On": 100}, "orientation": "centered", "channel": 3, "section": "S", "priority": 2,
        "_name": "Bank", "_usage": "which bank", "_description": "the bank", "_notes": "to measure", "_other": 1
    });
    let out = converted(&cc_device(Version::V0_6, vec![entry, json!({"osc": "/b", "cc": 1})]));
    assert_eq!(
        out["cc_params"]["entries"],
        json!([
            {
                "osc": "/a", "range": [0, 100], "transmit": "absolute", "recognize": "none", "default": 5, "step_count": 3,
                "values": [{"value": 0, "title": "Off"}, {"value": 100, "title": "On"}], "polarity": "bipolar", "channel": 3,
                "section": "S", "priority": 2, "title": "Bank", "value_description": "which bank", "description": "the bank",
                "notes": "to measure", "_other": 1, "ways": []
            },
            {"osc": "/b", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}
        ])
    );
}

#[test]
fn a_catalogue_control_that_gives_only_its_orientation_converts_from_0_4_and_0_5() {
    for version in [Version::V0_4, Version::V0_5] {
        let out = converted(&cc_device(version, vec![json!({"osc": "/bank_msb", "orientation": "0-based"})]));
        assert_eq!(out["cc_params"]["entries"], json!([{"osc": "/bank_msb", "polarity": "unipolar", "ways": []}]));
    }
}

#[test]
fn a_control_without_a_message_before_0_6_converts_to_0_6_as_it_is() {
    for version in [Version::V0_4, Version::V0_5] {
        let entries = flat_entries(version, &cc_fields());
        let out = osd::convert_to(&cc_device(version, entries.clone()).to_string(), Kind::Device, Version::V0_6).unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["cc_params"]["entries"], json!(entries), "{}", version.name());
    }
}

// ---- resolution

fn resolved(files: &[(&str, &Value)]) -> Value {
    let files: BTreeMap<String, String> = files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect();
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap()
}

/// A driver of `version` that shares the MIDI of `acme/b`.
fn variant(version: Version) -> Value {
    device(version, json!({"midi_of": "acme/b"}))
}

#[test]
fn a_midi_of_of_one_version_returns_the_fields_of_an_entry_without_a_message_as_they_are() {
    for version in FLAT_VERSIONS {
        let entries = flat_entries(version, &cc_fields());
        let out = resolved(&[("acme/a", &variant(version)), ("acme/b", &cc_device(version, entries.clone()))]);
        assert_eq!(out["cc_params"]["entries"], json!(entries), "{}", version.name());
    }
    let entries = flat_entries(Version::V0_6, &params_fields());
    let out = resolved(&[("acme/a", &variant(Version::V0_6)), ("acme/b", &params_device(Version::V0_6, entries.clone()))]);
    assert_eq!(out["params"]["entries"], json!(entries));
}

#[test]
fn a_midi_of_that_joins_two_versions_resolves_a_target_with_an_entry_without_a_message() {
    for target in FLAT_VERSIONS {
        let entries = flat_entries(target, &cc_fields());
        let want: Vec<Value> = entries.iter().map(|e| with(e.clone(), "ways", json!([]))).collect();
        let out = resolved(&[("acme/a", &variant(Version::V0_7)), ("acme/b", &cc_device(target, entries))]);
        assert_eq!(out["cc_params"]["entries"], json!(want), "{} under 0.7", target.name());
    }
    let entries = flat_entries(Version::V0_5, &cc_fields());
    let out = resolved(&[("acme/a", &variant(Version::V0_6)), ("acme/b", &cc_device(Version::V0_5, entries.clone()))]);
    assert_eq!(out["cc_params"]["entries"], json!(entries), "0.5 under 0.6");
}

#[test]
fn a_0_6_file_with_an_entry_without_a_message_resolves_on_a_0_7_target() {
    let mut own = variant(Version::V0_6);
    own["cc_params"] = json!({"entries": [{"osc": "/own", "range": [0, 3], "transmit": "both"}]});
    let target = cc_device(Version::V0_7, vec![json!({"osc": "/there", "ways": [], "range": [1, 2]})]);
    let out = resolved(&[("acme/a", &own), ("acme/b", &target)]);
    let entries = out["cc_params"]["entries"].as_array().unwrap();
    for want in [json!({"osc": "/own", "ways": [], "range": [0, 3], "transmit": "both"}), json!({"osc": "/there", "ways": [], "range": [1, 2]})] {
        assert!(entries.contains(&want), "{want} is not in {entries:?}");
    }
    assert_eq!(entries.len(), 2);
}

// ---- the text of 0.7

#[test]
fn the_text_of_0_7_says_the_empty_ways_and_names_the_cases_that_prove_it() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    assert!(spec.contains("`ways: []`"), "spec/0.7.md does not say `ways: []`");
    for case in [
        "0.7/read/control-without-a-way-warned",
        "0.7/read/params-entry-without-a-way-warned",
        "0.7/read/transmit-beside-a-way-refused",
        "0.7/convert/entries-without-a-message-keep-their-fields",
    ] {
        assert!(spec.contains(&format!("`{case}`")), "spec/0.7.md does not name the case {case}");
    }
}
