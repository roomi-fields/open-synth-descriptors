// @lit spec/0.7.md spec/vocabulary/** schemas/0.7/**
//! The changes of form of 0.7: `orientation` becomes `polarity`, `device.manufacturer_id` becomes
//! `identity.manufacturerId` and the `sysex` form of `_electra` becomes `id` on a command. The reader models
//! the newest form alone; the converter 0.6 to 0.7 carries the old ones.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const S7: &str = "https://w3id.org/osd/0.7/device.schema.json";
const S6: &str = "https://w3id.org/osd/0.6/device.schema.json";
const P7: &str = "https://w3id.org/osd/0.7/profile.schema.json";

fn doc(schema: &str, extra: Value) -> Value {
    let mut d = json!({"$schema": schema, "device": {"name": "Synth", "osc_prefix": "/synth"}});
    for (k, v) in extra.as_object().unwrap() {
        d[k] = v.clone();
    }
    d
}

fn cc_way(n: u8) -> Value {
    json!([{"ctrlType": "cc", "ctrlIndex": [n]}])
}

fn control_07(extra: Value) -> Value {
    let mut e = json!({"osc": "/pan", "ways": cc_way(10)});
    for (k, v) in extra.as_object().unwrap() {
        e[k] = v.clone();
    }
    doc(S7, json!({"cc_params": {"entries": [e]}}))
}

fn read_device(d: &Value) -> osd::Device {
    match osd::read(&d.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{d}: {f}")) {
        osd::Descriptor::Device(d) => *d,
        _ => panic!("not a device"),
    }
}

fn written(d: &Value) -> Value {
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(read_device(d))))).unwrap()
}

fn refused(d: &Value) -> (FaultKind, String) {
    let f = osd::read(&d.to_string(), Kind::Device).expect_err("refused");
    (f.kind, f.pointer)
}

fn converted(d: &Value) -> Value {
    let out = osd::convert_to(&d.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{d}: {f}"));
    serde_json::from_str(&out).unwrap()
}

fn not_convertible(d: &Value) -> String {
    let f = osd::convert_to(&d.to_string(), Kind::Device, Version::V0_7).expect_err("refused");
    assert_eq!(f.kind, FaultKind::NotConvertible, "{f}");
    f.pointer
}

fn schema_errors(d: &Value) -> Vec<(String, String)> {
    common::Schemas::load("0.7").errors(Kind::Device, d)
}

// ---- polarity

#[test]
fn a_0_7_control_reads_bipolar_and_unipolar_and_the_reader_names_them_in_its_type() {
    for (word, want) in [("bipolar", osd::Polarity::Bipolar), ("unipolar", osd::Polarity::Unipolar)] {
        let d = read_device(&control_07(json!({"polarity": word})));
        assert_eq!(d.cc_params.unwrap().entries[0].polarity, Some(want), "{word}");
    }
    let none = read_device(&control_07(json!({})));
    assert_eq!(none.cc_params.unwrap().entries[0].polarity, None);
}

#[test]
fn the_reader_writes_polarity_back_and_never_orientation() {
    for word in ["bipolar", "unipolar"] {
        let back = written(&control_07(json!({"polarity": word})));
        let e = &back["cc_params"]["entries"][0];
        assert_eq!(e["polarity"], word);
        assert!(e.get("orientation").is_none(), "{e}");
    }
    let back = written(&control_07(json!({})));
    assert!(back["cc_params"]["entries"][0].get("polarity").is_none());
}

#[test]
fn a_0_7_control_with_orientation_is_refused_as_an_unknown_field() {
    for word in ["centered", "0-based", "bipolar"] {
        assert_eq!(refused(&control_07(json!({"orientation": word}))), (FaultKind::UnknownField, "/cc_params/entries/0/orientation".to_string()), "{word}");
    }
}

#[test]
fn a_polarity_outside_the_two_words_is_refused_as_a_bad_value_at_the_field() {
    for bad in [json!("centered"), json!("0-based"), json!("Bipolar"), json!(1), json!(null)] {
        assert_eq!(refused(&control_07(json!({"polarity": bad}))), (FaultKind::BadValue, "/cc_params/entries/0/polarity".to_string()), "{bad}");
    }
}

#[test]
fn the_converter_turns_centered_into_bipolar_and_0_based_into_unipolar() {
    let d = doc(S6, json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1, "orientation": "centered"},
        {"osc": "/b", "cc": 2, "orientation": "0-based"},
        {"osc": "/c", "cc": 3},
        {"osc": "/d", "cc": 4, "cc_lsb": 36, "range": [0, 16383], "orientation": "centered"},
    ]}}));
    let out = converted(&d);
    let e = out["cc_params"]["entries"].as_array().unwrap();
    assert_eq!((e[0]["polarity"].as_str(), e[1]["polarity"].as_str(), e[3]["polarity"].as_str()), (Some("bipolar"), Some("unipolar"), Some("bipolar")));
    assert!(e.iter().all(|x| x.get("orientation").is_none()), "{out}");
    let read = read_device(&out);
    assert_eq!(read.cc_params.unwrap().entries[0].polarity, Some(osd::Polarity::Bipolar));
}

