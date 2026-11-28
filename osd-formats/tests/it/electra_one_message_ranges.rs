//! The range of a way and of the argument of a command is the MIDI range of the message of an
//! Electra One preset (page "Preset format description", section "Message", `min`, `max`,
//! `onValue` and `offValue`): the import reads it from the message, else from the value, and the
//! export writes it in each message (osd-formats/docs/CADRE.md R9, docs/CADRE.md R16).

use crate::common::electra::*;
use crate::common::fields::held;
use crate::common::resolved;
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

const VALUE: &str = "/controls/0/values/0";
const MESSAGE: &str = "/controls/0/values/0/message";

fn imported(p: &Value) -> (Value, Vec<Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("the preset imports");
    (json_of(&t.value), t.lost)
}

/// A preset of one control of this type: its message holding the keys of `message`, its value
/// those of `value`.
fn one(kind: &str, mut message: Value, extra_message: Value, value: Value) -> Value {
    for (k, v) in extra_message.as_object().unwrap() {
        message[k.as_str()] = v.clone();
    }
    let mut c = control(1, "Send", None, message, None);
    c["type"] = json!(kind);
    for (k, v) in value.as_object().unwrap() {
        c["values"][0][k.as_str()] = v.clone();
    }
    preset(&[], vec![c])
}

fn way_range(d: &Value) -> Value {
    d["cc_params"]["entries"][0]["ways"][0]["range"].clone()
}

fn arg_range(d: &Value) -> Value {
    d["commands"][0]["args"][0]["range"].clone()
}

/// The losses at a bound of the value or of the message, sorted.
fn bound_losses(lost: &[Loss]) -> Vec<&str> {
    let mut out: Vec<&str> = lost
        .iter()
        .map(|l| l.place.as_str())
        .filter(|p| ["/min", "/max", "/onValue", "/offValue"].iter().any(|k| p.ends_with(k)))
        .collect();
    out.sort();
    out
}

/// The kinds carried as controls with the whole scale of their way.
fn scales() -> Vec<(Value, i64)> {
    vec![(cc7(1), 127), (cc14(2), 16383), (nrpn(3), 16383), (rpn(4), 16383)]
}

// ---- import: a way ----------------------------------------------------------------------

#[test]
fn the_min_and_max_of_a_message_are_the_range_of_its_way() {
    for (message, _) in scales() {
        let (d, lost) = imported(&one("fader", message.clone(), json!({"min": 10, "max": 100}), json!({})));
        assert_eq!(way_range(&d), json!([10, 100]), "{message}: {d}");
        assert!(bound_losses(&lost).is_empty(), "{message}: {lost:?}");
    }
}

#[test]
fn each_bound_is_the_message_s_else_the_value_s() {
    for (message, value) in [(json!({"min": 10}), json!({"max": 100})), (json!({"max": 100}), json!({"min": 10}))] {
        let (d, lost) = imported(&one("fader", cc7(1), message.clone(), value.clone()));
        assert_eq!(way_range(&d), json!([10, 100]), "{message} {value}: {d}");
        assert!(bound_losses(&lost).is_empty(), "{message} {value}: {lost:?}");
    }
}

#[test]
fn a_bound_of_the_value_that_differs_from_the_range_carried_is_a_loss_at_its_pointer() {
    let (d, lost) = imported(&one("fader", cc7(1), json!({"min": 0, "max": 100}), json!({"min": -50, "max": 50})));
    assert_eq!(way_range(&d), json!([0, 100]), "{d}");
    assert_eq!(bound_losses(&lost), [format!("{VALUE}/max"), format!("{VALUE}/min")], "{lost:?}");
    let (d, lost) = imported(&one("fader", cc7(1), json!({"min": 5, "max": 100}), json!({"min": 5, "max": 90})));
    assert_eq!(way_range(&d), json!([5, 100]), "{d}");
    assert_eq!(bound_losses(&lost), [format!("{VALUE}/max")], "{lost:?}");
}

