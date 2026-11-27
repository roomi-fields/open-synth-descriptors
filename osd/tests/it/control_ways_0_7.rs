//! A 0.7 control reaches its messages by `ways` (proposal 0009, `spec/0.7.md`): the kinds and
//! fields of a way, the refusal of the flat fields of 0.6, the rules of `value_bits`, the message
//! claimed once by side, the reference scale of a control and the conversion of a value to the
//! scale of a way, and the converter from 0.6.

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.7/device.schema.json";

fn cc_doc(entries: Vec<Value>) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": entries}
    })
}

fn params_doc(entries: Vec<Value>) -> Value {
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": entries}
    })
}

fn one(entry: Value) -> Value {
    cc_doc(vec![entry])
}

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn device(doc: &Value) -> osd::Device {
    match read(doc).unwrap_or_else(|f| panic!("{f}\n{doc}")) {
        Descriptor::Device(d) => *d,
        _ => panic!("a device"),
    }
}

fn cc_entries(doc: &Value) -> Vec<osd::CcEntry> {
    device(doc).cc_params.unwrap().entries
}

fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let f = read(doc).expect_err(&format!("refused: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{f}");
}

fn bad(doc: &Value, pointer: &str) {
    refused(doc, FaultKind::BadValue, pointer);
}

/// The fault is `BadValue` at `pointer`, and its rule names `other` too.
fn bad_naming(doc: &Value, pointer: &str, other: &str) {
    let f = read(doc).expect_err(&format!("refused: {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
    assert!(f.rule.contains(other), "the rule does not name {other}: {}", f.rule);
}

fn ok(doc: &Value) {
    read(doc).unwrap_or_else(|f| panic!("{f}\n{doc}"));
}

fn way(kind: &str, index: Value) -> Value {
    json!({"ctrlType": kind, "ctrlIndex": index})
}

/// A `sysex` way that gives the `id` of its parameter.
fn sysex_id(id: i64) -> Value {
    json!({"ctrlType": "sysex", "id": id})
}

fn with(mut way: Value, key: &str, value: Value) -> Value {
    way[key] = value;
    way
}

/// The word of a direction, as the descriptor writes it.
fn word<T: std::fmt::Debug>(x: &Option<T>) -> Option<String> {
    x.as_ref().map(|d| format!("{d:?}").trim_matches('"').to_lowercase())
}

fn way_at(n: usize, m: usize) -> String {
    format!("/cc_params/entries/{n}/ways/{m}")
}

fn written(doc: &Value) -> Value {
    serde_json::from_str(&osd::write(&read(doc).unwrap_or_else(|f| panic!("{f}")))).unwrap()
}

fn shape(w: &osd::Way) -> (String, Vec<i64>) {
    (w.ctrl_type.clone(), w.ctrl_index.clone().unwrap_or_default())
}

/// The kind of a `sysex` way, its `ctrlIndex` and its numbers `pr`, `p`, `c`, `r`, `id`.
fn sysex_shape(w: &osd::Way) -> (String, Option<Vec<i64>>, [Option<i64>; 5]) {
    (w.ctrl_type.clone(), w.ctrl_index.clone(), [w.pr, w.p, w.c, w.r, w.id])
}

// ---- the form of a way

#[test]
fn every_kind_of_message_is_a_way_with_the_numbers_its_kind_gives() {
    let d = one(json!({"osc": "/a", "ways": [
        way("cc", json!([74])),
        with(way("cc", json!([1, 33])), "value_bits", json!(14)),
        with(way("nrpn", json!([1, 8])), "value_bits", json!(14)),
        with(way("rpn", json!([0, 2])), "value_bits", json!(7)),
        {"ctrlType": "chPress"},
        way("pPress", json!([60])),
        {"ctrlType": "pBend", "value_bits": 14},
        way("note", json!([36]))
    ]}));
    let ways = cc_entries(&d)[0].ways();
    let kinds: Vec<(String, Vec<i64>)> = ways.iter().map(shape).collect();
    let want: Vec<(String, Vec<i64>)> = [
        ("cc", vec![74]),
        ("cc", vec![1, 33]),
        ("nrpn", vec![1, 8]),
        ("rpn", vec![0, 2]),
        ("chPress", vec![]),
        ("pPress", vec![60]),
        ("pBend", vec![]),
        ("note", vec![36]),
    ]
    .into_iter()
    .map(|(k, i)| (k.to_string(), i))
    .collect();
    assert_eq!(kinds, want);
    assert!(ways[4].ctrl_index.is_none() && ways[6].ctrl_index.is_none(), "no index for chPress and pBend");
}

#[test]
fn the_ways_are_written_back_as_read_in_the_order_written_with_every_field_of_a_way() {
    let d = one(json!({"osc": "/a", "ways": [
        {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14, "range": [0, 16383], "transmit": "absolute", "recognize": "both", "nrpn_lsb_range": [8, 15]},
        {"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7, "range": [0, 127], "transmit": "none", "recognize": "relative"}
    ]}));
    assert_eq!(written(&d), d);
}

#[test]
fn a_way_gives_its_direction_by_the_four_words_and_a_word_outside_them_is_refused() {
    for w in ["absolute", "relative", "both", "none"] {
        let d = one(json!({"osc": "/a", "ways": [with(with(way("cc", json!([7])), "transmit", json!(w)), "recognize", json!(w))]}));
        let ways = cc_entries(&d)[0].ways();
        assert_eq!((word(&ways[0].transmit), word(&ways[0].recognize)), (Some(w.to_string()), Some(w.to_string())));
    }
    bad(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([7])), "transmit", json!("sometimes"))]})), &format!("{}/transmit", way_at(0, 0)));
}