#[test]
fn a_control_without_orientation_gets_no_polarity_and_no_orientation_key() {
    let out = converted(&doc(S6, json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 2, "range": [0, 63]}]}})));
    for e in out["cc_params"]["entries"].as_array().unwrap() {
        assert!(e.get("polarity").is_none() && e.get("orientation").is_none(), "{e}");
    }
}

#[test]
fn the_converter_refuses_another_orientation_word_as_not_convertible_at_the_entry_that_holds_it() {
    for (n, bad) in ["sideways", "Centered", "", "bipolar"].into_iter().enumerate() {
        let mut entries = vec![json!({"osc": "/ok", "cc": 1, "orientation": "centered"}); n];
        entries.push(json!({"osc": "/bad", "cc": 2, "orientation": bad}));
        let d = doc(S6, json!({"cc_params": {"entries": entries}}));
        assert_eq!(not_convertible(&d), format!("/cc_params/entries/{n}/orientation"), "{bad:?}");
    }
}

#[test]
fn a_converted_polarity_converts_to_itself_and_validates() {
    let d = doc(S6, json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "orientation": "centered"}]}}));
    let out = converted(&d);
    assert_eq!(converted(&out), out);
    assert!(schema_errors(&out).is_empty(), "{:?}", schema_errors(&out));
}

#[test]
fn the_0_7_schema_accepts_polarity_and_refuses_orientation_and_another_word() {
    for word in ["bipolar", "unipolar"] {
        assert!(schema_errors(&control_07(json!({"polarity": word}))).is_empty());
    }
    assert!(!schema_errors(&control_07(json!({"orientation": "centered"}))).is_empty());
    assert!(!schema_errors(&control_07(json!({"polarity": "centered"}))).is_empty());
}

// ---- manufacturerId

fn identity_of(d: &Value) -> Value {
    d["identity"].clone()
}

#[test]
fn a_0_6_device_without_midi_of_has_its_manufacturer_id_moved_to_the_identity_with_the_same_bytes() {
    for bytes in [json!([66]), json!([0, 32, 41]), json!([0])] {
        let mut d = doc(S6, json!({}));
        d["device"]["manufacturer_id"] = bytes.clone();
        d["device"]["device_id"] = json!([5]);
        let out = converted(&d);
        assert_eq!(identity_of(&out), json!({"manufacturerId": bytes}), "{out}");
        assert!(out["device"].get("manufacturer_id").is_none() && out["device"].get("_manufacturer_id").is_none(), "{out}");
        assert_eq!(out["device"]["device_id"], json!([5]));
    }
}

#[test]
fn the_manufacturer_id_joins_an_identity_block_the_file_already_has() {
    let mut d = doc(S6, json!({"identity": {"reply": [240, 126, "any", 6, 2], "family": [1, 2]}}));
    d["device"]["manufacturer_id"] = json!([66]);
    let out = converted(&d);
    assert_eq!(identity_of(&out), json!({"reply": [240, 126, "any", 6, 2], "family": [1, 2], "manufacturerId": [66]}));
}

