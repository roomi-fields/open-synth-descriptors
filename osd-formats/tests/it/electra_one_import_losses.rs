//! The losses of the Electra One import (osd-formats/docs/CADRE.md R8): any key of a preset, at
//! any depth, that the importer does not read is in the losses at its pointer or at the pointer of
//! a block that holds it, and what the import follows is not named.

use crate::common::electra::*;
use crate::common::fields::held;
use osd_formats::{import, Format, Loss};
use serde_json::{json, Value};

fn lost_of(p: &Value) -> Vec<Loss> {
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("the preset imports");
    json_of(&t.value);
    t.lost
}

fn assert_held(lost: &[Loss], pointers: &[&str]) {
    let silent: Vec<&&str> = pointers.iter().filter(|p| !held(lost, p)).collect();
    assert!(silent.is_empty(), "not in the losses: {silent:?}\n{lost:?}");
}

fn assert_not_named(lost: &[Loss], pointers: &[&str]) {
    let named: Vec<&&str> = pointers.iter().filter(|p| held(lost, p)).collect();
    assert!(named.is_empty(), "named though the import follows or carries it: {named:?}\n{lost:?}");
}

/// A preset whose every object holds keys the import reads, and room for more.
fn full() -> Value {
    json!({
        "version": 3, "name": "A preset", "projectId": "p1",
        "pages": [{"id": 1, "name": "Page 1"}],
        "devices": [{"id": 1, "name": "Synth", "port": 1, "channel": 1, "messages": [
            {"id": 5, "direction": "out", "data": [67, {"type": "value"}]},
            {"id": 6, "direction": "out", "data": [67, 2]}
        ]}],
        "overlays": [{"id": 1, "items": [{"value": 0, "label": "Off"}, {"value": 1, "label": "On"}]}],
        "groups": [
            {"id": 11, "pageId": 1, "name": "Filter", "bounds": [0, 0, 300, 100]},
            {"id": 12, "pageId": 1, "name": "Nobody", "bounds": [0, 100, 300, 100]},
            {"id": 13, "pageId": 1, "name": "Only a note", "bounds": [0, 200, 300, 100]},
            {"id": 14, "pageId": 1, "name": "Utility", "bounds": [0, 300, 300, 100]},
            {"id": 15, "pageId": 1, "name": "Only a claimed message", "bounds": [0, 400, 300, 100]}
        ],
        "controls": [
            {"id": 101, "type": "fader", "name": "Cutoff", "groupId": 11, "inputs": [{"potId": 1, "valueId": "value"}],
             "values": [{"id": "value", "overlayId": 1, "message": {"type": "cc7", "deviceId": 1, "parameterNumber": 74}}]},
            {"id": 102, "type": "pad", "name": "Dump", "groupId": 14,
             "values": [{"id": "value", "message": {"type": "sysex", "deviceId": 1, "data": [67, {"type": "value", "rules": [{"bitWidth": 7}]},
                 {"type": "checksum", "algorithm": "roland", "start": 0, "length": 2}]}}]},
            {"id": 103, "type": "pad", "name": "By id", "groupId": 14,
             "values": [{"id": "value", "message": {"type": "sysex", "deviceId": 1, "data": 5}}]},
            {"id": 104, "type": "pad", "name": "Note", "groupId": 13,
             "values": [{"id": "value", "message": {"type": "note", "deviceId": 1, "parameterNumber": 60}}]},
            {"id": 105, "type": "fader", "name": "Empty"},
            {"id": 106, "type": "fader", "name": "Again", "groupId": 15,
             "values": [{"id": "value", "message": {"type": "cc7", "deviceId": 1, "parameterNumber": 74}}]},
            {"id": 107, "type": "fader", "name": "Silent", "values": [{"id": "value"}]}
        ]
    })
}