#[test]
fn a_control_without_the_key_ways_is_refused_as_missing_and_an_empty_ways_is_read() {
    refused(&one(json!({"osc": "/a"})), FaultKind::Missing, "/cc_params/entries/0/ways");
    ok(&one(json!({"osc": "/a", "ways": []})));
}

#[test]
fn a_way_needs_a_known_kind() {
    refused(&one(json!({"osc": "/a", "ways": [{"ctrlIndex": [7]}]})), FaultKind::Missing, &format!("{}/ctrlType", way_at(0, 0)));
    bad(&one(json!({"osc": "/a", "ways": [way("aftertouch", json!([7]))]})), &format!("{}/ctrlType", way_at(0, 0)));
}

#[test]
fn the_numbers_of_a_way_follow_its_kind() {
    for index in [json!([]), json!([1, 2, 3]), json!([128]), json!([-1]), json!("7")] {
        assert!(read(&one(json!({"osc": "/a", "ways": [way("cc", index.clone())]}))).is_err(), "cc {index}");
    }
    for index in [json!([1]), json!([1, 2, 3]), json!([1, 128])] {
        assert!(read(&one(json!({"osc": "/a", "ways": [with(way("nrpn", index.clone()), "value_bits", json!(14))]}))).is_err(), "nrpn {index}");
    }
    assert!(read(&one(json!({"osc": "/a", "ways": [way("pPress", json!([128]))]}))).is_err());
    assert!(read(&one(json!({"osc": "/a", "ways": [way("note", json!([]))]}))).is_err());
}

#[test]
fn a_pressure_or_a_pitch_bend_way_has_no_ctrlindex_not_even_an_empty_one() {
    for kind in ["chPress", "pBend"] {
        bad(&one(json!({"osc": "/a", "ways": [way(kind, json!([]))]})), &format!("{}/ctrlIndex", way_at(0, 0)));
        bad(&one(json!({"osc": "/a", "ways": [way(kind, json!([1]))]})), &format!("{}/ctrlIndex", way_at(0, 0)));
    }
}

#[test]
fn nrpn_lsb_range_is_kept_as_written_and_claims_only_its_own_message() {
    let d = cc_doc(vec![
        json!({"osc": "/a", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [0, 5], "value_bits": 14, "nrpn_lsb_range": [5, 20]}]}),
        json!({"osc": "/b", "ways": [{"ctrlType": "nrpn", "ctrlIndex": [0, 6], "value_bits": 14}]}),
    ]);
    ok(&d);
    assert_eq!(written(&d)["cc_params"]["entries"][0]["ways"][0]["nrpn_lsb_range"], json!([5, 20]));
}

// ---- the flat form is refused in 0.7

#[test]
fn a_control_of_0_7_refuses_the_flat_fields_of_0_6_at_the_field() {
    let flat = [
        ("cc", json!(74)),
        ("cc_lsb", json!(33)),
        ("nrpn_msb", json!(1)),
        ("nrpn_lsb", json!(2)),
        ("rpn_msb", json!(0)),
        ("rpn_lsb", json!(1)),
        ("value_bits", json!(14)),
        ("range", json!([0, 127])),
        ("also", json!([{"cc": 1}])),
        ("transmit", json!("absolute")),
        ("recognize", json!("absolute")),
    ];
    for (field, value) in flat {
        let mut entry = json!({"osc": "/a", "ways": [way("cc", json!([74]))]});
        entry[field] = value;
        refused(&one(entry), FaultKind::UnknownField, &format!("/cc_params/entries/0/{field}"));
    }
}

#[test]
fn a_params_entry_of_0_7_refuses_the_flat_fields_of_0_6_at_the_field() {
    let flat = [
        ("id", json!(5)),
        ("pr", json!(16)),
        ("p", json!(2)),
        ("c", json!(0)),
        ("r", json!(0)),
        ("value_bits", json!(7)),
        ("range", json!([0, 127])),
        ("transmit", json!("absolute")),
        ("recognize", json!("absolute")),
    ];
    for (field, value) in flat {
        let mut entry = json!({"osc": "/a", "ways": [sysex_id(5)]});
        entry[field] = value;
        refused(&params_doc(vec![entry]), FaultKind::UnknownField, &format!("/params/entries/0/{field}"));
    }
}

#[test]
fn the_entry_keeps_what_is_the_controls_own() {
    let mut d = one(json!({
        "osc": "/a", "default": 5, "priority": 2, "step_count": 8, "expects": "continuous",
        "section": "Filter", "transform": "x", "channel": 3, "title": "T", "description": "D", "scale": [0, 127],
        "values": [{"value": 0, "title": "off"}], "ways": [way("cc", json!([7]))]
    }));
    d["capabilities"] = json!(["lua"]);
    ok(&d);
    assert_eq!(written(&d), d);
}

// ---- value_bits

