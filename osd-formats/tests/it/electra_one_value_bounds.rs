//! The `min` and `max` of a value of an Electra One preset whose message is not a control: they are
//! the range of the argument `value` of the command a SysEx template becomes, or they are named in
//! the losses (osd-formats/docs/CADRE.md R8).

use crate::common::electra::*;
use osd_formats::{import, FaultKind, Format, Loss};
use serde_json::{json, Value};

const MIN: &str = "/controls/0/values/0/min";
const MAX: &str = "/controls/0/values/0/max";
const MESSAGE: &str = "/controls/0/values/0/message";

fn imported(p: &Value) -> (Value, Vec<Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("the preset imports");
    (json_of(&t.value), t.lost)
}

/// A preset of one control of this message, its value holding these bounds.
fn one(message: Value, min: Option<Value>, max: Option<Value>) -> Value {
    let mut c = control(1, "Send", None, message, None);
    for (key, bound) in [("min", min), ("max", max)] {
        if let Some(b) = bound {
            c["values"][0][key] = b;
        }
    }
    preset(&[], vec![c])
}

fn with_value() -> Value {
    sysex_template(1, json!([67, {"type": "value"}]))
}

fn wide_value() -> Value {
    sysex_template(1, json!([67, value_element(0, 0, 7), value_element(7, 0, 7)]))
}

