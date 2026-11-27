//! A relative message of an Electra One preset (page "Preset format description", section
//! "Message", `relative` and `relativeMode`): the import reads it as a way with `transmit` and
//! `recognize` `relative`, and the export writes such a way back.

use crate::common::electra::*;
use crate::common::fields::held;
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

const MESSAGE: &str = "/controls/0/values/0/message";

fn imported(message: Value) -> (Value, Vec<Loss>) {
    let p = preset(&[], vec![control(1, "Encoder", None, message, None)]);
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("imports");
    (json_of(&t.value)["cc_params"]["entries"][0]["ways"][0].clone(), t.lost)
}

fn kinds() -> [Value; 4] {
    [cc7(20), cc14(5), nrpn(300), rpn(2)]
}

#[test]
fn a_message_with_relative_true_is_a_way_that_sends_and_receives_relative_values() {
    for mut message in kinds() {
        message["relative"] = json!(true);
        let (way, lost) = imported(message.clone());
        assert_eq!((way["transmit"].clone(), way["recognize"].clone()), (json!("relative"), json!("relative")), "{message}: {way}");
        assert!(!held(&lost, &format!("{MESSAGE}/relative")), "{message}: {lost:?}");
    }
}

#[test]
fn a_message_without_relative_or_with_relative_false_is_not_relative() {
    for mut message in kinds() {
        for relative in [None, Some(false)] {
            if let Some(r) = relative {
                message["relative"] = json!(r);
            }
            let (way, _) = imported(message.clone());
            assert!(way.get("transmit").is_none() && way.get("recognize").is_none(), "{message}: {way}");
        }
    }
}

#[test]
fn the_encoding_of_the_steps_of_a_relative_message_stays_a_loss() {
    for mut message in kinds() {
        message["relative"] = json!(true);
        message["relativeMode"] = json!("twosComplement");
        let (_, lost) = imported(message.clone());
        assert!(held(&lost, &format!("{MESSAGE}/relativeMode")), "{message}: {lost:?}");
    }
}

/// The relative way of each kind, with the type of message the preset writes it as and whether
/// the message says `relative: true`.
fn relative_ways() -> Vec<(Value, &'static str)> {
    let relative = |mut way: Value| {
        way["transmit"] = json!("relative");
        way["recognize"] = json!("relative");
        way
    };
    vec![
        (relative(json!({"ctrlType": "cc", "ctrlIndex": [20], "value_bits": 7})), "relcc"),
        (relative(json!({"ctrlType": "cc", "ctrlIndex": [5, 37], "value_bits": 14})), "cc14"),
        (relative(json!({"ctrlType": "nrpn", "ctrlIndex": [2, 44], "value_bits": 14})), "nrpn"),
        (relative(json!({"ctrlType": "rpn", "ctrlIndex": [0, 2], "value_bits": 14})), "rpn"),
    ]
}

#[test]
fn a_relative_way_is_written_as_relcc_for_a_cc_of_one_number_and_else_with_relative_true() {
    for (way, kind) in relative_ways() {
        let d = device6(json!([{"osc": "/encoder", "title": "Encoder", "ways": [way]}]));
        let p: Value = serde_json::from_str(&export(&d, Format::ElectraOne).unwrap().value).unwrap();
        let message = &p["controls"][0]["values"][0]["message"];
        assert_eq!(message["type"], kind, "{message}");
        if kind != "relcc" {
            assert_eq!(message["relative"], json!(true), "{message}");
        }
    }
}

#[test]
fn a_relative_way_of_every_kind_comes_back_relative_and_names_neither_direction() {
    for (way, kind) in relative_ways() {
        let d = device6(json!([{"osc": "/encoder", "title": "Encoder", "ways": [way.clone()]}]));
        let out = export(&d, Format::ElectraOne).unwrap();
        for key in ["transmit", "recognize"] {
            assert!(!held(&out.lost, &format!("/cc_params/entries/0/ways/0/{key}")), "{kind}: {:?}", out.lost);
        }
        let back = import(&out.value, Format::ElectraOne, &source()).unwrap();
        let got = json_of(&back.value)["cc_params"]["entries"][0]["ways"][0].clone();
        assert_eq!(got, way, "{kind}");
    }
}

#[test]
fn a_way_relative_on_one_side_only_does_not_come_back_as_it_was_and_the_side_is_named() {
    for (key, other) in [("transmit", "recognize"), ("recognize", "transmit")] {
        for kind in ["cc", "nrpn"] {
            let mut way = if kind == "cc" {
                json!({"ctrlType": "cc", "ctrlIndex": [20], "value_bits": 7})
            } else {
                json!({"ctrlType": "nrpn", "ctrlIndex": [2, 44], "value_bits": 14})
            };
            way[key] = json!("relative");
            way[other] = json!("absolute");
            let d = device6(json!([{"osc": "/encoder", "title": "Encoder", "ways": [way.clone()]}]));
            let out = export(&d, Format::ElectraOne).unwrap();
            let back = import(&out.value, Format::ElectraOne, &source()).unwrap();
            let got = json_of(&back.value)["cc_params"]["entries"][0]["ways"][0].clone();
            for side in ["transmit", "recognize"] {
                if got[side] != way[side] {
                    assert!(held(&out.lost, &format!("/cc_params/entries/0/ways/0/{side}")), "{kind} {key} {side}: {:?}", out.lost);
                }
            }
        }
    }
}
