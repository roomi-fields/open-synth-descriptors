//! The numbers that address a parameter in the maker's frame, each under its name: `pr`, `p`,
//! `c`, `r`, each a byte, and `id`, any integer. An entry of `params` of 0.4 to 0.6 gives them
//! (`spec/0.6.md`, table of words); in 0.7 each is a field of a `sysex` way, which has no
//! `ctrlIndex` (`spec/0.7.md` 5.3.3). No name is lost, and nothing a published version admits is
//! refused.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version, WarningKind, Way};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const OLDER: [(&str, Version); 3] = [("0.4", Version::V0_4), ("0.5", Version::V0_5), ("0.6", Version::V0_6)];
const NAMES: [&str; 5] = ["pr", "p", "c", "r", "id"];
const BYTES: [&str; 4] = ["pr", "p", "c", "r"];

/// A device of `version` whose `params` table holds `entries`.
fn doc(version: &str, entries: Vec<Value>) -> Value {
    let mut d = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": entries}
    });
    // A text of 0.4 declares no schema.
    if version != "0.4" {
        d["$schema"] = format!("https://w3id.org/osd/{version}/device.schema.json").into();
    }
    d
}

fn read(d: &Value) -> Descriptor {
    osd::read(&d.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{d}"))
}

/// The ways of each entry of `params`, as the reader gives them.
fn ways(d: &Value) -> Vec<Vec<Way>> {
    let Descriptor::Device(device) = read(d) else { panic!("a device") };
    device.params.unwrap().entries.iter().map(|e| e.ways()).collect()
}

/// `pr`, `p`, `c`, `r` and `id` of a way, which has no `ctrlIndex`.
fn numbers(w: &Way) -> [Option<i64>; 5] {
    assert_eq!((w.ctrl_type.as_str(), &w.ctrl_index), ("sysex", &None));
    [w.pr, w.p, w.c, w.r, w.id]
}

/// What `numbers` gives for a way or an older entry that gives the fields of `given`.
fn named(given: &Value) -> [Option<i64>; 5] {
    NAMES.map(|k| given.get(k).map(|n| n.as_i64().unwrap()))
}

/// A `sysex` way that gives the fields of `given`.
fn sysex(given: &Value) -> Value {
    let mut way = given.clone();
    way["ctrlType"] = "sysex".into();
    way
}

/// The numbers of a frame an entry may give.
fn sets_of_numbers() -> Vec<Value> {
    vec![
        json!({"pr": 16, "p": 2, "c": 0, "r": 0}),
        json!({"p": 2, "c": 3, "r": 4}),
        json!({"pr": 16}),
        json!({"p": 2}),
        json!({"c": 3, "r": 4}),
        json!({"pr": 0, "p": 255, "c": 0, "r": 255}),
        json!({"id": 0}),
        json!({"id": 300}),
        json!({"id": 4_294_967_296_i64}),
        json!({"id": -1}),
        json!({"id": -70000}),
        json!({"pr": 16, "id": 300}),
        json!({"pr": 16, "id": -1}),
        json!({"r": 4, "id": 5}),
        json!({"pr": 1, "p": 2, "c": 3, "r": 4, "id": 5}),
    ]
}

fn with(mut object: Value, key: &str, value: Value) -> Value {
    object[key] = value;
    object
}

/// A 0.7 device whose one `params` entry has this one way.
fn one_way(way: Value) -> Value {
    doc("0.7", vec![json!({"osc": "/a", "ways": [way]})])
}

/// A 0.7 device whose one control of `cc_params` has this one way.
fn one_control_way(way: Value) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"entries": [{"osc": "/a", "ways": [way]}]}
    })
}

const PARAM_WAY: &str = "/params/entries/0/ways/0";
const CONTROL_WAY: &str = "/cc_params/entries/0/ways/0";


fn written(d: &Value) -> Value {
    serde_json::from_str(&osd::write(&read(d))).unwrap()
}

/// The text of `d` converted to `to`, which reads as `to` and converts to itself.
fn converted(d: &Value, to: Version) -> Value {
    let out = osd::convert_to(&d.to_string(), Kind::Device, to).unwrap_or_else(|f| panic!("{f}\n{d}"));
    osd::read_as(&out, Kind::Device, to).unwrap_or_else(|f| panic!("{f} in {out}"));
    assert_eq!(osd::convert_to(&out, Kind::Device, to).as_deref(), Ok(out.as_str()), "converting the result changes it");
    serde_json::from_str(&out).unwrap()
}