#[test]
fn the_same_bounds_on_the_message_and_the_value_are_one_range_and_no_loss() {
    let (d, lost) = imported(&one("fader", cc7(1), json!({"min": 10, "max": 100}), json!({"min": 10, "max": 100})));
    assert_eq!(way_range(&d), json!([10, 100]), "{d}");
    assert!(bound_losses(&lost).is_empty(), "{lost:?}");
}

#[test]
fn a_message_range_equal_to_the_whole_scale_without_a_bound_on_the_value_gives_no_range() {
    for (message, high) in scales() {
        let (d, lost) = imported(&one("fader", message.clone(), json!({"min": 0, "max": high}), json!({})));
        assert!(way_range(&d).is_null(), "{message}: {d}");
        assert!(bound_losses(&lost).is_empty(), "{message}: {lost:?}");
    }
}

#[test]
fn a_message_range_equal_to_the_whole_scale_is_carried_when_the_value_has_a_bound() {
    let (d, lost) = imported(&one("fader", cc7(1), json!({"min": 0, "max": 127}), json!({"min": 0, "max": 127})));
    assert_eq!(way_range(&d), json!([0, 127]), "{d}");
    assert!(bound_losses(&lost).is_empty(), "{lost:?}");
    let (d, lost) = imported(&one("fader", cc7(1), json!({"min": 0, "max": 127}), json!({"min": 0, "max": 100})));
    assert_eq!(way_range(&d), json!([0, 127]), "{d}");
    assert_eq!(bound_losses(&lost), [format!("{VALUE}/max")], "{lost:?}");
}

#[test]
fn a_lone_bound_of_a_message_gives_no_range_and_is_a_loss() {
    for bound in ["min", "max"] {
        let mut extra = json!({});
        extra[bound] = json!(10);
        let (d, lost) = imported(&one("fader", cc7(1), extra, json!({})));
        assert!(way_range(&d).is_null(), "{bound}: {d}");
        assert!(held(&lost, &format!("{MESSAGE}/{bound}")), "{bound}: {lost:?}");
    }
}

/// The `offValue` and `onValue` of a pad whose range is carried: each named at its pointer, the
/// reason saying that the range is carried and that no pad is written back.
fn assert_pad_values_named(lost: &[Loss], what: &str) {
    for key in ["offValue", "onValue"] {
        let place = format!("{MESSAGE}/{key}");
        let found: Vec<&Loss> = lost.iter().filter(|l| l.place == place).collect();
        assert_eq!(found.len(), 1, "{what}: one loss at {place}: {lost:?}");
        let reason = found[0].reason.to_lowercase();
        assert!(["range", "carried", "pad"].iter().all(|w| reason.contains(w)), "{what}: the range is carried, no pad is written: {reason:?}");
    }
}

#[test]
fn the_off_and_on_values_of_a_pad_are_the_range_of_its_way_and_are_named_for_the_pad() {
    for (message, _) in scales() {
        let (d, lost) = imported(&one("pad", message.clone(), json!({"offValue": 0, "onValue": 1}), json!({})));
        assert_eq!(way_range(&d), json!([0, 1]), "{message}: {d}");
        assert_eq!(bound_losses(&lost), [format!("{MESSAGE}/offValue"), format!("{MESSAGE}/onValue")], "{message}: {lost:?}");
        assert_pad_values_named(&lost, &message.to_string());
    }
    let (d, lost) = imported(&one("pad", cc7(1), json!({"offValue": 20, "onValue": 100}), json!({})));
    assert_eq!(way_range(&d), json!([20, 100]), "{d}");
    assert_pad_values_named(&lost, "20..100");
    let (d, lost) = imported(&one("pad", cc7(1), json!({"offValue": 20, "onValue": 100, "min": 0, "max": 127}), json!({})));
    assert_eq!(way_range(&d), json!([20, 100]), "{d}");
    assert_pad_values_named(&lost, "20..100 with a message range");
}

#[test]
fn the_off_and_on_values_of_the_pad_of_a_command_are_the_range_of_its_argument_and_are_named_for_the_pad() {
    for kind in ["u7", "u14"] {
        let (message, _) = command_of(kind);
        for (off, on) in [(0, 1), (20, 100)] {
            let (d, lost) = imported(&one("pad", message.clone(), json!({"offValue": off, "onValue": on}), json!({})));
            assert_eq!(arg_range(&d), json!([off, on]), "{kind}: {d}");
            assert_pad_values_named(&lost, kind);
        }
    }
}

