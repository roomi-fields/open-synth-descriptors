//! What the Electra One import writes reads with the reader of 0.7: a message a control already
//! claims is written once, and a control takes the items of its overlay that are values of the
//! scale of its way; what is left out is named in the losses.

use crate::common::electra::*;
use osd_formats::{import, Format, Loss};
use serde_json::{json, Value};

/// The imported device, which [`json_of`] reads with the reader of `osd`, and the losses.
fn imported(p: &Value) -> (Value, Vec<Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("the preset imports");
    (json_of(&t.value), t.lost)
}

/// The title of each control and the kind and numbers of its way, in the order written.
fn controls_of(d: &Value) -> Vec<(String, String, Value)> {
    let entries = d["cc_params"]["entries"].as_array().cloned().unwrap_or_default();
    entries
        .iter()
        .map(|e| {
            let way = &e["ways"][0];
            (e["title"].as_str().unwrap().to_string(), way["ctrlType"].as_str().unwrap().to_string(), way["ctrlIndex"].clone())
        })
        .collect()
}

fn titles_of(d: &Value) -> Vec<String> {
    controls_of(d).into_iter().map(|(title, _, _)| title).collect()
}

fn places(lost: &[Loss]) -> Vec<&str> {
    lost.iter().map(|l| l.place.as_str()).collect()
}

/// The places of the losses of a message and of an overlay.
fn claim_places(lost: &[Loss]) -> Vec<&str> {
    places(lost).into_iter().filter(|p| p.ends_with("/message") || p.starts_with("/overlays")).collect()
}

/// A preset of one control per message, named `C0`, `C1`…
fn preset_of(messages: Vec<Value>) -> Value {
    let controls = messages.into_iter().enumerate().map(|(n, m)| control(n as u64 + 1, &format!("C{n}"), None, m, None)).collect();
    preset(&[], controls)
}

fn relcc(n: u64) -> Value {
    json!({"type": "relcc", "deviceId": 1, "parameterNumber": n, "relative": "twosComplement"})
}

// ---- a message is claimed once ----------------------------------------------------------