/// The entries of `params` of `d` converted to 0.7.
fn to_0_7(d: &Value) -> Value {
    let out = converted(d, Version::V0_7);
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &out), None, "{out}");
    out["params"]["entries"].clone()
}

/// The reader refuses `d` with this kind at this pointer.
fn refused_as(d: &Value, kind: FaultKind, pointer: &str) {
    let f = osd::read(&d.to_string(), Kind::Device).expect_err(&format!("refused: {d}"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}\n{d}");
}

fn refused(d: &Value, pointer: &str) {
    refused_as(d, FaultKind::BadValue, pointer);
}

/// The reader and the 0.7 schema both refuse `d`, the reader with this kind at this pointer.
fn refused_by_both(schemas: &common::Schemas, d: &Value, kind: FaultKind, pointer: &str) {
    refused_as(d, kind, pointer);
    assert!(schemas.check(Kind::Device, d).is_some(), "the schema accepts {d}");
}

// ---- the sysex way of 0.7

#[test]
fn a_sysex_way_gives_each_number_of_the_frame_under_its_name() {
    let schemas = common::Schemas::load("0.7");
    for given in sets_of_numbers() {
        let d = one_way(sysex(&given));
        let read = ways(&d);
        assert_eq!(numbers(&read[0][0]), named(&given), "{given}");
        assert_eq!(written(&d), d, "{given}");
        assert_eq!(schemas.check(Kind::Device, &d), None, "{given}");
    }
}

#[test]
fn a_note_of_a_sysex_way_of_0_7_is_never_read_as_a_number() {
    let d = one_way(sysex(&json!({"pr": 16, "_flat": {"id": 9, "p": 3}})));
    assert_eq!(numbers(&ways(&d)[0][0]), named(&json!({"pr": 16})));
    assert_eq!(written(&d), d);
    assert_eq!(to_0_7(&d)[0]["ways"], d["params"]["entries"][0]["ways"]);
}

// ---- the refusals of a way of 0.7

#[test]
fn a_sysex_way_with_none_of_the_five_numbers_is_refused_at_the_way() {
    let schemas = common::Schemas::load("0.7");
    for given in [json!({}), json!({"value_bits": 7}), json!({"range": [0, 100], "transmit": "absolute"}), json!({"_flat": {"pr": 16}})] {
        refused_by_both(&schemas, &one_way(sysex(&given)), FaultKind::BadValue, PARAM_WAY);
    }
    let second = doc("0.7", vec![json!({"osc": "/a", "ways": [sysex(&json!({"id": 1})), sysex(&json!({}))]})]);
    refused_by_both(&schemas, &second, FaultKind::BadValue, "/params/entries/0/ways/1");
}

#[test]
fn a_ctrlindex_on_a_sysex_way_is_an_unknown_field() {
    let schemas = common::Schemas::load("0.7");
    // The writings of the earlier 0.7 candidate, which no converter rewrites, then a list beside a number.
    for given in [
        json!({"ctrlIndex": [16, 2, 0, 0]}),
        json!({"ctrlIndex": [5]}),
        json!({"ctrlIndex": []}),
        json!({"ctrlIndex": [16], "_flat": {"pr": 16}}),
        json!({"ctrlIndex": [16], "pr": 16}),
        json!({"ctrlIndex": [5], "id": 5}),
    ] {
        let d = one_way(sysex(&given));
        let pointer = format!("{PARAM_WAY}/ctrlIndex");
        refused_by_both(&schemas, &d, FaultKind::UnknownField, &pointer);
        let fault = osd::convert_to(&d.to_string(), Kind::Device, Version::V0_7).expect_err(&format!("no converter rewrites {d}"));
        assert_eq!((fault.kind, fault.pointer), (FaultKind::UnknownField, pointer), "{given}");
    }
}

#[test]
fn a_byte_of_a_sysex_way_outside_0_to_255_or_not_an_integer_is_refused_at_the_field() {
    let schemas = common::Schemas::load("0.7");
    for key in BYTES {
        for value in [json!(-1), json!(256), json!(70000), json!(1.5), json!("16"), json!([16]), json!(true)] {
            let way = with(sysex(&json!({"id": 1})), key, value.clone());
            refused_by_both(&schemas, &one_way(way), FaultKind::BadValue, &format!("{PARAM_WAY}/{key}"));
            let alone = with(sysex(&json!({})), key, value);
            refused_by_both(&schemas, &one_way(alone), FaultKind::BadValue, &format!("{PARAM_WAY}/{key}"));
        }
    }
}

#[test]
fn an_id_of_a_sysex_way_that_is_not_an_integer_is_refused_at_the_field() {
    let schemas = common::Schemas::load("0.7");
    for value in [json!(1.5), json!(-0.5), json!("5"), json!([5]), json!(true), json!({"id": 5})] {
        for given in [json!({}), json!({"pr": 16})] {
            let way = with(sysex(&given), "id", value.clone());
            refused_by_both(&schemas, &one_way(way), FaultKind::BadValue, &format!("{PARAM_WAY}/id"));
        }
    }
}

/// A way of each kind of MIDI, whole.
fn ways_of_midi() -> Vec<Value> {
    vec![
        json!({"ctrlType": "cc", "ctrlIndex": [7]}),
        json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14}),
        json!({"ctrlType": "rpn", "ctrlIndex": [0, 1], "value_bits": 7}),
        json!({"ctrlType": "chPress"}),
        json!({"ctrlType": "pPress", "ctrlIndex": [60]}),
        json!({"ctrlType": "pBend"}),
        json!({"ctrlType": "note", "ctrlIndex": [36]}),
    ]
}