#[test]
fn a_midi_of_file_with_an_identity_block_gets_the_manufacturer_id_in_it() {
    let mut d = doc(S6, json!({"midi_of": "acme/b", "identity": {"reply": [240, 126, "any", 6, 2]}}));
    d["device"]["manufacturer_id"] = json!([66]);
    let out = converted(&d);
    assert_eq!(identity_of(&out), json!({"reply": [240, 126, "any", 6, 2], "manufacturerId": [66]}));
    assert!(out["device"].get("_manufacturer_id").is_none());
}

#[test]
fn a_midi_of_file_without_an_identity_block_gets_none_and_keeps_the_value_as_a_note() {
    let mut d = doc(S6, json!({"midi_of": "acme/b"}));
    d["device"]["manufacturer_id"] = json!([0, 32, 41]);
    d["device"]["device_id"] = json!([1]);
    let out = converted(&d);
    assert!(out.get("identity").is_none(), "{out}");
    assert_eq!(out["device"]["_manufacturer_id"], json!([0, 32, 41]));
    assert!(out["device"].get("manufacturer_id").is_none());
    assert_eq!(out["device"]["device_id"], json!([1]));
    assert_eq!(converted(&out), out, "the note is kept as it is");
    assert_eq!(written(&out)["device"]["_manufacturer_id"], json!([0, 32, 41]), "and written back");
}

#[test]
fn a_device_without_manufacturer_id_gets_none_and_no_identity_block() {
    let mut d = doc(S6, json!({}));
    d["device"]["device_id"] = json!([5]);
    let out = converted(&d);
    assert!(out.get("identity").is_none(), "{out}");
    assert!(out["device"].get("_manufacturer_id").is_none());
    assert_eq!(out["device"]["device_id"], json!([5]));
    let with_identity = converted(&doc(S6, json!({"identity": {"reply": [240, 126, "any", 6, 2]}})));
    assert!(with_identity["identity"].get("manufacturerId").is_none());
}

#[test]
fn a_0_7_device_reads_the_manufacturer_id_in_its_identity_and_refuses_it_in_the_device() {
    let d = doc(S7, json!({"identity": {"manufacturerId": [0, 32, 41]}}));
    assert_eq!(written(&d)["identity"]["manufacturerId"], json!([0, 32, 41]));
    let mut old = doc(S7, json!({}));
    old["device"]["manufacturer_id"] = json!([66]);
    assert_eq!(refused(&old), (FaultKind::UnknownField, "/device/manufacturer_id".to_string()));
}

#[test]
fn a_manufacturer_id_that_is_not_a_list_of_bytes_is_refused_at_the_field() {
    for bad in [json!(66), json!("42"), json!([128]), json!([-1])] {
        let (kind, pointer) = refused(&doc(S7, json!({"identity": {"manufacturerId": bad}})));
        assert_eq!(kind, FaultKind::BadValue, "{bad}");
        assert!(pointer.starts_with("/identity/manufacturerId"), "{bad}: {pointer}");
    }
}

#[test]
fn a_null_manufacturer_id_is_read_and_written_back_as_null() {
    let d = doc(S7, json!({"identity": {"manufacturerId": null}}));
    assert_eq!(written(&d)["identity"]["manufacturerId"], Value::Null);
    assert!(schema_errors(&d).is_empty(), "{:?}", schema_errors(&d));
}

#[test]
fn the_0_7_schema_accepts_the_identity_manufacturer_id_and_refuses_it_in_the_device() {
    assert!(schema_errors(&doc(S7, json!({"identity": {"manufacturerId": [66]}}))).is_empty());
    let mut old = doc(S7, json!({}));
    old["device"]["manufacturer_id"] = json!([66]);
    assert!(!schema_errors(&old).is_empty());
}

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn resolved_identity(files: &BTreeMap<String, String>, id: &str) -> Value {
    let r = osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"));
    let d: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device)))).unwrap();
    d.get("identity").cloned().unwrap_or(Value::Null)
}

fn target() -> Value {
    doc(S7, json!({"identity": {"manufacturerId": [66], "reply": [240, 126, "any", 6, 2, 66]}, "cc_params": {"entries": [{"osc": "/x", "ways": cc_way(1)}]}}))
}

