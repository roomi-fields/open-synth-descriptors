//! The flat message fields of a control (0.4 to 0.6): `cc`, `cc_lsb`, `nrpn_msb`, `nrpn_lsb`,
//! `nrpn_lsb_range`, `rpn_msb`, `rpn_lsb` and `range`, on the entry and, from 0.6, on each way of
//! `also`. The reader models the ways of 0.7; a device of an older version written back in its own
//! version says exactly the fields its text gave. In 0.4 and 0.5 an entry may mix a CC, an NRPN and
//! an RPN: it is one control with a way for each, and the converter to 0.6 does not choose between
//! them (`spec/0.6.md` 21.3). What 0.7 takes of such a control is in `mixed_controls_0_7`.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

const S5: &str = "https://w3id.org/osd/0.5/device.schema.json";
const S6: &str = "https://w3id.org/osd/0.6/device.schema.json";
const S7: &str = "https://w3id.org/osd/0.7/device.schema.json";

/// A device of `version` with these entries of `cc_params`; a 0.4 text states no `$schema`.
fn doc(version: Version, entries: Vec<Value>) -> Value {
    let mut doc = json!({
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": entries}
    });
    match version {
        Version::V0_4 => {}
        Version::V0_5 => doc["$schema"] = json!(S5),
        Version::V0_6 => doc["$schema"] = json!(S6),
        _ => doc["$schema"] = json!(S7),
    }
    doc
}

const FLAT_VERSIONS: [Version; 2] = [Version::V0_4, Version::V0_5];

fn read(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}\n{doc}"))
}

fn entries_of(d: Descriptor) -> Vec<osd::CcEntry> {
    match d {
        Descriptor::Device(d) => d.cc_params.unwrap().entries,
        _ => panic!("a device"),
    }
}

fn written(doc: &Value) -> Value {
    serde_json::from_str(&osd::write(&read(doc))).unwrap()
}

fn assert_unchanged(doc: &Value) {
    assert_eq!(&written(doc), doc, "read then written: {doc}");
}

/// A way as `(kind, numbers, value_bits, range)`.
type Shape = (String, Vec<i64>, Option<i64>, Option<[i64; 2]>);

fn shapes(doc: &Value, entry: usize) -> Vec<Shape> {
    entries_of(read(doc))[entry]
        .ways()
        .iter()
        .map(|w| (w.ctrl_type.clone(), w.ctrl_index.clone().unwrap_or_default(), w.value_bits, w.range))
        .collect()
}

fn shape(kind: &str, index: &[i64], bits: Option<i64>, range: Option<[i64; 2]>) -> Shape {
    (kind.to_string(), index.to_vec(), bits, range)
}

/// The synthetic control: a CC and an NRPN on the entry.
fn synthetic(msb: u8) -> Value {
    json!({"osc": "/x", "cc": 93, "nrpn_msb": msb, "nrpn_lsb": 102, "range": [0, 127]})
}

/// The same control as 0.6 says it: the NRPN a way of `also`.
fn synthetic_0_6(msb: u8) -> Value {
    json!({"osc": "/x", "cc": 93, "range": [0, 127], "also": [{"nrpn_msb": msb, "nrpn_lsb": 102, "value_bits": 14}]})
}

// ---- what is written back

#[test]
fn a_control_with_a_cc_and_an_nrpn_is_written_back_unchanged_in_0_4_and_0_5() {
    for version in FLAT_VERSIONS {
        assert_unchanged(&doc(version, vec![synthetic(1)]));
    }
}

#[test]
fn an_nrpn_msb_of_zero_beside_a_cc_is_written_back_in_0_4_and_0_5() {
    for version in FLAT_VERSIONS {
        assert_unchanged(&doc(version, vec![synthetic(0)]));
        assert_unchanged(&doc(version, vec![json!({"osc": "/z", "cc": 0, "nrpn_msb": 0, "nrpn_lsb": 0})]));
    }
}

#[test]
fn the_0_6_form_of_a_cc_and_an_nrpn_is_written_back_unchanged_an_msb_of_zero_included() {
    for msb in [1, 0] {
        assert_unchanged(&doc(Version::V0_6, vec![synthetic_0_6(msb)]));
    }
}