#[test]
fn none_of_the_five_numbers_is_a_field_of_a_way_of_a_kind_of_midi() {
    let schemas = common::Schemas::load("0.7");
    for way in ways_of_midi() {
        assert_eq!(schemas.check(Kind::Device, &one_control_way(way.clone())), None, "{way}");
        for key in NAMES {
            // Whatever its value: the field is unknown before its value is judged.
            for value in [json!(1), json!(300), json!("x")] {
                let d = one_control_way(with(way.clone(), key, value));
                refused_by_both(&schemas, &d, FaultKind::UnknownField, &format!("{CONTROL_WAY}/{key}"));
            }
        }
    }
}

// ---- the order of the faults of a way

#[test]
fn a_kind_its_table_does_not_take_is_refused_before_any_fault_of_the_numbers() {
    let pointer = format!("{CONTROL_WAY}/ctrlType");
    for given in [
        json!({"p": 16}),
        json!({}),
        json!({"pr": 300}),
        json!({"id": "x"}),
        json!({"ctrlIndex": [16, 2, 0, 0]}),
        json!({"ctrlIndex": [16], "pr": 300}),
    ] {
        refused(&one_control_way(sysex(&given)), &pointer);
    }
    let pointer = format!("{PARAM_WAY}/ctrlType");
    for way in [
        json!({"ctrlType": "cc", "ctrlIndex": [7], "pr": 16}),
        json!({"ctrlType": "cc", "pr": 300}),
        json!({"ctrlType": "chPress", "ctrlIndex": [1], "id": "x"}),
    ] {
        refused(&one_way(way), &pointer);
    }
}

#[test]
fn a_field_the_kind_of_a_way_does_not_hold_is_refused_before_a_wrong_or_a_missing_number() {
    let pointer = format!("{PARAM_WAY}/ctrlIndex");
    for given in [json!({"ctrlIndex": [1]}), json!({"ctrlIndex": [1], "pr": 300}), json!({"ctrlIndex": "x", "id": 1.5})] {
        refused_as(&one_way(sysex(&given)), FaultKind::UnknownField, &pointer);
    }
    let d = one_control_way(json!({"ctrlType": "cc", "ctrlIndex": [7], "r": 256}));
    refused_as(&d, FaultKind::UnknownField, &format!("{CONTROL_WAY}/r"));
}

#[test]
fn a_wrong_number_is_refused_at_its_field_and_not_as_a_way_without_a_number() {
    for (given, key) in [(json!({"pr": 300}), "pr"), (json!({"r": -1}), "r"), (json!({"id": "x"}), "id")] {
        refused(&one_way(sysex(&given)), &format!("{PARAM_WAY}/{key}"));
    }
}

// ---- the numbers of 0.4 to 0.6

#[test]
fn the_bytes_pr_p_c_r_of_an_older_text_are_bounded_where_they_are_read() {
    for (version, _) in OLDER {
        for key in BYTES {
            for n in [256, 300, -1] {
                let mut entry = json!({"osc": "/a"});
                entry[key] = n.into();
                refused(&doc(version, vec![entry.clone()]), &format!("/params/entries/0/{key}"));
                entry["id"] = 300.into();
                refused(&doc(version, vec![entry]), &format!("/params/entries/0/{key}"));
            }
        }
    }
}