#[test]
fn a_midi_of_file_with_an_identity_block_replaces_the_target_block_manufacturer_id_included() {
    let files = catalogue(vec![("acme/a", doc(S7, json!({"midi_of": "acme/b", "identity": {"reply": [240, 126, "any", 6, 2]}}))), ("acme/b", target())]);
    assert_eq!(resolved_identity(&files, "acme/a"), json!({"reply": [240, 126, "any", 6, 2]}));
    let own = catalogue(vec![("acme/a", doc(S7, json!({"midi_of": "acme/b", "identity": {"manufacturerId": [67]}}))), ("acme/b", target())]);
    assert_eq!(resolved_identity(&own, "acme/a"), json!({"manufacturerId": [67]}));
}

#[test]
fn a_midi_of_file_without_an_identity_block_takes_the_target_identity_manufacturer_id_with_it() {
    let files = catalogue(vec![("acme/a", doc(S7, json!({"midi_of": "acme/b"}))), ("acme/b", target())]);
    assert_eq!(resolved_identity(&files, "acme/a"), json!({"manufacturerId": [66], "reply": [240, 126, "any", 6, 2, 66]}));
}

#[test]
fn a_0_6_midi_of_file_that_gave_no_identity_block_leaves_the_target_identity_whole() {
    let mut a = doc(S6, json!({"midi_of": "acme/b"}));
    a["device"]["manufacturer_id"] = json!([67]);
    let files = catalogue(vec![("acme/a", a), ("acme/b", target())]);
    assert_eq!(resolved_identity(&files, "acme/a"), json!({"manufacturerId": [66], "reply": [240, 126, "any", 6, 2, 66]}));
}

#[test]
fn a_0_6_midi_of_file_with_an_identity_block_and_a_manufacturer_id_gives_its_own_block() {
    let mut a = doc(S6, json!({"midi_of": "acme/b", "identity": {"reply": [240, 126, "any", 6, 2]}}));
    a["device"]["manufacturer_id"] = json!([67]);
    let files = catalogue(vec![("acme/a", a), ("acme/b", target())]);
    assert_eq!(resolved_identity(&files, "acme/a"), json!({"manufacturerId": [67], "reply": [240, 126, "any", 6, 2]}));
}

fn profile_files(profile_identity: Value, driver_identity: Value) -> BTreeMap<String, String> {
    catalogue(vec![
        ("acme/_profile", json!({"$schema": P7, "identity": profile_identity})),
        ("acme/synth", doc(S7, json!({"identity": driver_identity}))),
    ])
}

#[test]
fn a_null_manufacturer_id_drops_the_profile_one_from_the_resolved_description() {
    let files = profile_files(json!({"request": [240, 126, 127, 6, 1, 247], "manufacturerId": [66]}), json!({"profile": "acme", "manufacturerId": null}));
    assert_eq!(resolved_identity(&files, "acme/synth"), json!({"request": [240, 126, 127, 6, 1, 247], "profile": "acme"}));
}

#[test]
fn a_null_manufacturer_id_on_a_profile_that_does_not_give_it_is_accepted_and_leaves_it_absent() {
    let files = profile_files(json!({"request": [240, 126, 127, 6, 1, 247]}), json!({"profile": "acme", "manufacturerId": null}));
    assert_eq!(resolved_identity(&files, "acme/synth"), json!({"request": [240, 126, 127, 6, 1, 247], "profile": "acme"}));
}

#[test]
fn a_profile_manufacturer_id_reaches_a_block_that_does_not_say_it() {
    let files = profile_files(json!({"manufacturerId": [66]}), json!({"profile": "acme"}));
    assert_eq!(resolved_identity(&files, "acme/synth"), json!({"manufacturerId": [66], "profile": "acme"}));
}

// ---- id on a command

fn commands_06(list: Vec<Value>) -> Value {
    doc(S6, json!({"commands": list}))
}

fn command(osc: &str, electra: Value) -> Value {
    json!({"osc": osc, "_electra": electra, "frame": [240, 125, 247]})
}

fn sysex(number: Value) -> Value {
    json!({"parameter": {"type": "sysex", "number": number}})
}