#[test]
fn a_pad_without_off_and_on_values_takes_the_range_of_its_message() {
    let (d, lost) = imported(&one("pad", cc7(1), json!({"min": 10, "max": 100}), json!({})));
    assert_eq!(way_range(&d), json!([10, 100]), "{d}");
    assert!(bound_losses(&lost).is_empty(), "{lost:?}");
}

#[test]
fn a_lone_off_or_on_value_of_a_pad_or_an_off_value_not_below_the_on_value_gives_no_range_and_a_loss() {
    for extra in [json!({"offValue": 0}), json!({"onValue": 100}), json!({"offValue": 100, "onValue": 0}), json!({"offValue": 5, "onValue": 5})] {
        let (d, lost) = imported(&one("pad", cc7(1), extra.clone(), json!({})));
        assert!(way_range(&d).is_null(), "{extra}: {d}");
        assert!(lost.iter().any(|l| l.place.starts_with(MESSAGE) && !l.place.ends_with("/deviceId")), "{extra}: {lost:?}");
    }
}

#[test]
fn the_off_and_on_values_of_a_control_that_is_no_pad_give_no_range_and_are_named() {
    let (d, lost) = imported(&one("fader", cc7(1), json!({"offValue": 0, "onValue": 1}), json!({})));
    assert!(way_range(&d).is_null(), "{d}");
    for key in ["offValue", "onValue"] {
        assert!(held(&lost, &format!("{MESSAGE}/{key}")), "{key}: {lost:?}");
    }
}

#[test]
fn the_default_value_of_a_value_is_a_loss_and_no_default_of_the_control() {
    let (d, lost) = imported(&one("fader", cc7(1), json!({}), json!({"defaultValue": 64, "min": 0, "max": 100})));
    assert!(d["cc_params"]["entries"][0].get("default").is_none(), "{d}");
    assert!(held(&lost, &format!("{VALUE}/defaultValue")), "{lost:?}");
}

#[test]
fn the_items_of_a_list_are_judged_on_the_range_the_message_gives() {
    let mut p = one("list", cc7(1), json!({"min": 0, "max": 1}), json!({"overlayId": 1}));
    p["overlays"] = json!([{"id": 1, "items": [{"value": 0, "label": "Off"}, {"value": 1, "label": "On"}, {"value": 2, "label": "Out"}]}]);
    let (d, lost) = imported(&p);
    assert_eq!(d["cc_params"]["entries"][0]["values"].as_array().map(Vec::len), Some(2), "{d}");
    assert!(held(&lost, "/overlays/0/items/2"), "{lost:?}");
}

// ---- import: the argument of a command --------------------------------------------------

fn command_of(kind: &str) -> (Value, i64) {
    let data = if kind == "u7" { json!([67, {"type": "value"}]) } else { json!([67, value_element(0, 0, 7), value_element(7, 0, 7)]) };
    (sysex_template(1, data), if kind == "u7" { 127 } else { 16383 })
}

#[test]
fn the_min_and_max_of_a_sysex_message_are_the_range_of_the_argument_of_its_command() {
    for kind in ["u7", "u14"] {
        let (message, _) = command_of(kind);
        let (d, lost) = imported(&one("fader", message, json!({"min": 3, "max": 100}), json!({})));
        assert_eq!(arg_range(&d), json!([3, 100]), "{kind}: {d}");
        assert!(bound_losses(&lost).is_empty(), "{kind}: {lost:?}");
    }
}

#[test]
fn a_sysex_message_range_equal_to_the_scale_of_the_argument_gives_no_range() {
    for kind in ["u7", "u14"] {
        let (message, high) = command_of(kind);
        let (d, lost) = imported(&one("fader", message, json!({"min": 0, "max": high}), json!({})));
        assert!(arg_range(&d).is_null(), "{kind}: {d}");
        assert!(bound_losses(&lost).is_empty(), "{kind}: {lost:?}");
    }
}