/// An entry of an older text that gives the fields of `given`, with a range and a direction.
fn older_entry(given: &Value) -> Value {
    let mut entry = given.clone();
    entry["osc"] = "/a".into();
    entry["range"] = json!([0, 100]);
    entry
}

#[test]
fn an_older_text_reads_as_one_sysex_way_that_gives_each_number_under_its_name() {
    for (version, _) in OLDER {
        for given in sets_of_numbers() {
            let d = doc(version, vec![older_entry(&given)]);
            let read = ways(&d);
            assert_eq!(read[0].len(), 1, "{version} {given}");
            assert_eq!(numbers(&read[0][0]), named(&given), "{version} {given}");
            assert_eq!(read[0][0].range, Some([0, 100]), "{version} {given}");
            assert_eq!(written(&d), d, "{version} {given}");
        }
    }
}

#[test]
fn the_conversion_to_0_7_writes_each_number_the_entry_gives_under_its_name_and_no_other() {
    for (version, _) in OLDER {
        for given in sets_of_numbers() {
            let d = doc(version, vec![older_entry(&given)]);
            let way = with(sysex(&given), "range", json!([0, 100]));
            assert_eq!(to_0_7(&d), json!([{"osc": "/a", "ways": [way]}]), "{version} {given}");
        }
    }
}

#[test]
fn an_entry_that_gives_p_c_r_reads_back_without_pr() {
    for (version, _) in OLDER {
        let d = doc(version, vec![json!({"osc": "/a", "p": 2, "c": 3, "r": 4})]);
        let out = to_0_7(&d);
        assert_eq!(out, json!([{"osc": "/a", "ways": [{"ctrlType": "sysex", "p": 2, "c": 3, "r": 4}]}]), "{version}");
        let read = ways(&doc("0.7", out.as_array().unwrap().clone()));
        assert_eq!(numbers(&read[0][0]), [None, Some(2), Some(3), Some(4), None], "{version}");
    }
}

#[test]
fn a_negative_id_alone_converts_to_a_way_that_takes_the_range_and_direction_with_no_note_and_no_warning() {
    for (version, _) in OLDER {
        let mut entry = json!({"osc": "/a", "id": -1, "range": [0, 100]});
        let mut way = json!({"ctrlType": "sysex", "id": -1, "range": [0, 100]});
        // `transmit` and `recognize` of a params entry are fields of 0.6.
        if version == "0.6" {
            for (key, word) in [("transmit", "absolute"), ("recognize", "both")] {
                entry[key] = word.into();
                way[key] = word.into();
            }
        }
        let d = doc(version, vec![entry]);
        assert_eq!(written(&d), d, "{version}");
        assert_eq!(to_0_7(&d), json!([{"osc": "/a", "ways": [way]}]), "{version}");
        for text in [d.to_string(), osd::convert_to(&d.to_string(), Kind::Device, Version::V0_7).unwrap()] {
            let partial: Vec<String> = osd::warnings(&osd::read(&text, Kind::Device).unwrap())
                .into_iter()
                .filter(|w| w.kind == WarningKind::PartialControl)
                .map(|w| w.pointer)
                .collect();
            assert_eq!(partial, Vec::<String>::new(), "{version}: {text}");
        }
    }
}

#[test]
fn a_flat_note_an_older_text_writes_itself_stays_as_written() {
    for (version, _) in OLDER {
        for (given, note) in [
            (json!({"id": -1}), json!("mine")),
            (json!({"id": -1}), json!({"id": 3})),
            (json!({"pr": 16}), json!({"pr": 3, "id": 4})),
            (json!({"p": 2, "id": 300}), json!("mine")),
        ] {
            let mut entry = given.clone();
            entry["osc"] = "/a".into();
            entry["_flat"] = note.clone();
            let d = doc(version, vec![entry]);
            assert_eq!(numbers(&ways(&d)[0][0]), named(&given), "{version} {given}");
            assert_eq!(written(&d), d, "{version}");
            assert_eq!(to_0_7(&d), json!([{"osc": "/a", "_flat": note, "ways": [sysex(&given)]}]), "{version} {given}");
        }
    }
}

#[test]
fn a_flat_note_of_an_entry_without_a_number_gives_no_way() {
    for (version, _) in OLDER {
        let d = doc(version, vec![json!({"osc": "/a", "_flat": {"id": 3}})]);
        assert_eq!(written(&d), d, "{version}");
        assert_eq!(to_0_7(&d), json!([{"osc": "/a", "ways": [], "_flat": {"id": 3}}]), "{version}");
    }
}