#[test]
fn a_control_with_a_cc_and_no_nrpn_lsb_never_gains_one() {
    for version in FLAT_VERSIONS {
        for entry in [
            json!({"osc": "/a", "cc": 93}),
            json!({"osc": "/a", "cc": 93, "nrpn_msb": 1}),
            json!({"osc": "/a", "cc": 93, "nrpn_msb": 0, "range": [0, 127]}),
        ] {
            assert_unchanged(&doc(version, vec![entry]));
        }
    }
}

#[test]
fn an_nrpn_or_an_rpn_given_by_one_number_never_gains_the_other_in_0_4_and_0_5() {
    for version in FLAT_VERSIONS {
        for key in ["nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"] {
            for number in [0, 5] {
                let mut entry = json!({"osc": "/a", "range": [0, 9]});
                entry[key] = json!(number);
                assert_unchanged(&doc(version, vec![entry]));
            }
        }
    }
}

#[test]
fn an_nrpn_or_an_rpn_given_by_one_number_never_gains_the_other_on_a_0_6_entry_or_a_way_of_also() {
    for key in ["nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"] {
        for number in [0, 5] {
            let mut entry = json!({"osc": "/a", "value_bits": 7, "range": [0, 9]});
            entry[key] = json!(number);
            assert_unchanged(&doc(Version::V0_6, vec![entry]));
            let mut way = json!({"value_bits": 14, "range": [0, 100]});
            way[key] = json!(number);
            assert_unchanged(&doc(Version::V0_6, vec![json!({"osc": "/a", "cc": 7, "also": [way]})]));
        }
    }
}

#[test]
fn a_cc_lsb_without_a_cc_is_written_back_as_a_cc_lsb_in_0_4_and_0_5() {
    for version in FLAT_VERSIONS {
        assert_unchanged(&doc(version, vec![json!({"osc": "/a", "cc_lsb": 33})]));
        assert_unchanged(&doc(version, vec![json!({"osc": "/a", "cc_lsb": 0, "range": [0, 127]})]));
    }
}

#[test]
fn a_cc_lsb_without_a_cc_is_written_back_as_a_cc_lsb_on_a_0_6_entry_and_on_a_way_of_also() {
    assert_unchanged(&doc(Version::V0_6, vec![json!({"osc": "/a", "cc_lsb": 33, "range": [0, 127]})]));
    assert_unchanged(&doc(Version::V0_6, vec![json!({"osc": "/a", "cc": 7, "also": [{"cc_lsb": 33}]})]));
}

#[test]
fn an_nrpn_lsb_range_is_written_back_beside_any_message_or_none() {
    for version in [Version::V0_4, Version::V0_5, Version::V0_6] {
        for entry in [
            json!({"osc": "/a", "cc": 93, "nrpn_lsb_range": [0, 15]}),
            json!({"osc": "/a", "nrpn_lsb_range": [0, 15]}),
            json!({"osc": "/a", "nrpn_lsb_range": [3, 4], "range": [0, 127]}),
        ] {
            assert_unchanged(&doc(version, vec![entry]));
        }
    }
    for version in FLAT_VERSIONS {
        assert_unchanged(&doc(version, vec![json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 1, "nrpn_lsb_range": [0, 15]})]));
    }
    assert_unchanged(&doc(Version::V0_6, vec![json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 1, "value_bits": 14, "nrpn_lsb_range": [0, 15]})]));
    assert_unchanged(&doc(Version::V0_6, vec![json!({"osc": "/a", "nrpn_lsb_range": [0, 15], "also": [{"cc": 3}]})]));
}

#[test]
fn the_range_of_a_control_that_gives_no_message_is_written_back() {
    for version in [Version::V0_4, Version::V0_5, Version::V0_6] {
        assert_unchanged(&doc(version, vec![json!({"osc": "/a", "range": [0, 127]}), json!({"osc": "/b"})]));
    }
}

#[test]
fn a_0_6_control_whose_messages_are_all_in_also_keeps_them_there() {
    for entry in [
        json!({"osc": "/a", "also": [{"cc": 1}]}),
        json!({"osc": "/a", "range": [0, 127], "also": [{"cc": 2, "range": [0, 9]}, {"cc": 3}]}),
        json!({"osc": "/a", "also": [{"nrpn_msb": 1, "value_bits": 7}]}),
    ] {
        assert_unchanged(&doc(Version::V0_6, vec![entry]));
    }
}