#[test]
fn an_nrpn_or_an_rpn_way_needs_value_bits_of_7_or_14() {
    for kind in ["nrpn", "rpn"] {
        refused(&one(json!({"osc": "/a", "ways": [way(kind, json!([1, 2]))]})), FaultKind::Missing, &format!("{}/value_bits", way_at(0, 0)));
        for width in [json!(8), json!(0), json!(16), json!("14")] {
            bad(&one(json!({"osc": "/a", "ways": [with(way(kind, json!([1, 2])), "value_bits", width)]})), &format!("{}/value_bits", way_at(0, 0)));
        }
        for width in [7, 14] {
            ok(&one(json!({"osc": "/a", "ways": [with(way(kind, json!([1, 2])), "value_bits", json!(width))]})));
        }
    }
}

#[test]
fn a_cc_way_of_two_numbers_needs_value_bits_14() {
    refused(&one(json!({"osc": "/a", "ways": [way("cc", json!([1, 33]))]})), FaultKind::Missing, &format!("{}/value_bits", way_at(0, 0)));
    bad(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([1, 33])), "value_bits", json!(7))]})), &format!("{}/value_bits", way_at(0, 0)));
    bad(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([1, 33])), "value_bits", json!(8))]})), &format!("{}/value_bits", way_at(0, 0)));
    ok(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([1, 33])), "value_bits", json!(14))]})));
}

#[test]
fn a_simple_cc_way_may_omit_value_bits_or_give_7_and_no_other_width() {
    ok(&one(json!({"osc": "/a", "ways": [way("cc", json!([7]))]})));
    ok(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([7])), "value_bits", json!(7))]})));
    for width in [json!(14), json!(8)] {
        bad(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([7])), "value_bits", width)]})), &format!("{}/value_bits", way_at(0, 0)));
    }
}

#[test]
fn a_pressure_or_a_note_way_carries_no_value_bits_and_a_pitch_bend_may_give_14() {
    for (kind, index) in [("chPress", None), ("pPress", Some(60)), ("note", Some(36))] {
        for width in [7, 14] {
            let mut w = json!({"ctrlType": kind, "value_bits": width});
            if let Some(n) = index {
                w["ctrlIndex"] = json!([n]);
            }
            bad(&one(json!({"osc": "/a", "ways": [w]})), &format!("{}/value_bits", way_at(0, 0)));
        }
    }
    ok(&one(json!({"osc": "/a", "ways": [{"ctrlType": "pBend"}]})));
    ok(&one(json!({"osc": "/a", "ways": [{"ctrlType": "pBend", "value_bits": 14}]})));
    bad(&one(json!({"osc": "/a", "ways": [{"ctrlType": "pBend", "value_bits": 7}]})), &format!("{}/value_bits", way_at(0, 0)));
}

#[test]
fn the_value_bits_rules_hold_on_every_way_of_a_control_at_the_way() {
    refused(
        &one(json!({"osc": "/a", "ways": [way("cc", json!([7])), way("nrpn", json!([1, 2]))]})),
        FaultKind::Missing,
        &format!("{}/value_bits", way_at(0, 1)),
    );
}

// ---- the kinds a control and a params entry take

#[test]
fn a_sysex_way_is_for_a_params_entry_and_the_kinds_of_midi_are_for_a_control() {
    bad(&one(json!({"osc": "/a", "ways": [{"ctrlType": "sysex", "pr": 16, "p": 2, "c": 0, "r": 0}]})), &format!("{}/ctrlType", way_at(0, 0)));
    for (kind, index) in [("cc", json!([7])), ("nrpn", json!([1, 2])), ("rpn", json!([1, 2])), ("pPress", json!([60])), ("note", json!([36])), ("chPress", Value::Null), ("pBend", Value::Null)] {
        let mut w = json!({"ctrlType": kind, "value_bits": 14});
        if kind == "cc" {
            w = way("cc", index.clone());
        } else if index != Value::Null {
            w["ctrlIndex"] = index;
        }
        if matches!(kind, "pPress" | "note" | "chPress") {
            w.as_object_mut().unwrap().remove("value_bits");
        }
        bad(&params_doc(vec![json!({"osc": "/a", "ways": [w]})]), "/params/entries/0/ways/0/ctrlType");
    }
}

#[test]
fn a_params_entry_has_sysex_ways_with_the_numbers_of_its_frame_range_width_and_direction() {
    let d = params_doc(vec![
        json!({"osc": "/a", "default": 3, "ways": [
            {"ctrlType": "sysex", "pr": 16, "p": 2, "c": 0, "r": 0, "range": [0, 100], "value_bits": 7, "transmit": "absolute", "recognize": "both"}
        ]}),
        json!({"osc": "/b", "ways": [sysex_id(5)]}),
    ]);
    ok(&d);
    assert_eq!(written(&d), d);
    let entries = device(&d).params.unwrap().entries;
    let ways: Vec<Vec<_>> = entries.iter().map(|e| e.ways().iter().map(sysex_shape).collect()).collect();
    assert_eq!(ways, vec![vec![("sysex".to_string(), None, [Some(16), Some(2), Some(0), Some(0), None])], vec![("sysex".to_string(), None, [None, None, None, None, Some(5)])]]);
}

// ---- the message claimed once, by side

#[test]
fn two_controls_with_the_same_cc_and_no_direction_are_refused_at_the_later_way_naming_the_earlier() {
    let d = cc_doc(vec![json!({"osc": "/a", "ways": [way("cc", json!([7]))]}), json!({"osc": "/b", "ways": [way("cc", json!([7]))]})]);
    bad_naming(&d, &way_at(1, 0), &way_at(0, 0));
}