/// The objects of that preset, by pointer: each takes a key.
const OBJECTS: [&str; 19] = [
    "",
    "/pages/0",
    "/devices/0",
    "/devices/0/messages/0",
    "/devices/0/messages/0/data/1",
    "/overlays/0",
    "/overlays/0/items/0",
    "/groups/0",
    "/controls/0",
    "/controls/0/inputs/0",
    "/controls/0/values/0",
    "/controls/0/values/0/message",
    "/controls/1/values/0/message",
    "/controls/1/values/0/message/data/1",
    "/controls/1/values/0/message/data/1/rules/0",
    "/controls/1/values/0/message/data/2",
    "/controls/2/values/0/message",
    "/controls/3/values/0/message",
    "/controls/4",
];

#[test]
fn a_key_the_maker_does_not_document_is_in_the_losses_at_any_depth() {
    for object in OBJECTS {
        for (key, escaped) in [("zzUnread", "zzUnread"), ("a/b~c", "a~1b~0c")] {
            let mut p = full();
            p.pointer_mut(object).unwrap_or_else(|| panic!("{object}"))[key] = json!(1);
            let pointer = format!("{object}/{escaped}");
            assert_held(&lost_of(&p), &[&pointer]);
        }
    }
}

#[test]
fn each_documented_attribute_the_import_does_not_read_is_in_the_losses() {
    let attributes: [(&str, &[&str]); 7] = [
        ("/devices/0", &["name", "port", "channel", "interfaces", "runningStatus", "rate", "patch"]),
        ("/overlays/0/items/0", &["bitmap", "color"]),
        ("/groups/0", &["pageId", "bounds", "color", "variant", "font", "values"]),
        ("/controls/0", &["type", "color", "variant", "mode", "font", "bounds", "pageId", "controlSetId", "visible", "inputs", "events", "valueRefs"]),
        ("/controls/0/values/0", &["name", "defaultValue", "placeholder", "sensitivity", "formatter", "function"]),
        ("/controls/0/values/0/message", &["lsbFirst", "resetRpn", "signMode", "bitWidth", "relativeMode", "accelerated"]),
        ("/controls/1", &["color", "inputs"]),
    ];
    for (object, keys) in attributes {
        for key in keys {
            let mut p = full();
            let at = p.pointer_mut(object).unwrap();
            if at.get(*key).is_none() {
                at[*key] = json!(1);
            }
            let pointer = format!("{object}/{key}");
            assert_held(&lost_of(&p), &[&pointer]);
        }
    }
}

#[test]
fn the_id_of_a_control_and_of_a_value_is_in_the_losses() {
    let lost = lost_of(&full());
    assert_held(&lost, &["/controls/0/id", "/controls/1/id", "/controls/2/id", "/controls/4/id", "/controls/0/values/0/id", "/controls/1/values/0/id"]);
}

#[test]
fn the_name_of_a_group_no_carried_control_or_command_uses_is_in_the_losses() {
    let lost = lost_of(&full());
    assert_held(&lost, &["/groups/1/name", "/groups/2/name", "/groups/4/name"]);
}

#[test]
fn the_name_of_a_group_a_carried_control_or_command_uses_is_not_named() {
    assert_not_named(&lost_of(&full()), &["/groups/0/name", "/groups/3/name"]);
}

#[test]
fn what_the_import_follows_is_not_named() {
    let lost = lost_of(&full());
    assert_not_named(&lost, &[
        "/version",
        "/groups/0/id",
        "/groups/3/id",
        "/overlays/0/id",
        "/devices/0/messages/0/id",
        "/devices/0/messages/0/data",
        "/controls/0/groupId",
        "/controls/0/values/0/overlayId",
    ]);
}

#[test]
fn the_name_the_message_and_the_items_a_control_carries_are_not_named() {
    let lost = lost_of(&full());
    assert_not_named(&lost, &[
        "/name",
        "/controls/0/name",
        "/controls/0/values/0/message/type",
        "/controls/0/values/0/message/parameterNumber",
        "/controls/1/name",
        "/controls/1/values/0/message/data",
        "/overlays/0/items/0/value",
        "/overlays/0/items/0/label",
    ]);
}

#[test]
fn the_device_of_a_message_is_in_the_losses_as_the_setting_of_the_controller() {
    assert_held(&lost_of(&full()), &["/controls/0/values/0/message/deviceId", "/controls/1/values/0/message/deviceId"]);
}