const MESSAGE_FIELDS: [&str; 6] = ["cc", "cc_lsb", "nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb"];

/// A control with the message fields `mask` names, each with a number of its own; the number of
/// the MSB of a pair is 0.
fn with_fields(osc: &str, mask: u32) -> Map<String, Value> {
    let mut entry = Map::new();
    entry.insert("osc".into(), json!(osc));
    for (bit, (key, number)) in MESSAGE_FIELDS.iter().zip([93, 33, 0, 102, 0, 2]).enumerate() {
        if mask & (1 << bit) != 0 {
            entry.insert((*key).into(), json!(number));
        }
    }
    entry
}

#[test]
fn every_set_of_flat_message_fields_of_0_4_and_0_5_is_written_back_as_given() {
    let mut changed = Vec::new();
    for version in FLAT_VERSIONS {
        for mask in 0..1u32 << MESSAGE_FIELDS.len() {
            for (range, block) in [(false, false), (true, false), (false, true), (true, true)] {
                let mut entry = with_fields("/a", mask);
                if range {
                    entry.insert("range".into(), json!([0, 127]));
                }
                if block {
                    entry.insert("nrpn_lsb_range".into(), json!([0, 15]));
                }
                let text = doc(version, vec![Value::Object(entry)]);
                let back = written(&text);
                if back != text {
                    changed.push(format!("{version:?} {} became {}", text["cc_params"]["entries"][0], back["cc_params"]["entries"][0]));
                }
            }
        }
    }
    common::report("controls changed by a read and a write", changed);
}

#[test]
fn every_set_of_message_fields_of_one_kind_of_0_6_is_written_back_as_given_on_the_entry_and_in_also() {
    // The masks of one kind each: the CC fields, the NRPN fields, the RPN fields.
    let masks: Vec<u32> = [0, 2, 4].iter().flat_map(|shift| (1..4u32).map(move |m| m << shift)).collect();
    let mut changed = Vec::new();
    for mask in masks {
        for range in [false, true] {
            let mut way = with_fields("/a", mask);
            way.remove("osc");
            if mask > 3 {
                way.insert("value_bits".into(), json!(14));
            }
            if range {
                way.insert("range".into(), json!([0, 127]));
            }
            let mut flat = way.clone();
            flat.insert("osc".into(), json!("/a"));
            for entry in [Value::Object(flat), json!({"osc": "/a", "cc": 7, "also": [way]})] {
                let text = doc(Version::V0_6, vec![entry]);
                let back = written(&text);
                if back != text {
                    changed.push(format!("{} became {}", text["cc_params"]["entries"][0], back["cc_params"]["entries"][0]));
                }
            }
        }
    }
    common::report("controls changed by a read and a write", changed);
}

// ---- a 0.4 or 0.5 entry that mixes kinds reads as one control with a way for each

#[test]
fn a_flat_cc_nrpn_and_rpn_of_0_4_or_0_5_read_as_one_control_with_three_ways_in_that_order() {
    for version in FLAT_VERSIONS {
        let d = doc(version, vec![json!({
            "osc": "/x", "rpn_msb": 0, "rpn_lsb": 2, "nrpn_msb": 1, "nrpn_lsb": 102, "cc": 93, "range": [0, 127]
        })]);
        assert_eq!(entries_of(read(&d)).len(), 1);
        let range = Some([0, 127]);
        assert_eq!(
            shapes(&d, 0),
            vec![
                shape("cc", &[93], None, range),
                shape("nrpn", &[1, 102], Some(14), range),
                shape("rpn", &[0, 2], Some(14), range),
            ]
        );
    }
}