#[test]
fn two_ways_of_one_control_that_claim_the_same_message_are_refused_naming_the_earlier_way() {
    let d = one(json!({"osc": "/a", "ways": [way("cc", json!([74])), way("nrpn", json!([1, 8])), way("cc", json!([74]))]}));
    let d = with_bits(d);
    bad_naming(&d, &way_at(0, 2), &way_at(0, 0));
}

fn with_bits(mut doc: Value) -> Value {
    doc["cc_params"]["entries"][0]["ways"][1]["value_bits"] = json!(14);
    doc
}

#[test]
fn the_later_way_of_another_control_names_the_earlier_way_of_the_first_control_whichever_its_rank() {
    let d = cc_doc(vec![
        json!({"osc": "/a", "ways": [way("cc", json!([1])), way("cc", json!([2]))]}),
        json!({"osc": "/b", "ways": [way("cc", json!([3])), way("cc", json!([2]))]}),
    ]);
    bad_naming(&d, &way_at(1, 1), &way_at(0, 1));
}

#[test]
fn a_way_that_receives_only_and_a_way_that_sends_only_of_one_cc_are_admitted() {
    let d = one(json!({"osc": "/a", "ways": [
        with(with(way("cc", json!([7])), "transmit", json!("absolute")), "recognize", json!("none")),
        with(with(way("cc", json!([7])), "transmit", json!("none")), "recognize", json!("absolute"))
    ]}));
    ok(&d);
}

#[test]
fn two_ways_that_send_the_same_cc_are_refused_on_the_sender_side() {
    let send = || with(with(way("cc", json!([7])), "transmit", json!("absolute")), "recognize", json!("none"));
    bad_naming(&one(json!({"osc": "/a", "ways": [send(), send()]})), &way_at(0, 1), &way_at(0, 0));
}

#[test]
fn two_ways_that_recognise_the_same_cc_are_refused_on_the_receiver_side() {
    let recv = || with(with(way("cc", json!([7])), "transmit", json!("none")), "recognize", json!("both"));
    bad_naming(&one(json!({"osc": "/a", "ways": [recv(), recv()]})), &way_at(0, 1), &way_at(0, 0));
}

#[test]
fn a_way_with_neither_direction_claims_both_sides() {
    let neither = || way("cc", json!([7]));
    bad(&one(json!({"osc": "/a", "ways": [neither(), neither()]})), &way_at(0, 1));
    let sends = with(with(way("cc", json!([7])), "transmit", json!("absolute")), "recognize", json!("none"));
    bad(&one(json!({"osc": "/a", "ways": [neither(), sends.clone()]})), &way_at(0, 1));
    bad(&one(json!({"osc": "/a", "ways": [sends, neither()]})), &way_at(0, 1));
    let recv = with(with(way("cc", json!([7])), "transmit", json!("none")), "recognize", json!("absolute"));
    bad(&one(json!({"osc": "/a", "ways": [recv, neither()]})), &way_at(0, 1));
}

#[test]
fn only_an_explicit_none_removes_a_side_a_missing_word_is_not_none() {
    // `transmit` written, `recognize` absent: it recognises as well, so it meets a way that recognises.
    let sends_only_written = with(way("cc", json!([7])), "transmit", json!("none"));
    let recv = with(with(way("cc", json!([7])), "transmit", json!("none")), "recognize", json!("absolute"));
    bad(&one(json!({"osc": "/a", "ways": [sends_only_written, recv]})), &way_at(0, 1));
}

#[test]
fn a_way_with_both_directions_at_none_claims_nothing() {
    let silent = || with(with(way("cc", json!([7])), "transmit", json!("none")), "recognize", json!("none"));
    ok(&one(json!({"osc": "/a", "ways": [silent(), silent()]})));
    ok(&one(json!({"osc": "/a", "ways": [silent(), way("cc", json!([7]))]})));
    ok(&cc_doc(vec![json!({"osc": "/a", "ways": [way("cc", json!([7]))]}), json!({"osc": "/b", "ways": [silent()]})]));
}

#[test]
fn a_14_bit_cc_way_claims_its_msb_and_its_lsb_so_it_meets_a_simple_cc_on_either_number() {
    let fourteen = |msb: i64, lsb: i64| with(way("cc", json!([msb, lsb])), "value_bits", json!(14));
    for simple in [7, 39] {
        let d = one(json!({"osc": "/a", "ways": [fourteen(7, 39), way("cc", json!([simple]))]}));
        bad_naming(&d, &way_at(0, 1), &way_at(0, 0));
        let d = one(json!({"osc": "/a", "ways": [way("cc", json!([simple])), fourteen(7, 39)]}));
        bad_naming(&d, &way_at(0, 1), &way_at(0, 0));
    }
    // A [7, 39] against a [39] collides by the number 39 alone.
    let d = cc_doc(vec![json!({"osc": "/a", "ways": [way("cc", json!([39]))]}), json!({"osc": "/b", "ways": [fourteen(7, 39)]})]);
    bad_naming(&d, &way_at(1, 0), &way_at(0, 0));
    let d = cc_doc(vec![json!({"osc": "/a", "ways": [fourteen(1, 33)]}), json!({"osc": "/b", "ways": [fourteen(2, 33)]})]);
    bad_naming(&d, &way_at(1, 0), &way_at(0, 0));
    ok(&cc_doc(vec![json!({"osc": "/a", "ways": [fourteen(1, 33)]}), json!({"osc": "/b", "ways": [fourteen(2, 34)]})]));
}