#[test]
fn a_second_control_on_a_cc_already_claimed_is_not_written_and_its_message_is_named() {
    let (d, lost) = imported(&preset_of(vec![cc7(74), cc7(74), cc7(75), cc7(74)]));
    assert_eq!(titles_of(&d), ["C0", "C2"]);
    assert_eq!(claim_places(&lost), ["/controls/1/values/0/message", "/controls/3/values/0/message"]);
    assert!(lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn a_14_bit_cc_claims_its_two_numbers_and_a_relative_cc_its_number() {
    // The two messages, and whether the second is written.
    let cases = [
        (cc14(5), cc7(5), false),
        (cc14(5), cc7(37), false),
        (cc7(37), cc14(5), false),
        (cc7(5), cc14(5), false),
        (cc14(5), cc14(5), false),
        (cc14(5), relcc(37), false),
        (relcc(10), cc7(10), false),
        (cc7(10), relcc(10), false),
        (relcc(10), relcc(10), false),
        (cc14(5), cc7(6), true),
        (cc14(5), cc7(36), true),
        (cc14(5), cc14(6), true),
        (relcc(10), cc7(11), true),
    ];
    for (first, second, written) in cases {
        let (d, lost) = imported(&preset_of(vec![first.clone(), second.clone()]));
        let expected: &[&str] = if written { &["C0", "C1"] } else { &["C0"] };
        assert_eq!(titles_of(&d), expected, "{first} then {second}");
        let named: &[&str] = if written { &[] } else { &["/controls/1/values/0/message"] };
        assert_eq!(claim_places(&lost), named, "{first} then {second}");
    }
}

#[test]
fn an_nrpn_and_an_rpn_of_one_number_and_a_cc_are_three_messages_each_claimed_once() {
    let (d, lost) = imported(&preset_of(vec![nrpn(5), rpn(5), cc7(5), nrpn(5), rpn(5), nrpn(6), nrpn(133)]));
    assert_eq!(titles_of(&d), ["C0", "C1", "C2", "C5", "C6"]);
    assert_eq!(claim_places(&lost), ["/controls/3/values/0/message", "/controls/4/values/0/message"]);
}

#[test]
fn the_device_of_a_message_in_the_preset_does_not_tell_two_messages_apart_and_no_channel_is_written() {
    let mut other_device = cc7(74);
    other_device["deviceId"] = json!(2);
    let mut p = preset_of(vec![cc7(74), other_device]);
    p["devices"] = json!([{"id": 1, "name": "One", "port": 1, "channel": 1}, {"id": 2, "name": "Two", "port": 1, "channel": 2}]);
    let (d, lost) = imported(&p);
    assert_eq!(titles_of(&d), ["C0"]);
    assert_eq!(claim_places(&lost), ["/controls/1/values/0/message"]);
    assert!(d["cc_params"].get("channel").is_none() && d["cc_params"]["entries"][0].get("channel").is_none(), "{d}");
}

#[test]
fn a_control_left_out_claims_nothing() {
    // The 14-bit CC is left out for its first number: its second stays free.
    let (d, lost) = imported(&preset_of(vec![cc7(5), cc14(5), cc7(37)]));
    assert_eq!(titles_of(&d), ["C0", "C2"]);
    assert_eq!(claim_places(&lost), ["/controls/1/values/0/message"]);
}

#[test]
fn a_second_value_of_one_control_on_a_claimed_message_is_named_at_its_own_place() {
    let mut p = preset_of(vec![cc7(1), nrpn(9)]);
    p["controls"][0]["values"] = json!([{"id": "a", "message": cc7(1)}, {"id": "b", "message": cc7(1)}, {"id": "c", "message": cc7(2)}]);
    let (d, lost) = imported(&p);
    let numbers: Vec<Value> = controls_of(&d).into_iter().map(|(_, _, n)| n).collect();
    assert_eq!(numbers, [json!([1]), json!([2]), json!([0, 9])]);
    assert_eq!(claim_places(&lost), ["/controls/0/values/1/message"]);
}

#[test]
fn the_other_controls_and_the_commands_stay_when_a_message_is_claimed_twice() {
    let p = preset_of(vec![cc7(1), cc7(1), sysex(&[240, 1, 247]), nrpn(1), sysex(&[240, 1, 247])]);
    let (d, _) = imported(&p);
    assert_eq!(titles_of(&d), ["C0", "C3"]);
    let commands: Vec<&str> = d["commands"].as_array().unwrap().iter().map(|c| c["title"].as_str().unwrap()).collect();
    assert_eq!(commands, ["C2", "C4"]);
}

#[test]
fn an_overlay_only_a_control_left_out_refers_to_is_named_in_the_losses() {
    let mut p = preset_of(vec![cc7(1), cc7(1), cc7(1), cc7(2)]);
    let items = json!([{"value": 0, "label": "Off"}]);
    p["overlays"] = json!([{"id": 7, "items": items}, {"id": 8, "items": items}, {"id": 9, "items": items}]);
    // 7: a control written and one left out; 8: controls left out alone; 9: the last one written.
    for (control, overlay) in [(0, 7), (1, 8), (2, 7), (3, 9)] {
        p["controls"][control]["values"][0]["overlayId"] = json!(overlay);
    }
    let (d, lost) = imported(&p);
    assert_eq!(titles_of(&d), ["C0", "C3"]);
    let overlays: Vec<&str> = places(&lost).into_iter().filter(|p| p.starts_with("/overlays")).collect();
    assert_eq!(overlays, ["/overlays/1"], "{lost:?}");
}

// ---- the values of a control ------------------------------------------------------------

/// A preset of one control per `(message, range)`, named `C0`, `C1`…, the control `n` referring
/// to the overlay of index `overlay_of[n]`.
fn preset_with_overlays(controls: Vec<(Value, Option<(i64, i64)>)>, overlays: Vec<Value>, overlay_of: &[usize]) -> Value {
    let controls = controls.into_iter().enumerate().map(|(n, (m, range))| control(n as u64 + 1, &format!("C{n}"), None, m, range)).collect();
    let mut p = preset(&[], controls);
    p["overlays"] = overlays.into_iter().enumerate().map(|(k, items)| json!({"id": 10 + k, "items": items})).collect();
    for (n, k) in overlay_of.iter().enumerate() {
        p["controls"][n]["values"][0]["overlayId"] = json!(10 + k);
    }
    p
}

fn items(of: &[(i64, &str)]) -> Value {
    of.iter().map(|(value, label)| json!({"value": value, "label": label})).collect()
}

/// The `values` a control is to hold.
fn values(of: &[(i64, &str)]) -> Value {
    of.iter().map(|(value, title)| json!({"value": value, "title": title})).collect()
}

fn overlay_places(lost: &[Loss]) -> Vec<&str> {
    places(lost).into_iter().filter(|p| p.starts_with("/overlays")).collect()
}

#[test]
fn the_values_of_a_control_are_the_items_of_its_overlay_in_their_order_two_of_one_label_included() {
    let given = [(3, "Zed"), (0, "Ann"), (2, "Ann"), (1, "Mid"), (1, "Other")];
    let p = preset_with_overlays(vec![(cc7(9), Some((0, 3)))], vec![items(&given)], &[0]);
    let (d, lost) = imported(&p);
    assert_eq!(d["cc_params"]["entries"][0]["values"], values(&given));
    assert_eq!(overlay_places(&lost), Vec::<&str>::new());
}

#[test]
fn an_item_outside_the_scale_of_the_way_is_named_with_its_control_and_the_others_stay() {
    // The message, the range of the preset, the first and the last value of the scale.
    let cases = [
        (cc7(1), Some((10, 20)), 10, 20),
        (cc7(1), None, 0, 127),
        (cc14(1), None, 0, 16383),
        (cc14(1), Some((100, 200)), 100, 200),
        (nrpn(1), None, 0, 16383),
        (rpn(1), None, 0, 16383),
        (relcc(1), None, 0, 127),
        (nrpn(1), Some((-5, 5)), -5, 5),
    ];
    for (message, range, low, high) in cases {
        let given = [(low - 1, "below"), (high, "high"), (high + 1, "above"), (low, "low")];
        let p = preset_with_overlays(vec![(message.clone(), range)], vec![items(&given)], &[0]);
        let (d, lost) = imported(&p);
        let entry = &d["cc_params"]["entries"][0];
        assert_eq!(entry["values"], values(&[(high, "high"), (low, "low")]), "{message} {range:?}");
        assert_eq!(overlay_places(&lost), ["/overlays/0/items/0", "/overlays/0/items/2"], "{message} {range:?}");
        for l in lost.iter().filter(|l| l.place.starts_with("/overlays")) {
            assert!(l.reason.contains("C0"), "{message} {range:?}: the loss names the control: {}", l.reason);
        }
    }
}

#[test]
fn a_range_of_one_bound_leaves_the_scale_of_the_message_to_judge_the_values() {
    let mut p = preset_with_overlays(vec![(cc7(1), Some((10, 20)))], vec![items(&[(0, "a"), (127, "b"), (128, "c")])], &[0]);
    p["controls"][0]["values"][0].as_object_mut().unwrap().remove("max");
    let (d, lost) = imported(&p);
    assert_eq!(d["cc_params"]["entries"][0]["values"], values(&[(0, "a"), (127, "b")]));
    assert_eq!(overlay_places(&lost), ["/overlays/0/items/2"]);
}

#[test]
fn an_item_without_a_text_label_or_a_whole_value_is_named_with_its_control_and_the_others_stay() {
    let given = json!([
        {"value": 0, "label": "first"},
        {"value": 1},
        {"value": 2, "label": 7},
        {"value": 3, "label": null},
        {"label": "no value"},
        {"value": "4", "label": "text"},
        {"value": 4.5, "label": "fraction"},
        {"value": null, "label": "null"},
        "not an item",
        {"value": 5, "label": "last"},
    ]);
    let p = preset_with_overlays(vec![(cc7(1), None)], vec![given], &[0]);
    let (d, lost) = imported(&p);
    assert_eq!(d["cc_params"]["entries"][0]["values"], values(&[(0, "first"), (5, "last")]));
    let expected: Vec<String> = (1..=8).map(|i| format!("/overlays/0/items/{i}")).collect();
    assert_eq!(overlay_places(&lost), expected);
    for l in lost.iter().filter(|l| l.place.starts_with("/overlays")) {
        assert!(l.reason.contains("C0"), "the loss names the control: {}", l.reason);
    }
}

#[test]
fn a_control_of_which_no_value_remains_stays_without_values() {
    let p = preset_with_overlays(
        vec![(cc7(1), Some((0, 3))), (cc7(2), None), (cc7(3), None)],
        vec![items(&[(4, "a"), (9, "b")]), json!([{"value": 1}]), json!([])],
        &[0, 1, 2],
    );
    let (d, lost) = imported(&p);
    assert_eq!(titles_of(&d), ["C0", "C1", "C2"]);
    for n in 0..3 {
        assert!(d["cc_params"]["entries"][n].get("values").is_none(), "{d}");
    }
    assert_eq!(overlay_places(&lost), ["/overlays/0/items/0", "/overlays/0/items/1", "/overlays/1/items/0"]);
}

#[test]
fn an_overlay_two_controls_share_is_judged_on_the_scale_of_each() {
    let p = preset_with_overlays(
        vec![(cc7(1), Some((0, 1))), (nrpn(1), None)],
        vec![items(&[(0, "a"), (200, "b")])],
        &[0, 0],
    );
    let (d, lost) = imported(&p);
    assert_eq!(d["cc_params"]["entries"][0]["values"], values(&[(0, "a")]));
    assert_eq!(d["cc_params"]["entries"][1]["values"], values(&[(0, "a"), (200, "b")]));
    assert_eq!(overlay_places(&lost), ["/overlays/0/items/1"]);
    let reason = &lost.iter().find(|l| l.place == "/overlays/0/items/1").unwrap().reason;
    assert!(reason.contains("C0") && !reason.contains("C1"), "{reason}");
}
