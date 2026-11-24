//! The width of an NRPN or RPN value (0.6): `value_bits` is required on a control, or a way of
//! `also`, that gives an NRPN or an RPN, and says it in the words of the norm: 7 (the data MSB
//! alone) or 14 (the data MSB and LSB). `osd` refuses any other width at the field, and a width on
//! a control that is neither an NRPN nor an RPN.

use serde_json::{json, Value};

use osd::{FaultKind, Kind, Version};

const SCHEMA: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn doc(entry: Value) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [entry]}
    })
}

fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn refused_at(doc: &Value, pointer: &str) {
    let f = read(doc).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
}

fn round_trips(doc: &Value) {
    let out: Value = serde_json::from_str(&osd::write(&read(doc).unwrap_or_else(|f| panic!("{f}")))).unwrap();
    assert_eq!(&out, doc);
}

#[test]
fn an_nrpn_says_its_value_is_7_bits_in_the_data_msb_or_14_bits_over_msb_and_lsb() {
    for bits in [7, 14] {
        round_trips(&doc(json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": bits, "range": [0, 100]})));
    }
}

#[test]
fn an_rpn_says_its_value_is_7_bits_in_the_data_msb_or_14_bits_over_msb_and_lsb() {
    for bits in [7, 14] {
        round_trips(&doc(json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 2, "value_bits": bits})));
    }
}

#[test]
fn a_way_in_also_says_the_width_of_its_own_nrpn_or_rpn_value() {
    round_trips(&doc(json!({
        "osc": "/a", "cc": 74,
        "also": [
            {"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 7},
            {"rpn_msb": 0, "rpn_lsb": 1, "value_bits": 14}
        ]
    })));
}

#[test]
fn a_flat_nrpn_and_a_way_in_also_keep_each_its_own_width() {
    round_trips(&doc(json!({
        "osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 7,
        "also": [{"nrpn_msb": 3, "nrpn_lsb": 4, "value_bits": 14}]
    })));
}

#[test]
fn a_width_outside_the_words_of_the_norm_is_refused_at_the_field_of_the_way() {
    for bad in [json!(0), json!(8), json!(12), json!(16), json!(21), json!(-7), json!(14.5), json!("14"), json!(null)] {
        for entry in [
            json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": bad}),
            json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 1, "value_bits": bad}),
        ] {
            refused_at(&doc(entry), "/cc_params/entries/0/value_bits");
        }
        let also = doc(json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": bad}]}));
        refused_at(&also, "/cc_params/entries/0/also/0/value_bits");
    }
}

#[test]
fn an_nrpn_or_an_rpn_without_a_width_is_refused_as_missing() {
    let flat = [
        json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2}),
        json!({"osc": "/a", "rpn_msb": 0, "rpn_lsb": 1, "range": [0, 16383]}),
    ];
    for entry in flat {
        let f = read(&doc(entry)).expect_err("refused");
        assert_eq!(f.kind, FaultKind::Missing, "{f}");
        assert!(f.pointer.starts_with("/cc_params/entries/0"), "{f}");
    }
    let way = doc(json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2}]}));
    let f = read(&way).expect_err("refused");
    assert_eq!(f.kind, FaultKind::Missing, "{f}");
    assert!(f.pointer.starts_with("/cc_params/entries/0/also/0"), "{f}");
}

#[test]
fn a_width_on_a_control_that_is_neither_an_nrpn_nor_an_rpn_is_refused_at_the_field() {
    refused_at(&doc(json!({"osc": "/a", "cc": 74, "value_bits": 7})), "/cc_params/entries/0/value_bits");
    refused_at(&doc(json!({"osc": "/a", "cc": 1, "cc_lsb": 33, "value_bits": 14})), "/cc_params/entries/0/value_bits");
    let also = doc(json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "also": [{"cc": 9, "value_bits": 7}]}));
    refused_at(&also, "/cc_params/entries/0/also/0/value_bits");
}

#[test]
fn the_width_of_a_value_is_unknown_before_0_6() {
    let text = doc(json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14})).to_string().replace("osd/0.6/", "osd/0.5/");
    let f = osd::read(&text, Kind::Device).expect_err("refused");
    assert_eq!(f.kind, FaultKind::UnknownField, "{f}");
}

#[test]
fn converting_a_0_5_nrpn_or_rpn_gives_it_the_full_14_bit_width_and_the_result_reads() {
    let v05 = json!({
        "$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/n", "nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]},
            {"osc": "/r", "rpn_msb": 0, "rpn_lsb": 1},
            {"osc": "/c", "cc": 74}
        ]}
    });
    let out = osd::convert_to(&v05.to_string(), Kind::Device, Version::V0_6).unwrap();
    let v: Value = serde_json::from_str(&out).unwrap();
    let list = v["cc_params"]["entries"].as_array().unwrap();
    assert_eq!(list[0]["value_bits"], json!(14), "{out}");
    assert_eq!(list[1]["value_bits"], json!(14), "{out}");
    assert!(list[2].get("value_bits").is_none(), "{out}");
    assert!(osd::read(&out, Kind::Device).is_ok(), "{out}");
}

#[test]
fn an_nrpn_lsb_range_is_kept_as_written_and_written_back() {
    let d = doc(json!({"osc": "/a", "nrpn_msb": 3, "nrpn_lsb": 0, "nrpn_lsb_range": [0, 15], "value_bits": 7, "range": [0, 127]}));
    round_trips(&d);
}