#[test]
fn the_same_numbers_of_another_kind_are_another_message() {
    let d = cc_doc(vec![
        json!({"osc": "/a", "ways": [way("cc", json!([1]))]}),
        json!({"osc": "/b", "ways": [with(way("nrpn", json!([1, 1])), "value_bits", json!(14))]}),
        json!({"osc": "/c", "ways": [with(way("rpn", json!([1, 1])), "value_bits", json!(14))]}),
        json!({"osc": "/d", "ways": [way("note", json!([1]))]}),
        json!({"osc": "/e", "ways": [way("pPress", json!([1]))]}),
    ]);
    ok(&d);
}

#[test]
fn a_note_and_a_polyphonic_pressure_of_one_number_do_not_claim_each_other() {
    ok(&cc_doc(vec![json!({"osc": "/a", "ways": [way("note", json!([60]))]}), json!({"osc": "/b", "ways": [way("pPress", json!([60]))]})]));
    bad_naming(
        &cc_doc(vec![json!({"osc": "/a", "ways": [way("note", json!([60]))]}), json!({"osc": "/b", "ways": [way("note", json!([60]))]})]),
        &way_at(1, 0),
        &way_at(0, 0),
    );
    bad_naming(
        &cc_doc(vec![json!({"osc": "/a", "ways": [way("pPress", json!([60]))]}), json!({"osc": "/b", "ways": [way("pPress", json!([60]))]})]),
        &way_at(1, 0),
        &way_at(0, 0),
    );
}

#[test]
fn a_channel_pressure_and_a_pitch_bend_are_one_message_per_channel() {
    for kind in ["chPress", "pBend"] {
        let w = || json!({"ctrlType": kind});
        bad_naming(&cc_doc(vec![json!({"osc": "/a", "ways": [w()]}), json!({"osc": "/b", "ways": [w()]})]), &way_at(1, 0), &way_at(0, 0));
        ok(&cc_doc(vec![json!({"osc": "/a", "channel": 1, "ways": [w()]}), json!({"osc": "/b", "channel": 2, "ways": [w()]})]));
    }
}

#[test]
fn the_same_message_on_another_channel_is_another_message() {
    ok(&cc_doc(vec![json!({"osc": "/a", "channel": 1, "ways": [way("cc", json!([74]))]}), json!({"osc": "/b", "channel": 2, "ways": [way("cc", json!([74]))]})]));
    bad(
        &cc_doc(vec![json!({"osc": "/a", "channel": 2, "ways": [way("cc", json!([74]))]}), json!({"osc": "/b", "channel": 2, "ways": [way("cc", json!([74]))]})]),
        &way_at(1, 0),
    );
}

// ---- the scale of a control

fn entry_of(doc: &Value) -> osd::CcEntry {
    cc_entries(doc).remove(0)
}

fn nrpn14() -> Value {
    with(way("nrpn", json!([1, 8])), "value_bits", json!(14))
}

#[test]
fn the_reference_scale_is_the_range_of_the_most_precise_way() {
    let e = entry_of(&one(json!({"osc": "/a", "ways": [way("cc", json!([74])), nrpn14()]})));
    assert_eq!(e.reference_scale(), [0, 16383]);
    let e = entry_of(&one(json!({"osc": "/a", "ways": [nrpn14(), way("cc", json!([74]))]})));
    assert_eq!(e.reference_scale(), [0, 16383]);
    let e = entry_of(&one(json!({"osc": "/a", "ways": [with(way("cc", json!([74])), "range", json!([0, 100])), with(nrpn14(), "range", json!([0, 1000]))]})));
    assert_eq!(e.reference_scale(), [0, 1000]);
    let e = entry_of(&one(json!({"osc": "/a", "ways": [way("cc", json!([74]))]})));
    assert_eq!(e.reference_scale(), [0, 127]);
}

#[test]
fn the_reference_scale_of_two_ways_with_the_same_highest_maximum_is_the_first_ones() {
    let e = entry_of(&one(json!({"osc": "/a", "ways": [
        with(way("cc", json!([1])), "range", json!([5, 127])),
        with(way("cc", json!([2])), "range", json!([0, 127]))
    ]})));
    assert_eq!(e.reference_scale(), [5, 127]);
    let e = entry_of(&one(json!({"osc": "/a", "ways": [
        with(way("cc", json!([2])), "range", json!([0, 127])),
        with(way("cc", json!([1])), "range", json!([5, 127]))
    ]})));
    assert_eq!(e.reference_scale(), [0, 127]);
}

#[test]
fn the_scale_a_control_gives_is_its_reference_scale_even_when_a_way_is_more_precise() {
    let e = entry_of(&one(json!({"osc": "/a", "scale": [0, 127], "ways": [way("cc", json!([74])), nrpn14()]})));
    assert_eq!(e.reference_scale(), [0, 127]);
    assert_eq!(e.scale, Some([0, 127]));
    let e = entry_of(&one(json!({"osc": "/a", "ways": [way("cc", json!([74])), nrpn14()]})));
    assert_eq!(e.scale, None, "a scale that was not written is not invented by the reader");
}

#[test]
fn the_scale_of_a_way_without_a_range_is_the_one_its_kind_and_width_imply() {
    let e = entry_of(&one(json!({"osc": "/a", "ways": [way("cc", json!([74])), nrpn14()], "default": 8192})));
    let [cc, nrpn] = <[osd::Way; 2]>::try_from(e.ways()).unwrap();
    assert_eq!(e.in_way(8192, &cc), 64, "8192 × 127 / 16383 is 63.5, the half goes up");
    assert_eq!(e.in_way(8192, &nrpn), 8192);
}

