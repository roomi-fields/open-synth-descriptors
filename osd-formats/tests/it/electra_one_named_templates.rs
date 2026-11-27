//! A SysEx message of an Electra One preset whose `data` is a number takes the template of that
//! id in the `messages` of the device its `deviceId` names (page "Preset format description",
//! sections "Device" and "SysEx template"); a template whose direction is `in` is never sent.

use crate::common::electra::*;
use osd_formats::{import, Format, Loss};
use serde_json::{json, Value};

const MESSAGE: &str = "/controls/0/values/0/message";

fn imported(p: &Value) -> (Value, Vec<Loss>) {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("imports");
    (json_of(&t.value), t.lost)
}

/// A preset of one control "Send" with this message, and these devices.
fn with_devices(message: Value, devices: Value) -> Value {
    let mut p = preset(&[], vec![control(1, "Send", None, message, Some((0, 100)))]);
    p["devices"] = devices;
    p
}

fn device_of(id: u64, messages: Value) -> Value {
    json!({"id": id, "name": "Synth", "port": 1, "channel": 1, "messages": messages})
}

fn by_id(device: u64, id: u64) -> Value {
    json!({"type": "sysex", "deviceId": device, "parameterNumber": 7, "data": id})
}

fn commands(d: &Value) -> Vec<Value> {
    d.get("commands").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn templates() -> Vec<Value> {
    vec![
        json!([67, 1]),
        json!([67, {"type": "value"}]),
        json!([240, 65, 16, value_element(7, 0, 7), value_element(0, 0, 7), checksum_element(json!("roland"), json!(1), Some(4)), 247]),
    ]
}

/// The command a template written inline gives.
fn inline(template: &Value) -> Value {
    let (d, _) = imported(&with_devices(sysex_template(7, template.clone()), json!([device_of(1, json!([]))])));
    commands(&d).remove(0)
}

#[test]
fn a_template_named_by_its_id_gives_the_command_the_same_template_written_inline_gives() {
    for template in templates() {
        for direction in [Some("out"), None] {
            let mut message = json!({"id": 3, "data": template.clone()});
            if let Some(d) = direction {
                message["direction"] = json!(d);
            }
            let (d, lost) = imported(&with_devices(by_id(1, 3), json!([device_of(1, json!([message]))])));
            let got = commands(&d);
            assert_eq!(got.len(), 1, "{template}: {lost:?}");
            assert_eq!(got[0], inline(&template), "{template}");
            assert!(lost.iter().all(|l| l.place != MESSAGE), "{template}: {lost:?}");
        }
    }
}

#[test]
fn the_device_of_the_message_chooses_the_template_among_the_devices_that_hold_its_id() {
    let devices = json!([
        device_of(1, json!([{"id": 3, "data": [67, 1]}])),
        device_of(2, json!([{"id": 4, "data": [67, 2]}, {"id": 3, "data": [67, 3]}])),
    ]);
    for (device, id, byte) in [(1u64, 3u64, 1u64), (2, 3, 3), (2, 4, 2)] {
        let (d, lost) = imported(&with_devices(by_id(device, id), devices.clone()));
        let got = commands(&d);
        assert_eq!(got.len(), 1, "device {device}, id {id}: {lost:?}");
        assert_eq!(got[0]["frame"], json!([240, 67, byte, 247]), "device {device}, id {id}");
    }
}

#[test]
fn several_controls_may_name_one_template() {
    let mut p = preset(&[], vec![control(1, "One", None, by_id(1, 3), None), control(2, "Two", None, by_id(1, 3), None)]);
    p["devices"] = json!([device_of(1, json!([{"id": 3, "data": [67, 1]}]))]);
    let (d, lost) = imported(&p);
    assert_eq!(commands(&d).len(), 2, "{lost:?}");
}

#[test]
fn an_id_that_names_no_template_gives_no_command_and_a_loss_at_the_message() {
    let devices = json!([device_of(1, json!([{"id": 3, "data": [67, 1]}])), device_of(2, json!([]))]);
    for message in [by_id(1, 9), by_id(2, 3), by_id(5, 3)] {
        let (d, lost) = imported(&with_devices(message.clone(), devices.clone()));
        assert!(commands(&d).is_empty(), "{message}: {d}");
        assert!(lost.iter().any(|l| l.place == MESSAGE && !l.reason.is_empty()), "{message}: {lost:?}");
        // The bounds of a command that is not written are named too.
        for bound in ["min", "max"] {
            let place = format!("/controls/0/values/0/{bound}");
            assert!(lost.iter().any(|l| l.place == place), "{message}: {lost:?}");
        }
    }
}

#[test]
fn a_template_whose_direction_is_in_is_never_sent_and_gives_no_command_and_a_loss_at_the_message() {
    // The direction of the message, else that of the template.
    for (of_message, of_template, sent) in [
        (None, Some("in"), false),
        (Some("in"), None, false),
        (Some("in"), Some("out"), false),
        (Some("out"), Some("in"), true),
        (Some("out"), None, true),
        (None, Some("out"), true),
        (None, None, true),
    ] {
        let mut message = by_id(1, 3);
        if let Some(d) = of_message {
            message["direction"] = json!(d);
        }
        let mut template = json!({"id": 3, "data": [67, 1]});
        if let Some(d) = of_template {
            template["direction"] = json!(d);
        }
        let (d, lost) = imported(&with_devices(message, json!([device_of(1, json!([template]))])));
        let case = format!("message {of_message:?}, template {of_template:?}");
        assert_eq!(commands(&d).len(), usize::from(sent), "{case}: {lost:?}");
        assert_eq!(lost.iter().any(|l| l.place == MESSAGE), !sent, "{case}: {lost:?}");
    }
}

#[test]
fn a_named_template_with_an_element_osd_does_not_write_gives_no_command_and_a_loss_at_the_message() {
    for element in [json!({"type": "parameter"}), json!({"type": "function", "name": "pickByte"}), json!({"type": "any"})] {
        let template = json!({"id": 3, "data": [67, element.clone()]});
        let (d, lost) = imported(&with_devices(by_id(1, 3), json!([device_of(1, json!([template]))])));
        assert!(commands(&d).is_empty(), "{element}: {d}");
        let l = lost.iter().find(|l| l.place == MESSAGE).unwrap_or_else(|| panic!("{element}: {lost:?}"));
        assert!(l.reason.contains("Send"), "the loss names the control: {}", l.reason);
    }
}