#[test]
fn each_pair_of_flat_kinds_of_0_4_or_0_5_reads_as_two_ways_with_their_numbers() {
    for version in FLAT_VERSIONS {
        let d = doc(version, vec![
            json!({"osc": "/a", "cc": 93, "nrpn_msb": 0, "nrpn_lsb": 102}),
            json!({"osc": "/b", "cc": 1, "cc_lsb": 33, "rpn_msb": 0, "rpn_lsb": 1, "range": [0, 16383]}),
            json!({"osc": "/c", "nrpn_msb": 3, "nrpn_lsb": 4, "rpn_msb": 5, "rpn_lsb": 6}),
        ]);
        assert_eq!(shapes(&d, 0), vec![shape("cc", &[93], None, None), shape("nrpn", &[0, 102], Some(14), None)]);
        let range = Some([0, 16383]);
        assert_eq!(shapes(&d, 1), vec![shape("cc", &[1, 33], Some(14), range), shape("rpn", &[0, 1], Some(14), range)]);
        assert_eq!(shapes(&d, 2), vec![shape("nrpn", &[3, 4], Some(14), None), shape("rpn", &[5, 6], Some(14), None)]);
    }
}

#[test]
fn the_block_of_a_mixed_flat_entry_is_on_its_nrpn_way_only() {
    for version in FLAT_VERSIONS {
        let d = doc(version, vec![json!({"osc": "/a", "cc": 93, "nrpn_msb": 1, "nrpn_lsb": 0, "nrpn_lsb_range": [0, 15]})]);
        let ways = entries_of(read(&d))[0].ways().to_vec();
        assert_eq!(ways.len(), 2);
        assert_eq!(ways[0].nrpn_lsb_range, None);
        assert_eq!(ways[1].nrpn_lsb_range, Some([0, 15]));
    }
}

#[test]
fn a_0_6_entry_that_mixes_flat_kinds_is_still_refused_at_the_entry() {
    for entry in [synthetic(1), json!({"osc": "/a", "cc_lsb": 33, "rpn_lsb": 1, "value_bits": 14})] {
        let d = doc(Version::V0_6, vec![json!({"osc": "/first", "cc": 1}), entry]);
        let f = osd::read(&d.to_string(), Kind::Device).expect_err("mixed kinds");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/cc_params/entries/1"), "{f}");
    }
}

// ---- the converters

fn mixes() -> Vec<Value> {
    vec![
        synthetic(1),
        synthetic(0),
        json!({"osc": "/x", "cc": 93, "nrpn_msb": 1}),
        json!({"osc": "/x", "cc_lsb": 33, "nrpn_lsb": 2}),
        json!({"osc": "/x", "cc": 93, "rpn_msb": 0, "rpn_lsb": 2}),
        json!({"osc": "/x", "nrpn_msb": 1, "nrpn_lsb": 2, "rpn_msb": 0, "rpn_lsb": 2}),
    ]
}

#[test]
fn converting_0_4_to_0_5_keeps_the_cc_and_the_nrpn_with_the_numbers_the_text_gives() {
    for mix in mixes() {
        let text = doc(Version::V0_4, vec![mix.clone()]).to_string();
        let out = osd::convert_to(&text, Kind::Device, Version::V0_5).unwrap_or_else(|f| panic!("{f}\n{text}"));
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["$schema"], S5);
        assert_eq!(out["cc_params"]["entries"], json!([mix]));
    }
}

fn assert_not_convertible_at_the_second_entry(version: Version) {
    let mut converted = Vec::new();
    for mix in mixes() {
        let text = doc(version, vec![json!({"osc": "/first", "cc": 1}), mix.clone()]).to_string();
        let results = [
            ("0.6", osd::convert_to(&text, Kind::Device, Version::V0_6)),
            ("the published version", osd::convert(&text, Kind::Device)),
        ];
        for (target, result) in results {
            match result {
                Err(f) if (f.kind, f.pointer.as_str()) == (FaultKind::NotConvertible, "/cc_params/entries/1") => {}
                Err(f) => converted.push(format!("{mix} to {target}: {f}")),
                Ok(out) => converted.push(format!("{mix} to {target}: {out}")),
            }
        }
    }
    common::report("mixed controls not refused as NotConvertible at the entry", converted);
}

#[test]
fn a_0_4_control_that_mixes_flat_kinds_is_not_convertible_to_0_6() {
    assert_not_convertible_at_the_second_entry(Version::V0_4);
}