#[test]
fn a_value_is_scaled_to_a_way_rounded_to_the_nearest_the_half_upward_never_truncated() {
    let e = entry_of(&one(json!({"osc": "/a", "scale": [0, 16383], "ways": [way("cc", json!([74])), nrpn14()]})));
    let cc = e.ways().remove(0);
    let nrpn = e.ways().remove(1);
    let got: Vec<i64> = [0, 1, 100, 8191, 8192, 16383].iter().map(|v| e.in_way(*v, &cc)).collect();
    assert_eq!(got, vec![0, 0, 1, 63, 64, 127]);
    let got: Vec<i64> = [0, 1, 8192, 16383].iter().map(|v| e.in_way(*v, &nrpn)).collect();
    assert_eq!(got, vec![0, 1, 8192, 16383]);
    // From a coarse reference to a finer way: proportional, no loss at the ends.
    let e = entry_of(&one(json!({"osc": "/a", "scale": [0, 127], "ways": [way("cc", json!([74])), nrpn14()]})));
    let nrpn = e.ways().remove(1);
    assert_eq!((e.in_way(0, &nrpn), e.in_way(127, &nrpn), e.in_way(64, &nrpn)), (0, 16383, 8256));
}

#[test]
fn a_value_received_on_a_way_is_scaled_to_the_reference_scale_in_the_same_manner() {
    let e = entry_of(&one(json!({"osc": "/a", "scale": [0, 16383], "ways": [way("cc", json!([74])), nrpn14()]})));
    let cc = e.ways().remove(0);
    let got: Vec<i64> = [0, 1, 64, 127].iter().map(|v| e.from_way(*v, &cc)).collect();
    assert_eq!(got, vec![0, 129, 8256, 16383]);
    let nrpn = e.ways().remove(1);
    assert_eq!(e.from_way(12345, &nrpn), 12345);
}

#[test]
fn a_scale_that_does_not_start_at_0_is_scaled_affinely_between_its_bounds() {
    let e = entry_of(&one(json!({"osc": "/a", "scale": [-64, 63], "ways": [with(way("cc", json!([74])), "range", json!([0, 127]))]})));
    let w = e.ways().remove(0);
    let got: Vec<i64> = [-64, -1, 0, 1, 63].iter().map(|v| e.in_way(*v, &w)).collect();
    assert_eq!(got, vec![0, 63, 64, 65, 127]);
    let back: Vec<i64> = [0, 63, 64, 65, 127].iter().map(|v| e.from_way(*v, &w)).collect();
    assert_eq!(back, vec![-64, -1, 0, 1, 63]);
}

#[test]
fn a_way_whose_range_does_not_start_at_0_gets_the_value_added_to_its_lowest() {
    let e = entry_of(&one(json!({"osc": "/a", "scale": [0, 127], "ways": [with(way("cc", json!([74])), "range", json!([1, 127]))]})));
    let w = e.ways().remove(0);
    let got: Vec<i64> = [0, 64, 127].iter().map(|v| e.in_way(*v, &w)).collect();
    assert_eq!(got, vec![1, 64, 127]);
    let back: Vec<i64> = [1, 64, 127].iter().map(|v| e.from_way(*v, &w)).collect();
    assert_eq!(back, vec![0, 64, 127], "63.5 goes up");
}

#[test]
fn the_half_goes_upward_on_a_reference_scale_below_zero_too() {
    // [-1, 0] on [0, 1]: -1 → 0, 0 → 1; [-2, 0] on [0, 1]: -1 is the half, it goes up to 1.
    let e = entry_of(&one(json!({"osc": "/a", "scale": [-2, 0], "ways": [with(way("cc", json!([74])), "range", json!([0, 1]))]})));
    let w = e.ways().remove(0);
    assert_eq!((e.in_way(-2, &w), e.in_way(-1, &w), e.in_way(0, &w)), (0, 1, 1));
}

#[test]
fn the_default_of_a_params_entry_is_scaled_to_its_sysex_way() {
    let d = params_doc(vec![json!({"osc": "/a", "scale": [0, 200], "ways": [with(sysex_id(5), "range", json!([0, 100]))]})]);
    let e = device(&d).params.unwrap().entries.remove(0);
    let w = e.ways().remove(0);
    assert_eq!((e.in_way(0, &w), e.in_way(100, &w), e.in_way(200, &w), e.from_way(50, &w)), (0, 50, 100, 100));
}

// ---- reading a 0.6 control gives the same shape

#[test]
fn a_control_of_0_6_reads_as_ways_of_the_one_shape() {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [
            {"osc": "/a", "cc": 1, "cc_lsb": 33, "range": [0, 16383], "transmit": "absolute",
             "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14}, {"cc": 5}]}
        ]}
    });
    let Descriptor::Device(d) = osd::read(&doc.to_string(), Kind::Device).unwrap() else { panic!("a device") };
    let ways = d.cc_params.unwrap().entries[0].ways();
    let seen: Vec<(String, Vec<i64>, Option<i64>)> = ways.iter().map(|w| (w.ctrl_type.clone(), w.ctrl_index.clone().unwrap(), w.value_bits)).collect();
    assert_eq!(
        seen,
        vec![("cc".to_string(), vec![1, 33], Some(14)), ("nrpn".to_string(), vec![1, 2], Some(14)), ("cc".to_string(), vec![5], None)]
    );
}