#[test]
fn a_sysex_electra_with_an_integer_number_becomes_the_id_of_the_command_to_the_byte() {
    for n in [0i64, 1, 127, 128, 405, 70000] {
        let out = converted(&commands_06(vec![command("/p", sysex(json!(n)))]));
        let c = &out["commands"][0];
        assert_eq!(c["id"], json!(n), "{c}");
        assert!(c.get("_electra").is_none(), "{c}");
        assert_eq!((c["osc"].as_str(), &c["frame"]), (Some("/p"), &json!([240, 125, 247])));
    }
}

#[test]
fn two_commands_with_the_same_sysex_number_both_keep_it() {
    let out = converted(&commands_06(vec![command("/a", sysex(json!(1))), command("/b", sysex(json!(1))), command("/c", sysex(json!(2)))]));
    let ids: Vec<&Value> = out["commands"].as_array().unwrap().iter().map(|c| &c["id"]).collect();
    assert_eq!(ids, [&json!(1), &json!(1), &json!(2)]);
}

#[test]
fn a_virtual_electra_stays_a_note_with_its_source_and_gets_no_id() {
    let note = json!({"parameter": {"type": "virtual", "number": 405}});
    let out = converted(&commands_06(vec![command("/dev_id", note.clone())]));
    let c = &out["commands"][0];
    assert_eq!(c["_electra"], note);
    assert!(c.get("id").is_none() && c.get("virtual").is_none(), "{c}");
    assert_eq!(written(&out)["commands"][0]["_electra"], note, "kept and written back");
}

#[test]
fn an_electra_of_another_shape_stays_a_note_with_no_id_and_the_command_converts() {
    let shapes = [
        json!({"parameter": {"type": "cc7", "number": 3}}),
        json!({"parameter": {"type": "sysex", "number": "3"}}),
        json!({"parameter": {"type": "sysex", "number": 1.5}}),
        json!({"parameter": {"type": "sysex"}}),
        json!({"parameter": {"number": 3}}),
        json!({"preset": {"id": 9}}),
    ];
    for shape in shapes {
        let out = converted(&commands_06(vec![command("/x", shape.clone())]));
        let c = &out["commands"][0];
        assert_eq!(c["_electra"], shape);
        assert!(c.get("id").is_none(), "{c}");
    }
}

#[test]
fn a_command_of_0_7_reads_its_id_as_an_integer_and_refuses_another_value() {
    let ok = doc(S7, json!({"commands": [{"osc": "/a", "id": 405, "frame": [240, 125, 247]}, {"osc": "/b", "id": 405, "frame": [240, 125, 247]}]}));
    assert_eq!(written(&ok)["commands"][1]["id"], 405);
    assert!(schema_errors(&ok).is_empty(), "{:?}", schema_errors(&ok));
    for bad in [json!("405"), json!(1.5), json!(null), json!(true)] {
        let d = doc(S7, json!({"commands": [{"osc": "/a", "id": bad, "frame": [240, 125, 247]}]}));
        assert_eq!(refused(&d), (FaultKind::BadValue, "/commands/0/id".to_string()), "{bad}");
        assert!(!schema_errors(&d).is_empty(), "{bad}");
    }
}

#[test]
fn a_converted_id_converts_to_itself() {
    let out = converted(&commands_06(vec![command("/a", sysex(json!(7))), command("/b", json!({"parameter": {"type": "virtual", "number": 1}}))]));
    assert_eq!(converted(&out), out);
    assert!(schema_errors(&out).is_empty(), "{:?}", schema_errors(&out));
}

// ---- the text of 0.7

fn text(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn the_text_of_0_7_names_the_three_forms_and_no_longer_the_old_fields_as_fields() {
    let spec = text("spec/0.7.md");
    for word in ["polarity", "identity.manufacturerId", "unipolar", "bipolar"] {
        assert!(spec.contains(word), "spec/0.7.md does not say {word}");
    }
    for old in ["| `orientation` |", "| `device.manufacturer_id` |"] {
        assert!(!spec.contains(old), "spec/0.7.md still has a row {old}");
    }
    let schema = text("schemas/0.7/device.schema.json");
    assert!(schema.contains("\"polarity\"") && schema.contains("\"manufacturerId\""));
    assert!(!schema.contains("\"orientation\": {") && !schema.contains("\"manufacturer_id\""));
}