#[test]
fn a_0_5_control_that_mixes_flat_kinds_is_not_convertible_to_0_6() {
    assert_not_convertible_at_the_second_entry(Version::V0_5);
}

fn catalogue(files: &[(&str, &Value)]) -> BTreeMap<String, String> {
    files.iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn variant(version: Version) -> Value {
    let mut v = doc(version, vec![]);
    v.as_object_mut().unwrap().remove("cc_params");
    v["midi_of"] = json!("acme/b");
    v
}

#[test]
fn resolving_a_0_5_driver_onto_a_0_4_target_keeps_both_messages_of_a_mixed_control() {
    let files = catalogue(&[("acme/a", &variant(Version::V0_5)), ("acme/b", &doc(Version::V0_4, vec![synthetic(1)]))]);
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    let out: Value = serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device)))).unwrap();
    assert_eq!(out["cc_params"]["entries"], json!([synthetic(1)]));
}

/// The entries of the 0.7 text of a 0.6 device with these entries, checked to be written back as
/// converted.
fn converted_to_0_7(entries: Vec<Value>) -> Value {
    let text = doc(Version::V0_6, entries).to_string();
    let out = osd::convert_to(&text, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{text}"));
    let value: Value = serde_json::from_str(&out).unwrap();
    let back = osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}\n{out}"));
    assert_eq!(serde_json::from_str::<Value>(&osd::write(&back)).unwrap(), value);
    value["cc_params"]["entries"].clone()
}

#[test]
fn the_0_6_form_of_a_cc_and_an_nrpn_converts_to_0_7_as_two_ways_with_both_numbers() {
    for msb in [1, 0] {
        assert_eq!(
            converted_to_0_7(vec![synthetic_0_6(msb)]),
            json!([{"osc": "/x", "ways": [
                {"ctrlType": "cc", "ctrlIndex": [93], "range": [0, 127]},
                {"ctrlType": "nrpn", "ctrlIndex": [msb, 102], "value_bits": 14}
            ]}])
        );
    }
}

#[test]
fn a_0_7_text_holds_nothing_kept_to_write_an_older_version_back() {
    let got = converted_to_0_7(vec![
        json!({"osc": "/msb", "nrpn_msb": 1, "value_bits": 14}),
        json!({"osc": "/lsb", "nrpn_lsb": 5, "value_bits": 7, "range": [0, 9]}),
        json!({"osc": "/rpn", "rpn_lsb": 2, "value_bits": 14}),
        json!({"osc": "/low", "cc_lsb": 33}),
        json!({"osc": "/block", "cc": 93, "nrpn_lsb_range": [0, 15]}),
        json!({"osc": "/also", "also": [{"cc": 2}, {"nrpn_msb": 3, "value_bits": 7}, {"cc_lsb": 40}]}),
    ]);
    assert_eq!(
        got,
        json!([
            {"osc": "/msb", "ways": [], "_incomplete": [{"nrpn_msb": 1, "value_bits": 14}]},
            {"osc": "/lsb", "ways": [], "range": [0, 9], "_incomplete": [{"nrpn_lsb": 5, "value_bits": 7}]},
            {"osc": "/rpn", "ways": [], "_incomplete": [{"rpn_lsb": 2, "value_bits": 14}]},
            {"osc": "/low", "ways": [], "_incomplete": [{"cc_lsb": 33}]},
            {"osc": "/block", "_nrpn_lsb_range": [0, 15], "ways": [{"ctrlType": "cc", "ctrlIndex": [93]}]},
            {"osc": "/also", "ways": [{"ctrlType": "cc", "ctrlIndex": [2]}],
                "_incomplete": [{"nrpn_msb": 3, "value_bits": 7}, {"cc_lsb": 40}]}
        ])
    );
}

// ---- the conformance cases

#[test]
fn the_conformance_cases_of_0_5_and_0_6_write_the_numbers_of_a_control_back_as_given() {
    for version in ["0.5", "0.6"] {
        let dir = common::repo().join("conformance").join(version).join("read/control-numbers-written-back");
        let meta: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        let text: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("descriptor.json")).unwrap()).unwrap();
        assert_eq!(meta["result"], text, "{version}: the result is the text itself");
        assert_eq!(written(&text), meta["result"], "{version}");
    }
}
