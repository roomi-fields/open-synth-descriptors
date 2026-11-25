//! Contexts of 0.7 (`spec/0.7.md`, proposal 0003): a device declares `contexts`, a way of `cc_params`
//! carries `in` and `slice`, and two claims of one message are refused only when they can hold
//! together: no coordinate named by both has two sets with nothing in common.

use osd::{Descriptor, FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.7/device.schema.json";
const SCHEMA_06: &str = "https://w3id.org/osd/0.6/device.schema.json";

fn read(doc: &Value) -> Result<Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

fn declared() -> Value {
    json!({
        "channel_setting": {"values": [{"id": "single"}, {"id": "multi"}]},
        "part": {"values": [{"id": "track1"}, {"id": "track2"}]}
    })
}

/// A device with the contexts of the worked example and the ways given, one control per way.
fn device(ways: Vec<Value>) -> Value {
    let entries: Vec<Value> = ways.into_iter().enumerate().map(|(i, w)| json!({"osc": format!("/c{i}"), "ways": [w]})).collect();
    json!({
        "$schema": SCHEMA,
        "device": {"name": "T", "osc_prefix": "/t"},
        "contexts": declared(),
        "cc_params": {"entries": entries}
    })
}

fn nrpn(extra: Value) -> Value {
    let mut w = json!({"ctrlType": "nrpn", "ctrlIndex": [0, 122], "value_bits": 7});
    for (k, v) in extra.as_object().unwrap() {
        w[k] = v.clone();
    }
    w
}

fn fault(doc: &Value) -> (FaultKind, String) {
    let f = read(doc).expect_err(&format!("refused: {doc}"));
    (f.kind, f.pointer)
}

fn bad(doc: &Value, pointer: &str) {
    assert_eq!(fault(doc), (FaultKind::BadValue, pointer.to_string()));
}

/// The seven controls of the worked example of 0003, on the NRPN 0:122, by letter.
fn control(letter: char) -> Value {
    match letter {
        'a' => nrpn(json!({"in": {"channel_setting": "single"}})),
        'b' => nrpn(json!({"in": {"channel_setting": "multi"}})),
        'c' => nrpn(json!({"in": {"part": "track1"}})),
        'd' => nrpn(json!({"in": {"part": "track2"}})),
        'e' => nrpn(json!({"slice": [0, 63]})),
        'f' => nrpn(json!({"slice": [64, 127]})),
        'g' => nrpn(json!({})),
        _ => panic!("{letter}"),
    }
}

#[test]
fn of_the_twenty_one_pairs_of_the_worked_example_three_are_admitted_and_eighteen_refused() {
    let letters = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];
    let admitted = [('a', 'b'), ('c', 'd'), ('e', 'f')];
    let mut wrong = Vec::new();
    let mut count = 0;
    for (i, x) in letters.iter().enumerate() {
        for y in &letters[i + 1..] {
            count += 1;
            let doc = device(vec![control(*x), control(*y)]);
            let got = read(&doc);
            let want_ok = admitted.contains(&(*x, *y));
            match (got, want_ok) {
                (Ok(_), true) => {}
                (Err(f), false) if f.kind == FaultKind::BadValue && f.pointer == "/cc_params/entries/1/ways/0" => {}
                (got, _) => wrong.push(format!("{x}-{y}: admitted {want_ok}, got {:?}", got.map(|_| ()).map_err(|f| (f.kind, f.pointer)))),
            }
        }
    }
    assert_eq!(count, 21);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn the_order_of_two_claims_does_not_change_the_verdict() {
    for (x, y) in [('b', 'a'), ('d', 'c'), ('f', 'e')] {
        read(&device(vec![control(x), control(y)])).unwrap_or_else(|f| panic!("{x}-{y}: {f}"));
    }
    for (x, y) in [('c', 'a'), ('g', 'a'), ('e', 'c')] {
        bad(&device(vec![control(x), control(y)]), "/cc_params/entries/1/ways/0");
    }
}

#[test]
fn a_part_that_names_a_mode_too_is_separated_from_the_other_mode_and_refused_against_its_own() {
    let c_multi = nrpn(json!({"in": {"part": "track1", "channel_setting": "multi"}}));
    read(&device(vec![control('a'), c_multi.clone()])).unwrap();
    bad(&device(vec![control('b'), c_multi]), "/cc_params/entries/1/ways/0");
}

#[test]
fn a_dimension_named_by_one_way_only_leaves_the_two_together() {
    let both = nrpn(json!({"in": {"part": "track1", "channel_setting": "single"}}));
    bad(&device(vec![both, control('c')]), "/cc_params/entries/1/ways/0");
}

#[test]
fn two_claims_that_differ_on_one_shared_coordinate_are_separated_whatever_the_others_are() {
    let x = nrpn(json!({"in": {"part": "track1", "channel_setting": "single"}}));
    let y = nrpn(json!({"in": {"part": "track1", "channel_setting": "multi"}}));
    read(&device(vec![x, y])).unwrap();
}

#[test]
fn messages_that_differ_are_never_in_conflict_whatever_their_contexts() {
    let other = json!({"ctrlType": "nrpn", "ctrlIndex": [0, 123], "value_bits": 7});
    read(&device(vec![control('g'), other])).unwrap();
}

#[test]
fn two_slices_that_touch_by_a_bound_meet_and_the_next_integer_separates() {
    bad(&device(vec![nrpn(json!({"slice": [0, 63]})), nrpn(json!({"slice": [63, 127]}))]), "/cc_params/entries/1/ways/0");
    read(&device(vec![nrpn(json!({"slice": [0, 63]})), nrpn(json!({"slice": [64, 127]}))])).unwrap();
}

#[test]
fn a_slice_inside_another_slice_meets_it() {
    bad(&device(vec![nrpn(json!({"slice": [0, 127]})), nrpn(json!({"slice": [10, 20]}))]), "/cc_params/entries/1/ways/0");
}

#[test]
fn a_slice_is_not_checked_against_the_bits_or_the_range_of_the_message() {
    read(&device(vec![nrpn(json!({"slice": [0, 500], "range": [0, 10]}))])).unwrap();
    read(&device(vec![nrpn(json!({"slice": [-5, 3]}))])).unwrap();
}

#[test]
fn a_slice_of_a_way_is_read_back_with_its_bounds_and_an_in_with_its_ids() {
    let doc = device(vec![nrpn(json!({"slice": [0, 63], "in": {"part": ["track1", "track2"], "channel_setting": "multi"}}))]);
    let back: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(back, doc);
}

#[test]
fn a_part_whose_channel_is_the_users_setting_is_trusted_to_separate() {
    read(&device(vec![control('c'), control('d')])).unwrap();
    let mut doc = device(vec![control('c'), control('d')]);
    doc["contexts"]["part"]["values"][0]["channel"] = json!(0);
    read(&doc).unwrap();
}

#[test]
fn two_values_of_one_dimension_on_the_same_channel_are_refused_at_the_second() {
    let mut doc = device(vec![control('c')]);
    doc["contexts"]["part"]["values"] = json!([{"id": "track1", "channel": 3}, {"id": "track2", "channel": 3}]);
    bad(&doc, "/contexts/part/values/1/channel");
}

#[test]
fn values_without_a_channel_are_never_checked_against_each_other() {
    let mut doc = device(vec![control('c')]);
    doc["contexts"]["part"]["values"] = json!([{"id": "track1"}, {"id": "track2"}, {"id": "track3"}]);
    read(&doc).unwrap();
    doc["contexts"]["part"]["values"] = json!([{"id": "track1", "channel": 3}, {"id": "track2"}, {"id": "track3"}]);
    read(&doc).unwrap();
}

#[test]
fn a_context_value_without_an_id_or_with_a_channel_out_of_range_is_refused_at_its_pointer() {
    let mut doc = device(vec![control('c')]);
    doc["contexts"]["part"]["values"] = json!([{"channel": 1}]);
    bad(&doc, "/contexts/part/values/0/id");
    for channel in [json!(16), json!(-1), json!("1"), json!(1.5)] {
        doc["contexts"]["part"]["values"] = json!([{"id": "track1", "channel": channel}]);
        bad(&doc, "/contexts/part/values/0/channel");
    }
    for channel in [0, 15] {
        doc["contexts"]["part"]["values"] = json!([{"id": "track1", "channel": channel}]);
        read(&doc).unwrap();
    }
}

#[test]
fn an_undeclared_dimension_or_value_in_in_is_refused_at_its_pointer() {
    let way = "/cc_params/entries/0/ways/0";
    bad(&device(vec![nrpn(json!({"in": {"part": "track9"}}))]), &format!("{way}/in/part"));
    bad(&device(vec![nrpn(json!({"in": {"track": "track1"}}))]), &format!("{way}/in/track"));
    bad(&device(vec![nrpn(json!({"in": {"part": ["track1", "track9"]}}))]), &format!("{way}/in/part/1"));
    bad(&device(vec![nrpn(json!({"in": {"channel_setting": "track1"}}))]), &format!("{way}/in/channel_setting"));
}

#[test]
fn an_in_of_a_wrong_form_is_refused_at_its_pointer() {
    let way = "/cc_params/entries/0/ways/0";
    bad(&device(vec![nrpn(json!({"in": "part"}))]), &format!("{way}/in"));
    bad(&device(vec![nrpn(json!({"in": ["part"]}))]), &format!("{way}/in"));
    bad(&device(vec![nrpn(json!({"in": {"part": 1}}))]), &format!("{way}/in/part"));
    bad(&device(vec![nrpn(json!({"in": {"part": {"id": "track1"}}}))]), &format!("{way}/in/part"));
}

#[test]
fn a_slice_that_is_not_a_closed_interval_of_integers_is_refused_at_its_pointer() {
    let at = "/cc_params/entries/0/ways/0/slice";
    for slice in [json!([64, 63]), json!([1]), json!([1, 2, 3]), json!("0-63"), json!({"low": 0, "high": 1}), json!([0, 1.5])] {
        bad(&device(vec![nrpn(json!({"slice": slice}))]), at);
    }
    read(&device(vec![nrpn(json!({"slice": [5, 5]}))])).unwrap();
}

#[test]
fn in_and_slice_on_a_way_of_params_are_unknown_fields() {
    let params = |w: Value| {
        json!({
            "$schema": SCHEMA,
            "device": {"name": "T", "osc_prefix": "/t"},
            "contexts": declared(),
            "params": {
                "get_frame": [240, 66, {"arg": "pr"}, {"arg": "p"}, {"arg": "c"}, {"arg": "r"}, 247],
                "set_frame": [240, 67, {"arg": "pr"}, {"arg": "p"}, {"arg": "c"}, {"arg": "r"}, {"arg": "value"}, 247],
                "value_type": "u7",
                "entries": [{"osc": "/a", "ways": [w]}]
            }
        })
    };
    let base = json!({"ctrlType": "sysex", "ctrlIndex": [16, 2, 0, 0]});
    let mut with_in = base.clone();
    with_in["in"] = json!({"part": "track1"});
    let mut with_slice = base.clone();
    with_slice["slice"] = json!([0, 63]);
    assert_eq!(fault(&params(with_in)), (FaultKind::UnknownField, "/params/entries/0/ways/0/in".into()));
    assert_eq!(fault(&params(with_slice)), (FaultKind::UnknownField, "/params/entries/0/ways/0/slice".into()));
    read(&params(base)).unwrap();
}

fn midi_of(ways: Vec<Value>, contexts: Option<Value>) -> Value {
    let mut doc = device(ways);
    doc["midi_of"] = json!("acme/b");
    match contexts {
        Some(c) => doc["contexts"] = c,
        None => {
            doc.as_object_mut().unwrap().remove("contexts");
        }
    }
    doc
}

#[test]
fn a_midi_of_file_read_alone_keeps_an_in_that_names_a_context_of_its_target() {
    let doc = midi_of(vec![nrpn(json!({"in": {"part": "track1", "mode": ["x", "y"]}}))], None);
    let back: Value = serde_json::from_str(&osd::write(&read(&doc).unwrap())).unwrap();
    assert_eq!(back, doc);
}

#[test]
fn a_midi_of_file_read_alone_still_checks_the_shape_of_in() {
    bad(&midi_of(vec![nrpn(json!({"in": {"part": 3}}))], None), "/cc_params/entries/0/ways/0/in/part");
    bad(&midi_of(vec![nrpn(json!({"in": "part"}))], None), "/cc_params/entries/0/ways/0/in");
}

#[test]
fn a_midi_of_file_read_alone_compares_ways_on_known_coordinates_only() {
    let p = |id: &str| nrpn(json!({"in": {"part": id}}));
    bad(&midi_of(vec![p("track1"), p("track2")], None), "/cc_params/entries/1/ways/0");
    read(&midi_of(vec![nrpn(json!({"slice": [0, 63]})), nrpn(json!({"slice": [64, 127]}))], None)).unwrap();
    let s = |slice: Value| nrpn(json!({"in": {"part": "track1"}, "slice": slice}));
    read(&midi_of(vec![s(json!([0, 63])), s(json!([64, 127]))], None)).unwrap();
    bad(&midi_of(vec![s(json!([0, 63])), s(json!([63, 127]))], None), "/cc_params/entries/1/ways/0");
}

#[test]
fn a_midi_of_file_with_its_own_contexts_is_checked_against_them() {
    bad(&midi_of(vec![nrpn(json!({"in": {"part": "track9"}}))], Some(declared())), "/cc_params/entries/0/ways/0/in/part");
    read(&midi_of(vec![control('c'), control('d')], Some(declared()))).unwrap();
}

#[test]
fn a_descriptor_of_0_6_keeps_the_rule_that_a_message_is_claimed_once() {
    let doc = json!({
        "$schema": SCHEMA_06,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"entries": [{"osc": "/a", "cc": 7}, {"osc": "/b", "cc": 7}]}
    });
    bad(&doc, "/cc_params/entries/1");
}

#[test]
fn a_descriptor_of_0_6_does_not_know_contexts_in_or_slice() {
    let mut doc = json!({
        "$schema": SCHEMA_06,
        "device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"entries": [{"osc": "/a", "cc": 7}]}
    });
    doc["contexts"] = declared();
    assert_eq!(fault(&doc).0, FaultKind::UnknownField);
}

#[test]
fn the_text_of_the_controls_states_the_one_rule_of_claims() {
    let text = std::fs::read_to_string(crate::common::repo().join("spec/0.7.md")).unwrap();
    assert!(!text.contains("of proposal 0003 are not read"), "the text still says that `in` and `slice` are not read");
    assert!(text.contains("coordinate"), "the text states no rule by coordinates");
    assert!(text.contains("`contexts`"));
}