#[test]
fn a_bound_of_the_value_that_differs_from_the_range_of_the_argument_is_a_loss_at_its_pointer() {
    let (message, _) = command_of("u7");
    let (d, lost) = imported(&one("fader", message, json!({"min": 0, "max": 100}), json!({"min": 0, "max": 90})));
    assert_eq!(arg_range(&d), json!([0, 100]), "{d}");
    assert_eq!(bound_losses(&lost), [format!("{VALUE}/max")], "{lost:?}");
}

// ---- export -----------------------------------------------------------------------------

fn exported(d: &osd::Resolved) -> Value {
    serde_json::from_str(&export(d, Format::ElectraOne).expect("exports").value).unwrap()
}

/// The `min` and `max` of the message, then of the value, of the control at `n`.
fn bounds(p: &Value, n: usize) -> [Value; 4] {
    let v = &p["controls"][n]["values"][0];
    [v["message"]["min"].clone(), v["message"]["max"].clone(), v["min"].clone(), v["max"].clone()]
}

fn whole(low: i64, high: i64) -> [Value; 4] {
    [json!(low), json!(high), Value::Null, Value::Null]
}

fn ranged(low: i64, high: i64) -> [Value; 4] {
    [json!(low), json!(high), json!(low), json!(high)]
}

fn entry(title: &str, way: Value) -> Value {
    json!({"osc": format!("/{}", title.to_lowercase()), "title": title, "ways": [way]})
}

#[test]
fn the_export_writes_in_each_message_the_scale_of_its_way_and_the_bounds_of_the_value_only_for_a_range() {
    let p = exported(&device6(json!([
        entry("A", json!({"ctrlType": "cc", "ctrlIndex": [1], "value_bits": 7})),
        entry("B", json!({"ctrlType": "cc", "ctrlIndex": [2, 34], "value_bits": 14})),
        entry("C", json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14})),
        entry("D", json!({"ctrlType": "rpn", "ctrlIndex": [0, 1], "value_bits": 14})),
        entry("E", json!({"ctrlType": "cc", "ctrlIndex": [5], "value_bits": 7, "range": [10, 100]})),
        entry("F", json!({"ctrlType": "nrpn", "ctrlIndex": [1, 3], "value_bits": 14, "range": [0, 1000]})),
        entry("G", json!({"ctrlType": "cc", "ctrlIndex": [7], "value_bits": 7, "range": [0, 127]})),
    ])));
    assert_eq!(bounds(&p, 0), whole(0, 127));
    assert_eq!(bounds(&p, 1), whole(0, 16383));
    assert_eq!(bounds(&p, 2), whole(0, 16383));
    assert_eq!(bounds(&p, 3), whole(0, 16383));
    assert_eq!(bounds(&p, 4), ranged(10, 100));
    assert_eq!(bounds(&p, 5), ranged(0, 1000));
    assert_eq!(bounds(&p, 6), ranged(0, 127));
    assert_eq!(violations(&p), Vec::<String>::new());
}

#[test]
fn a_list_of_named_values_without_a_range_writes_the_scale_in_its_message_and_no_bound_on_its_value() {
    let mut e = entry("Wave", json!({"ctrlType": "cc", "ctrlIndex": [9], "value_bits": 7}));
    e["values"] = json!([{"value": 0, "title": "Sine"}, {"value": 1, "title": "Saw"}]);
    let p = exported(&device6(json!([e])));
    assert_eq!(p["controls"][0]["type"], "list");
    assert_eq!(bounds(&p, 0), whole(0, 127));
}

#[test]
fn the_export_writes_the_off_and_on_values_of_a_pad_as_the_scale_of_its_way() {
    let mut a = entry("A", json!({"ctrlType": "cc", "ctrlIndex": [1], "value_bits": 7}));
    a["expects"] = json!("switch");
    let mut b = entry("B", json!({"ctrlType": "cc", "ctrlIndex": [2], "value_bits": 7, "range": [0, 1]}));
    b["expects"] = json!("momentary");
    let p = exported(&device6(json!([a, b])));
    for (n, low, high, expected) in [(0, 0, 127, whole(0, 127)), (1, 0, 1, ranged(0, 1))] {
        assert_eq!(p["controls"][n]["type"], "pad");
        let m = &p["controls"][n]["values"][0]["message"];
        assert_eq!((m["offValue"].clone(), m["onValue"].clone()), (json!(low), json!(high)), "{m}");
        assert_eq!(bounds(&p, n), expected, "control {n}");
    }
}