#[test]
fn a_param_entry_of_0_6_reads_as_one_sysex_way() {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [
            {"osc": "/a", "pr": 16, "p": 2, "c": 0, "r": 0, "range": [0, 100]},
            {"osc": "/b", "id": 5}
        ]}
    });
    let Descriptor::Device(d) = osd::read(&doc.to_string(), Kind::Device).unwrap() else { panic!("a device") };
    let entries = d.params.unwrap().entries;
    let first = entries[0].ways();
    assert_eq!((first.len(), sysex_shape(&first[0]), first[0].range), (1, ("sysex".to_string(), None, [Some(16), Some(2), Some(0), Some(0), None]), Some([0, 100])));
    assert_eq!(sysex_shape(&entries[1].ways()[0]), ("sysex".to_string(), None, [None, None, None, None, Some(5)]));
}

// ---- the converter from 0.6

fn convert(doc: &Value) -> Value {
    serde_json::from_str(&osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"))).unwrap()
}

fn v06_cc(entries: Vec<Value>) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": entries}
    })
}

fn v06_params(entries: Vec<Value>) -> Value {
    json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "device": {"name": "T", "osc_prefix": "/t"},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": entries}
    })
}

fn converted_entries(doc: &Value) -> Vec<Value> {
    convert(doc)["cc_params"]["entries"].as_array().unwrap().clone()
}

#[test]
fn the_converter_turns_the_flat_form_into_ways_in_the_order_written_with_no_loss() {
    let out = converted_entries(&v06_cc(vec![json!({
        "osc": "/a", "cc": 1, "cc_lsb": 33, "range": [0, 16383], "transmit": "absolute", "recognize": "both",
        "also": [
            {"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 1000]},
            {"rpn_msb": 0, "rpn_lsb": 2, "value_bits": 7},
            {"cc": 8}
        ]
    })]));
    assert_eq!(
        out[0]["ways"],
        json!([
            {"ctrlType": "cc", "ctrlIndex": [1, 33], "value_bits": 14, "range": [0, 16383], "transmit": "absolute", "recognize": "both"},
            {"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14, "range": [0, 1000], "transmit": "absolute", "recognize": "both"},
            {"ctrlType": "rpn", "ctrlIndex": [0, 2], "value_bits": 7, "transmit": "absolute", "recognize": "both"},
            {"ctrlType": "cc", "ctrlIndex": [8], "transmit": "absolute", "recognize": "both"}
        ])
    );
    for flat in ["cc", "cc_lsb", "range", "also", "transmit", "recognize", "value_bits", "nrpn_msb"] {
        assert!(out[0].get(flat).is_none(), "{flat} left on the entry: {}", out[0]);
    }
}

#[test]
fn the_converter_writes_transmit_and_recognize_on_a_way_only_when_the_entry_had_them() {
    let out = converted_entries(&v06_cc(vec![
        json!({"osc": "/a", "cc": 7}),
        json!({"osc": "/b", "cc": 8, "transmit": "relative", "also": [{"cc": 9}]}),
        json!({"osc": "/c", "cc": 10, "recognize": "none"}),
    ]));
    assert_eq!(out[0]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [7]}]));
    assert_eq!(out[1]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [8], "transmit": "relative"}, {"ctrlType": "cc", "ctrlIndex": [9], "transmit": "relative"}]));
    assert_eq!(out[2]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [10], "recognize": "none"}]));
}

#[test]
fn the_converter_keeps_nrpn_lsb_range_on_the_nrpn_way_and_the_fields_of_the_control_on_the_entry() {
    let mut doc = v06_cc(vec![json!({
        "osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "nrpn_lsb_range": [8, 15],
        "priority": 2, "expects": "continuous", "section": "S", "transform": "x", "channel": 4
    })]);
    doc["capabilities"] = json!(["lua"]);
    let out = converted_entries(&doc);
    assert_eq!(out[0]["ways"], json!([{"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14, "nrpn_lsb_range": [8, 15]}]));
    for (k, v) in [("priority", json!(2)), ("expects", json!("continuous")), ("section", json!("S")), ("transform", json!("x")), ("channel", json!(4))] {
        assert_eq!(out[0][k], v, "{k}");
    }
}

#[test]
fn the_converter_makes_values_a_list_of_value_and_title_in_the_order_of_the_dictionary() {
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "range": [0, 127], "values": {"closed": 0, "open": 127}})]));
    assert_eq!(out[0]["values"], json!([{"value": 0, "title": "closed"}, {"value": 127, "title": "open"}]));
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "values": {"Mid": 1, "Middle": 1}})]));
    assert_eq!(out[0]["values"], json!([{"value": 1, "title": "Mid"}, {"value": 1, "title": "Middle"}]));
}