#[test]
fn an_older_text_converts_to_a_newer_older_version_with_every_number_kept() {
    let entries = vec![json!({"osc": "/a", "id": 300}), json!({"osc": "/b", "id": -1}), json!({"osc": "/c", "pr": 16})];
    for (from, to) in [("0.4", Version::V0_5), ("0.4", Version::V0_6), ("0.5", Version::V0_6)] {
        assert_eq!(converted(&doc(from, entries.clone()), to)["params"]["entries"], Value::Array(entries.clone()), "{from}");
    }
}

// ---- resolution

#[test]
fn a_resolution_that_carries_an_older_target_to_0_7_keeps_every_number_under_its_name() {
    let target = doc("0.6", vec![json!({"osc": "/a", "id": 300}), json!({"osc": "/b", "id": -1}), json!({"osc": "/c", "p": 2, "c": 3, "r": 4})]);
    let file = json!({"$schema": "https://w3id.org/osd/0.7/device.schema.json", "device": {"name": "T", "osc_prefix": "/f"}, "midi_of": "acme/base"});
    let files: BTreeMap<String, String> = [("acme/base".to_string(), target.to_string()), ("acme/synth".to_string(), file.to_string())].into();
    let resolved = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(resolved.device)))).unwrap();
    assert_eq!(
        out["params"]["entries"],
        json!([
            {"osc": "/a", "ways": [{"ctrlType": "sysex", "id": 300}]},
            {"osc": "/b", "ways": [{"ctrlType": "sysex", "id": -1}]},
            {"osc": "/c", "ways": [{"ctrlType": "sysex", "p": 2, "c": 3, "r": 4}]}
        ])
    );
}

#[test]
fn a_resolution_of_older_files_keeps_every_number_as_given() {
    let target = doc("0.6", vec![json!({"osc": "/a", "id": 300})]);
    let own = vec![json!({"osc": "/b", "id": -1}), json!({"osc": "/c", "pr": 16})];
    let file = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json", "device": {"name": "T", "osc_prefix": "/f"},
        "midi_of": "acme/base", "params": {"entries": own}
    });
    let files: BTreeMap<String, String> = [("acme/base".to_string(), target.to_string()), ("acme/synth".to_string(), file.to_string())].into();
    let resolved = osd::resolve("acme/synth", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(resolved.device)))).unwrap();
    assert_eq!(out["params"]["entries"], json!([{"osc": "/a", "id": 300}, {"osc": "/b", "id": -1}, {"osc": "/c", "pr": 16}]));
}

// ---- the text of 0.7

#[test]
fn the_text_of_the_ways_names_the_cases_of_the_numbers_of_a_sysex_way() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    let section: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with("### 5.3.3 ")).skip(1).take_while(|l| !l.starts_with('#')).collect();
    let cited: Vec<&str> = section.iter().filter(|l| l.starts_with("Cases:")).flat_map(|l| l.split('`').skip(1).step_by(2)).collect();
    let missing: Vec<&str> = [
        "0.7/read/sysex-way-with-id-beyond-a-byte-accepted",
        "0.7/read/sysex-way-with-negative-id-accepted",
        "0.7/read/sysex-way-byte-above-255-refused",
        "0.7/read/sysex-way-byte-below-0-refused",
        "0.7/read/sysex-way-byte-not-an-integer-refused",
        "0.7/read/sysex-way-id-not-an-integer-refused",
        "0.7/read/sysex-way-without-a-number-refused",
        "0.7/read/ctrlindex-on-a-sysex-way-refused",
        "0.7/read/sysex-number-on-a-way-of-midi-refused",
        "0.7/convert/params-p-c-r-keep-their-names",
        "0.7/convert/params-pr-alone-keeps-its-name",
        "0.7/convert/params-id-beside-pr-keeps-both",
        "0.7/convert/params-id-beyond-a-byte-becomes-the-id-of-the-way",
        "0.7/convert/params-negative-id-alone-becomes-the-id-of-the-way",
    ]
    .into_iter()
    .filter(|c| !cited.contains(c) || !common::repo().join("conformance").join(c).join("case.json").is_file())
    .collect();
    assert!(missing.is_empty(), "5.3.3 of spec/0.7.md does not cite, or the suite does not hold: {missing:?}");
}