#[test]
fn the_pad_of_a_command_without_an_argument_sends_127_when_pressed() {
    let p = exported(&device(json!([]), json!([{"osc": "/dump", "frame": [240, 1, 247]}])));
    assert_eq!(p["controls"][0]["type"], "pad");
    assert_eq!(p["controls"][0]["values"][0]["message"]["onValue"], 127, "{p}");
    let v = &p["controls"][0]["values"][0];
    assert!(v.get("min").is_none() && v.get("max").is_none(), "{v}");
}

#[test]
fn every_range_and_whole_scale_comes_back_through_the_export_then_the_import() {
    let mut pad = entry("Pad", json!({"ctrlType": "cc", "ctrlIndex": [6], "value_bits": 7, "range": [0, 1]}));
    pad["expects"] = json!("switch");
    let d = device6(json!([
        entry("A", json!({"ctrlType": "cc", "ctrlIndex": [1], "value_bits": 7})),
        entry("B", json!({"ctrlType": "cc", "ctrlIndex": [2, 34], "value_bits": 14})),
        entry("C", json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14, "range": [0, 16383]})),
        entry("D", json!({"ctrlType": "cc", "ctrlIndex": [5], "value_bits": 7, "range": [10, 100]})),
        pad,
    ]));
    let out = export(&d, Format::ElectraOne).unwrap();
    let back = import(&out.value, Format::ElectraOne, &source()).unwrap();
    let ranges = |d: &osd::Device| crate::common::entries(d).iter().map(|e| e.ways[0].range).collect::<Vec<_>>();
    assert_eq!(ranges(&back.value), ranges(&d.device));
    let again: Value = serde_json::from_str(&export(&resolved(back.value), Format::ElectraOne).unwrap().value).unwrap();
    let first: Value = serde_json::from_str(&out.value).unwrap();
    for n in 0..5 {
        assert_eq!(bounds(&again, n), bounds(&first, n), "control {n}");
    }
}

// ---- import: a pad over the whole scale -------------------------------------------------

#[test]
fn the_off_and_on_values_of_a_pad_that_give_the_whole_scale_give_no_range_and_are_named() {
    let ways = scales().into_iter().map(|(message, high)| (message, high, false));
    let commands = ["u7", "u14"].into_iter().map(|kind| {
        let (message, high) = command_of(kind);
        (message, high, true)
    });
    for (message, high, is_command) in ways.chain(commands) {
        let (d, lost) = imported(&one("pad", message.clone(), json!({"offValue": 0, "onValue": high}), json!({})));
        let range = if is_command { arg_range(&d) } else { way_range(&d) };
        assert!(range.is_null(), "{message}: {d}");
        assert_eq!(bound_losses(&lost), [format!("{MESSAGE}/offValue"), format!("{MESSAGE}/onValue")], "{message}: {lost:?}");
    }
}

#[test]
fn the_off_and_on_values_of_a_pad_over_the_whole_scale_are_a_range_and_named_when_the_value_has_a_bound() {
    let (d, lost) = imported(&one("pad", cc7(1), json!({"offValue": 0, "onValue": 127}), json!({"min": 0, "max": 127})));
    assert_eq!(way_range(&d), json!([0, 127]), "{d}");
    assert_eq!(bound_losses(&lost), [format!("{MESSAGE}/offValue"), format!("{MESSAGE}/onValue")], "{lost:?}");
    assert_pad_values_named(&lost, "whole scale with a bound on the value");
    let (message, _) = command_of("u7");
    let (d, lost) = imported(&one("pad", message, json!({"offValue": 0, "onValue": 127}), json!({"min": 0, "max": 127})));
    assert_eq!(arg_range(&d), json!([0, 127]), "{d}");
    assert_pad_values_named(&lost, "a command over the whole scale");
}
