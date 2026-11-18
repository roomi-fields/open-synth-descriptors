//! A control reachable by several ways (0.6): the flat fields of a CC entry (cc, cc_lsb, nrpn_msb,
//! nrpn_lsb, rpn_msb, rpn_lsb, range) are its first way, and `also`, a list, gives the ways
//! after it, each with one message kind and its own range. `osd` reads and writes them in the
//! order written, gives them as `CcEntry::ways`, and refuses a message that two ways or two
//! controls claim, naming both places.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn doc(entries: Vec<Value>) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": entries}
    })
}

fn one(entry: Value) -> Value {
    doc(vec![entry])
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn device(doc: &Value) -> osd::Device {
    match read(doc).unwrap_or_else(|f| panic!("{f}")) {
        Descriptor::Device(d) => *d,
        _ => panic!("a device"),
    }
}

fn entries(doc: &Value) -> Vec<osd::CcEntry> {
    device(doc).cc_params.unwrap().entries
}

/// The fault is `BadValue` at `pointer`, and its rule names `other` too.
fn refused_with(doc: &Value, pointer: &str, other: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
    assert!(f.rule.contains(other), "the rule does not name {other}: {}", f.rule);
}

fn refused_at(doc: &Value, pointer: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
}

#[test]
fn a_control_with_a_cc_first_and_an_nrpn_in_also_reads_as_one_control_with_two_ways() {
    let d = one(json!({
        "osc": "/filter/cutoff", "cc": 74, "range": [0, 127],
        "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]}]
    }));
    let list = entries(&d);
    assert_eq!(list.len(), 1);
    let ways = list[0].ways();
    assert_eq!(ways.len(), 2);
    assert_eq!((ways[0].cc, ways[0].nrpn_msb, ways[0].range), (Some(74), None, Some([0, 127])));
    assert_eq!((ways[1].cc, ways[1].nrpn_msb, ways[1].nrpn_lsb, ways[1].range), (None, Some(1), Some(2), Some([0, 16383])));
}

#[test]
fn every_message_kind_is_a_way_in_also_with_its_own_range() {
    let d = one(json!({
        "osc": "/a", "cc": 7, "range": [0, 127],
        "also": [
            {"cc": 1, "cc_lsb": 33, "range": [0, 16383]},
            {"nrpn_msb": 3, "nrpn_lsb": 4, "range": [0, 1000]},
            {"rpn_msb": 0, "rpn_lsb": 2, "range": [0, 24]},
            {"cc": 8}
        ]
    }));
    let ways = entries(&d)[0].ways();
    assert_eq!(ways.len(), 5);
    assert_eq!((ways[1].cc, ways[1].cc_lsb, ways[1].range), (Some(1), Some(33), Some([0, 16383])));
    assert_eq!((ways[2].nrpn_msb, ways[2].nrpn_lsb, ways[2].range), (Some(3), Some(4), Some([0, 1000])));
    assert_eq!((ways[3].rpn_msb, ways[3].rpn_lsb, ways[3].range), (Some(0), Some(2), Some([0, 24])));
    assert_eq!((ways[4].cc, ways[4].range), (Some(8), None));
}

#[test]
fn the_ways_come_in_the_order_written_the_flat_way_first_then_also_in_order() {
    let d = one(json!({
        "osc": "/a", "nrpn_msb": 9, "nrpn_lsb": 9,
        "also": [{"cc": 40}, {"cc": 1, "cc_lsb": 33}, {"cc": 3}]
    }));
    let ways = entries(&d)[0].ways();
    let seen: Vec<(Option<u8>, Option<u8>, Option<u8>)> = ways.iter().map(|w| (w.nrpn_msb, w.cc, w.cc_lsb)).collect();
    assert_eq!(seen, vec![(Some(9), None, None), (None, Some(40), None), (None, Some(1), Some(33)), (None, Some(3), None)]);
}

#[test]
fn a_control_with_one_way_reads_as_in_0_5_with_that_one_way() {
    let d = one(json!({"osc": "/a", "cc": 74, "range": [0, 127], "section": "Filter"}));
    let e = &entries(&d)[0];
    assert!(e.also.is_none());
    let ways = e.ways();
    assert_eq!(ways.len(), 1);
    assert_eq!((ways[0].cc, ways[0].range), (Some(74), Some([0, 127])));
    let n = one(json!({"osc": "/n", "nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 9]}));
    let ways = entries(&n)[0].ways();
    assert_eq!((ways.len(), ways[0].nrpn_msb, ways[0].nrpn_lsb, ways[0].range), (1, Some(1), Some(2), Some([0, 9])));
}

#[test]
fn a_control_with_also_is_written_back_key_for_key_and_one_without_writes_no_also() {
    let with = one(json!({
        "osc": "/a", "cc": 74, "range": [0, 127], "priority": 2,
        "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]}, {"cc": 1, "cc_lsb": 33}]
    }));
    let out: Value = serde_json::from_str(&osd::write(&read(&with).unwrap())).unwrap();
    assert_eq!(out, with);
    let without = one(json!({"osc": "/a", "cc": 74}));
    let out: Value = serde_json::from_str(&osd::write(&read(&without).unwrap())).unwrap();
    assert!(out["cc_params"]["entries"][0].get("also").is_none());
}

#[test]
fn also_is_unknown_before_0_6() {
    let text = one(json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2}]})).to_string().replace("osd/0.6/", "osd/0.5/");
    let f = osd::read(&text, Kind::Device).expect_err("0.5 has no also");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/cc_params/entries/0/also"), "{f}");
}