fn commands(d: &Value) -> Vec<Value> {
    d.get("commands").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// The places of the losses of a bound, sorted.
fn bound_places(lost: &[Loss]) -> Vec<&str> {
    let mut out: Vec<&str> = lost.iter().map(|l| l.place.as_str()).filter(|p| p.ends_with("/min") || p.ends_with("/max")).collect();
    out.sort();
    out
}

// ---- a command with an argument ---------------------------------------------------------

#[test]
fn the_min_and_max_of_a_sysex_value_are_the_range_of_the_argument_of_its_command() {
    for (message, ty, (lo, hi)) in [
        (with_value(), "u7", (0, 10)),
        (with_value(), "u7", (10, 100)),
        (with_value(), "u7", (-64, 63)),
        (wide_value(), "u14", (0, 16000)),
    ] {
        let (d, lost) = imported(&one(message, Some(json!(lo)), Some(json!(hi))));
        let c = commands(&d);
        assert_eq!(c.len(), 1, "{d}");
        assert_eq!(c[0]["args"].as_array().map(Vec::len), Some(1), "{d}");
        assert_eq!((c[0]["args"][0]["name"].clone(), c[0]["args"][0]["type"].clone()), (json!("value"), json!(ty)), "{d}");
        assert_eq!(c[0]["args"][0]["range"], json!([lo, hi]), "{d}");
        assert!(bound_places(&lost).is_empty(), "{lo} {hi}: {lost:?}");
    }
}

#[test]
fn each_sysex_value_gives_its_own_bounds_to_its_own_command() {
    let mut p = preset(
        &[],
        vec![control(1, "First", None, with_value(), Some((1, 2))), control(2, "Second", None, with_value(), None)],
    );
    let mut second = p["controls"][0]["values"][0].clone();
    second["min"] = json!(3);
    second["max"] = json!(4);
    p["controls"][1]["values"].as_array_mut().unwrap().push(second);
    let (d, lost) = imported(&p);
    let ranges: Vec<Value> = commands(&d).iter().map(|c| c["args"][0]["range"].clone()).collect();
    assert_eq!(ranges, [json!([1, 2]), Value::Null, json!([3, 4])], "{d}");
    assert!(bound_places(&lost).is_empty(), "{lost:?}");
}

#[test]
fn a_lone_bound_of_a_sysex_value_gives_no_range_and_is_named() {
    for (min, max, named) in [(Some(json!(0)), None, MIN), (None, Some(json!(10)), MAX)] {
        for message in [with_value(), wide_value()] {
            let (d, lost) = imported(&one(message, min.clone(), max.clone()));
            let c = commands(&d);
            assert_eq!(c.len(), 1, "{d}");
            assert!(c[0]["args"][0].get("range").is_none(), "{d}");
            assert_eq!(bound_places(&lost), [named], "{lost:?}");
            let reason = &lost.iter().find(|l| l.place == named).unwrap().reason;
            assert!(reason.contains("both bounds"), "{reason}");
        }
    }
}

#[test]
fn a_bound_of_a_sysex_argument_that_is_not_a_whole_number_is_refused_at_its_pointer() {
    for (min, max, place) in [
        (Some(json!(1.5)), Some(json!(10)), MIN),
        (Some(json!(0)), Some(json!("10")), MAX),
        (Some(json!("0")), None, MIN),
        (None, Some(json!(2.5)), MAX),
        (Some(json!(null)), Some(json!(10)), MIN),
    ] {
        let p = one(with_value(), min.clone(), max.clone());
        let fault = import(&p.to_string(), Format::ElectraOne, &source()).expect_err(&format!("{min:?} {max:?} is refused"));
        assert_eq!((fault.kind, fault.place.as_str()), (FaultKind::InvalidFile, place), "{min:?} {max:?}");
    }
}

#[test]
fn a_sysex_value_without_bounds_gives_an_argument_without_a_range_and_names_no_bound() {
    let (d, lost) = imported(&one(with_value(), None, None));
    assert!(commands(&d)[0]["args"][0].get("range").is_none(), "{d}");
    assert!(bound_places(&lost).is_empty(), "{lost:?}");
}

// ---- bounds the descriptor does not carry -----------------------------------------------

/// Both bounds of the value at `at` are named once each, with a reason.
fn assert_both_named(lost: &[Loss], at: &str, case: &str) {
    assert_eq!(bound_places(lost), [format!("{at}/max"), format!("{at}/min")], "{case}: {lost:?}");
    assert!(lost.iter().all(|l| !l.reason.is_empty()), "{case}: {lost:?}");
}

/// The messages of one control that give no argument to hold the bounds, and whether the message
/// itself is lost.
fn messages_without_a_place_for_bounds() -> Vec<(Value, bool)> {
    let mut out = vec![
        // A command written without an argument.
        (sysex(&[240, 67, 1, 247]), false),
        (sysex_template(1, json!([67, 1])), false),
        (sysex_template(1, json!([67, 2, checksum_element(json!("roland"), json!(0), Some(3))])), false),
        // A template that is not carried.
        (sysex_template(1, json!([67, {"type": "parameter"}])), true),
        (sysex_template(1, json!([67, {"type": "function", "name": "pickByte"}])), true),
        (sysex_template(1, json!([67, {"type": "value"}, {"type": "parameter"}])), true),
        (sysex_template(1, json!("named")), true),
        (json!({"type": "sysex", "deviceId": 1}), true),
    ];
    // A message of a kind that is lost.
    let carried = |k: &&str| CONTROL_KINDS.contains(k) || *k == COMMAND_KIND;
    out.extend(KINDS.iter().filter(|k| !carried(k)).map(|k| (other(k), true)));
    out
}

#[test]
fn the_bounds_of_a_value_whose_message_gives_nothing_to_hold_them_are_each_named() {
    for (message, message_lost) in messages_without_a_place_for_bounds() {
        let (_, lost) = imported(&one(message.clone(), Some(json!(0)), Some(json!(10))));
        let case = message.to_string();
        assert_both_named(&lost, "/controls/0/values/0", &case);
        assert_eq!(lost.iter().any(|l| l.place == MESSAGE), message_lost, "{case}: {lost:?}");
    }
}

#[test]
fn a_command_without_an_argument_is_still_written_when_its_bounds_are_named() {
    let (d, lost) = imported(&one(sysex_template(1, json!([67, 1])), Some(json!(0)), Some(json!(10))));
    let c = commands(&d);
    assert_eq!(c.len(), 1, "{d}");
    assert!(c[0].get("args").is_none(), "{d}");
    assert_both_named(&lost, "/controls/0/values/0", "fixed bytes");
}

#[test]
fn only_the_bounds_a_value_holds_are_named_when_its_message_cannot_carry_them() {
    for (message, _) in messages_without_a_place_for_bounds() {
        let (_, lost) = imported(&one(message.clone(), Some(json!(0)), None));
        assert_eq!(bound_places(&lost), [MIN], "{message}: {lost:?}");
        let (_, lost) = imported(&one(message.clone(), None, Some(json!(10))));
        assert_eq!(bound_places(&lost), [MAX], "{message}: {lost:?}");
        let (_, lost) = imported(&one(message.clone(), None, None));
        assert!(bound_places(&lost).is_empty(), "{message}: {lost:?}");
    }
}

#[test]
fn the_bounds_of_a_value_whose_message_an_earlier_control_claims_are_each_named() {
    for message in [cc7(74), cc14(5), nrpn(5), rpn(5)] {
        let p = preset(
            &[],
            vec![control(1, "First", None, message.clone(), Some((0, 100))), control(2, "Second", None, message.clone(), Some((0, 10)))],
        );
        let (d, lost) = imported(&p);
        let case = message.to_string();
        assert_eq!(d["cc_params"]["entries"].as_array().map(Vec::len), Some(1), "{case}: {d}");
        assert_eq!(d["cc_params"]["entries"][0]["ways"][0]["range"], json!([0, 100]), "{case}: {d}");
        assert_both_named(&lost, "/controls/1/values/0", &case);
        assert!(lost.iter().any(|l| l.place == "/controls/1/values/0/message"), "{case}: {lost:?}");
    }
}

#[test]
fn a_lone_bound_of_a_claimed_message_is_named() {
    let mut p = preset(&[], vec![control(1, "First", None, cc7(74), None), control(2, "Second", None, cc7(74), None)]);
    p["controls"][1]["values"][0]["max"] = json!(10);
    let (_, lost) = imported(&p);
    assert_eq!(bound_places(&lost), ["/controls/1/values/0/max"], "{lost:?}");
}

#[test]
fn a_value_without_a_message_is_named_whole_and_not_by_its_bounds() {
    let mut p = one(cc7(1), Some(json!(0)), Some(json!(10)));
    p["controls"][0]["values"][0].as_object_mut().unwrap().remove("message");
    let (_, lost) = imported(&p);
    assert!(lost.iter().any(|l| l.place == "/controls/0/values/0"), "{lost:?}");
    assert!(bound_places(&lost).is_empty(), "{lost:?}");
}

#[test]
fn the_bounds_a_control_carries_in_the_range_of_its_way_are_not_named() {
    for message in [cc7(1), cc14(2), nrpn(3), rpn(4)] {
        let (d, lost) = imported(&one(message.clone(), Some(json!(0)), Some(json!(10))));
        assert_eq!(d["cc_params"]["entries"][0]["ways"][0]["range"], json!([0, 10]), "{message}: {d}");
        assert!(bound_places(&lost).is_empty(), "{message}: {lost:?}");
    }
}