#[test]
fn the_converter_never_joins_two_controls() {
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 1}), json!({"osc": "/a2", "cc": 2}), json!({"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 1, "value_bits": 14})]));
    assert_eq!(out.len(), 3);
    assert!(out.iter().all(|e| e["ways"].as_array().unwrap().len() == 1), "{out:?}");
}

#[test]
fn the_midi_guide_and_the_electra_examples_of_the_proposal_come_out_as_the_text_shows_them() {
    let guide = converted_entries(&v06_cc(vec![json!({"osc": "/filter/cutoff", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14}]})]));
    assert_eq!(guide[0]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [74]}, {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14}]));
    let relative = converted_entries(&v06_cc(vec![json!({"osc": "/encoder/1", "cc": 20, "transmit": "relative", "recognize": "relative"})]));
    assert_eq!(relative[0]["ways"], json!([{"ctrlType": "cc", "ctrlIndex": [20], "transmit": "relative", "recognize": "relative"}]));
    let worked = converted_entries(&v06_cc(vec![json!({
        "osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "transmit": "absolute", "recognize": "both",
        "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383]}]
    })]));
    assert_eq!(
        worked[0],
        json!({"osc": "/filter/cutoff", "ways": [
            {"ctrlType": "cc", "ctrlIndex": [74], "range": [0, 127], "transmit": "absolute", "recognize": "both"},
            {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14, "range": [0, 16383], "transmit": "absolute", "recognize": "both"}
        ]})
    );
}

#[test]
fn the_converter_writes_a_sysex_way_for_a_params_entry_and_keeps_the_rest_on_the_entry() {
    let out = convert(&v06_params(vec![
        json!({"osc": "/a", "pr": 16, "p": 2, "c": 0, "r": 1, "range": [0, 100], "transmit": "absolute", "recognize": "both", "default": 5, "values": {"a": 1}, "priority": 3}),
        json!({"osc": "/b", "id": 5}),
        json!({"osc": "/c", "pr": 16, "p": 2}),
    ]));
    let entries = out["params"]["entries"].as_array().unwrap();
    assert_eq!(
        entries[0],
        json!({"osc": "/a", "default": 5, "values": [{"value": 1, "title": "a"}], "priority": 3, "ways": [
            {"ctrlType": "sysex", "pr": 16, "p": 2, "c": 0, "r": 1, "range": [0, 100], "transmit": "absolute", "recognize": "both"}
        ]})
    );
    assert_eq!(entries[1], json!({"osc": "/b", "ways": [{"ctrlType": "sysex", "id": 5}]}));
    assert_eq!(entries[2]["ways"], json!([{"ctrlType": "sysex", "pr": 16, "p": 2}]));
}

#[test]
fn the_converter_writes_scale_as_the_range_of_the_first_way_when_a_later_way_is_more_precise() {
    // `default` 64 on a 7-bit CC, then a 14-bit NRPN: the first way is not the most precise.
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "default": 64, "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14}]})]));
    assert_eq!(out[0]["scale"], json!([0, 127]));
    assert_eq!(out[0]["default"], 64, "the default is not rewritten");
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "range": [0, 100], "default": 50, "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383]}]})]));
    assert_eq!((out[0]["scale"].clone(), out[0]["default"].clone()), (json!([0, 100]), json!(50)));
}

#[test]
fn the_converter_writes_scale_for_values_or_a_step_count_too_and_leaves_them_unchanged() {
    let later = || json!([{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14}]);
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "values": {"off": 0, "on": 127}, "also": later()})]));
    assert_eq!(out[0]["scale"], json!([0, 127]));
    assert_eq!(out[0]["values"], json!([{"value": 0, "title": "off"}, {"value": 127, "title": "on"}]));
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "step_count": 8, "also": later()})]));
    assert_eq!((out[0]["scale"].clone(), out[0]["step_count"].clone()), (json!([0, 127]), json!(8)));
}

#[test]
fn the_converter_writes_no_scale_without_default_values_or_step_count() {
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14}]})]));
    assert!(out[0].get("scale").is_none(), "{}", out[0]);
}

#[test]
fn the_converter_writes_no_scale_when_the_first_way_is_the_most_precise_or_ties() {
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "default": 8192, "also": [{"cc": 74}]})]));
    assert!(out[0].get("scale").is_none(), "{}", out[0]);
    assert_eq!(out[0]["default"], 8192);
    // The same highest maximum: the first stays the reference and nothing is written.
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "range": [0, 127], "default": 64, "also": [{"cc": 75, "range": [5, 127]}]})]));
    assert!(out[0].get("scale").is_none(), "{}", out[0]);
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "cc": 74, "default": 64})]));
    assert!(out[0].get("scale").is_none(), "{}", out[0]);
}

#[test]
fn the_converter_writes_the_scale_of_an_nrpn_first_way_from_its_width_when_it_has_no_range() {
    let out = converted_entries(&v06_cc(vec![json!({"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 7, "default": 5, "also": [{"cc": 1, "cc_lsb": 33}]})]));
    assert_eq!(out[0]["scale"], json!([0, 127]));
}

#[test]
fn what_the_converter_writes_reads_back_and_converts_to_itself() {
    let text = v06_cc(vec![json!({
        "osc": "/a", "cc": 74, "default": 64, "values": {"off": 0}, "transmit": "absolute",
        "also": [{"nrpn_msb": 1, "nrpn_lsb": 8, "value_bits": 14, "range": [0, 16383]}]
    })])
    .to_string();
    let out = osd::convert_to(&text, Kind::Device, Version::V0_7).unwrap();
    osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(osd::convert_to(&out, Kind::Device, Version::V0_7).unwrap(), out);
}

#[test]
fn the_converter_never_writes_a_text_its_reader_refuses_a_message_claimed_twice_is_not_convertible() {
    let text = v06_cc(vec![json!({"osc": "/a", "cc": 74}), json!({"osc": "/b", "cc": 74})]).to_string();
    // 0.6 already refuses it: the refusal is at the same place.
    assert!(osd::convert_to(&text, Kind::Device, Version::V0_7).is_err());
}