#[test]
fn an_also_that_is_not_a_list_of_objects_is_refused() {
    for bad in [json!("x"), json!({"cc": 1}), json!([5]), json!([[]])] {
        let d = one(json!({"osc": "/a", "cc": 74, "also": bad}));
        assert!(read(&d).is_err(), "{d}");
    }
}

#[test]
fn a_way_in_also_gives_one_message_kind_only() {
    let mixes = [
        json!({"cc": 5, "nrpn_msb": 1, "nrpn_lsb": 2}),
        json!({"nrpn_msb": 1, "nrpn_lsb": 2, "rpn_msb": 0, "rpn_lsb": 1}),
        json!({"cc": 5, "rpn_msb": 0, "rpn_lsb": 1}),
        json!({"cc_lsb": 33, "nrpn_msb": 1, "nrpn_lsb": 2}),
    ];
    for mix in mixes {
        let d = one(json!({"osc": "/a", "cc": 74, "also": [mix]}));
        refused_at(&d, "/cc_params/entries/0/also/0");
    }
}

#[test]
fn a_way_in_also_without_a_message_kind_is_refused_at_the_way() {
    for bare in [json!({}), json!({"range": [0, 127]})] {
        let d = one(json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2}, bare]}));
        refused_at(&d, "/cc_params/entries/0/also/1");
    }
}

#[test]
fn a_way_in_also_that_repeats_the_flat_way_or_another_way_is_refused_naming_both_places() {
    let flat = one(json!({"osc": "/a", "cc": 74, "also": [{"cc": 74, "range": [0, 100]}]}));
    refused_with(&flat, "/cc_params/entries/0/also/0", "/cc_params/entries/0");
    let other = one(json!({
        "osc": "/a", "cc": 74,
        "also": [{"nrpn_msb": 1, "nrpn_lsb": 2}, {"cc": 9}, {"nrpn_msb": 1, "nrpn_lsb": 2}]
    }));
    refused_with(&other, "/cc_params/entries/0/also/2", "/cc_params/entries/0/also/0");
}

#[test]
fn a_14_bit_way_claims_its_cc_and_its_lsb_so_it_collides_with_a_simple_cc_on_either() {
    let on_msb = one(json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "also": [{"cc": 1}]}));
    refused_with(&on_msb, "/cc_params/entries/0/also/0", "/cc_params/entries/0");
    let on_lsb = one(json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "also": [{"cc": 33}]}));
    refused_with(&on_lsb, "/cc_params/entries/0/also/0", "/cc_params/entries/0");
    let lsb_first = one(json!({"osc": "/a", "cc": 33, "also": [{"cc": 1, "cc_lsb": 33}]}));
    refused_with(&lsb_first, "/cc_params/entries/0/also/0", "/cc_params/entries/0");
    let two_14 = one(json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "also": [{"cc": 2, "cc_lsb": 33}]}));
    refused_with(&two_14, "/cc_params/entries/0/also/0", "/cc_params/entries/0");
}

#[test]
fn two_controls_that_claim_the_same_message_are_refused_at_the_later_naming_the_earlier() {
    let cc = doc(vec![json!({"osc": "/a", "cc": 74}), json!({"osc": "/b", "cc": 74})]);
    refused_with(&cc, "/cc_params/entries/1", "/cc_params/entries/0");
    let nrpn = doc(vec![
        json!({"osc": "/a", "cc": 1}),
        json!({"osc": "/b", "nrpn_msb": 4, "nrpn_lsb": 5}),
        json!({"osc": "/c", "nrpn_msb": 4, "nrpn_lsb": 5}),
    ]);
    refused_with(&nrpn, "/cc_params/entries/2", "/cc_params/entries/1");
    let rpn = doc(vec![json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 2}), json!({"osc": "/b", "rpn_msb": 0, "rpn_lsb": 2})]);
    refused_with(&rpn, "/cc_params/entries/1", "/cc_params/entries/0");
}

#[test]
fn a_14_bit_control_collides_with_a_simple_cc_control_on_either_number() {
    for (first, second) in [
        (json!({"osc": "/a", "cc": 1, "cc_lsb": 33}), json!({"osc": "/b", "cc": 1})),
        (json!({"osc": "/a", "cc": 1, "cc_lsb": 33}), json!({"osc": "/b", "cc": 33})),
        (json!({"osc": "/a", "cc": 33}), json!({"osc": "/b", "cc": 1, "cc_lsb": 33})),
    ] {
        refused_with(&doc(vec![first, second]), "/cc_params/entries/1", "/cc_params/entries/0");
    }
}

#[test]
fn a_way_in_also_that_another_control_claims_is_refused_at_the_later_place_naming_the_earlier_way() {
    let later_flat = doc(vec![
        json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2}]}),
        json!({"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 2}),
    ]);
    refused_with(&later_flat, "/cc_params/entries/1", "/cc_params/entries/0/also/0");
    let later_also = doc(vec![
        json!({"osc": "/a", "cc": 74}),
        json!({"osc": "/b", "cc": 75, "also": [{"cc": 74}]}),
    ]);
    refused_with(&later_also, "/cc_params/entries/1/also/0", "/cc_params/entries/0");
}

#[test]
fn the_same_numbers_on_another_kind_or_another_channel_are_another_message() {
    let kinds = doc(vec![
        json!({"osc": "/a", "cc": 1}),
        json!({"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 1}),
        json!({"osc": "/c", "rpn_msb": 1, "rpn_lsb": 1}),
    ]);
    assert_eq!(entries(&kinds).len(), 3);
    let channels = doc(vec![json!({"osc": "/a", "cc": 74, "channel": 1}), json!({"osc": "/b", "cc": 74, "channel": 2})]);
    assert_eq!(entries(&channels).len(), 2);
    let same = doc(vec![json!({"osc": "/a", "cc": 74, "channel": 2}), json!({"osc": "/b", "cc": 74, "channel": 2})]);
    refused_with(&same, "/cc_params/entries/1", "/cc_params/entries/0");
    let inherited = doc(vec![json!({"osc": "/a", "cc": 74}), json!({"osc": "/b", "cc": 74, "channel": 1})]);
    assert_eq!(entries(&inherited).len(), 2);
}

#[test]
fn a_0_5_descriptor_still_reads_two_controls_that_claim_one_message() {
    let text = doc(vec![json!({"osc": "/a", "cc": 74}), json!({"osc": "/b", "cc": 74})]).to_string().replace("osd/0.6/", "osd/0.5/");
    assert!(osd::read(&text, Kind::Device).is_ok());
}

#[test]
fn converting_a_0_5_descriptor_keeps_each_control_and_never_joins_two() {
    let v05 = json!({
        "$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/filter/cutoff", "cc": 74, "range": [0, 127]},
            {"osc": "/filter/cutoff-fine", "nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]}
        ]}
    });
    for out in [
        osd::convert_to(&v05.to_string(), Kind::Device, Version::V0_6).unwrap(),
        osd::convert(&v05.to_string(), Kind::Device).unwrap(),
    ] {
        let v: Value = serde_json::from_str(&out).unwrap();
        let list = v["cc_params"]["entries"].as_array().unwrap();
        assert_eq!(list.len(), 2, "{out}");
        assert!(list.iter().all(|e| e.get("also").is_none()), "{out}");
        assert_eq!((list[0]["cc"].clone(), list[1]["nrpn_msb"].clone()), (json!(74), json!(1)));
    }
    let out = osd::convert_to(&v05.to_string(), Kind::Device, Version::V0_6).unwrap();
    assert_eq!(entries_of_text(&out).len(), 2);
}

fn entries_of_text(text: &str) -> Vec<osd::CcEntry> {
    match osd::read(text, Kind::Device).unwrap() {
        Descriptor::Device(d) => d.cc_params.unwrap().entries,
        _ => panic!("a device"),
    }
}

/// The conformance cases of the ways: `(case, the fault's pointer, empty for a result)`.
const CASES: [(&str, &str); 3] = [
    ("control-ways-two", ""),
    ("control-way-one", ""),
    ("control-way-claimed-refused", "/cc_params/entries/1"),
];

#[test]
fn the_conformance_cases_of_the_ways_are_read_as_they_say() {
    for (case, pointer) in CASES {
        let dir = common::repo().join("conformance/0.6/read").join(case);
        let meta: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
        let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
        let got = osd::read(&text, Kind::Device);
        if pointer.is_empty() {
            let d = got.unwrap_or_else(|f| panic!("{case}: {f}"));
            let written: Value = serde_json::from_str(&osd::write(&d)).unwrap();
            assert_eq!(written, meta["result"], "{case}");
        } else {
            let f = got.expect_err(case);
            assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{case}: {f}");
            assert_eq!(meta["fault"]["pointer"], pointer, "{case}");
            assert_eq!(meta["schema_accepts"], true, "{case}");
        }
    }
}
