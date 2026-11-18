//! What the Electra One tests share: presets built control by control, the rules of the
//! maker's page "Preset format description" (docs.electra.one/developers/presetformat.html,
//! documentation 5.0.0) written as checks, and a device as its JSON.

#![allow(dead_code)]

use serde_json::{json, Value};

/// Every kind of message the maker's page documents.
pub const KINDS: [&str; 18] = [
    "cc7", "cc14", "nrpn", "rpn", "sysex", "note", "program", "start", "stop", "tune", "atpoly", "atchannel", "pitchbend",
    "spp", "relcc", "macro", "virtual", "none",
];

/// The kinds carried as controls; then the kind carried as a command.
pub const CONTROL_KINDS: [&str; 4] = ["cc7", "cc14", "nrpn", "rpn"];
pub const COMMAND_KIND: &str = "sysex";

/// A source with the credit and licence of a preset.
pub fn source() -> osd::Source {
    let mut s = super::source();
    s.url = Some("https://app.electra.one/preset/abc".to_string());
    s.author = Some("A preset maker".to_string());
    s.license = Some("CC-BY-4.0".to_string());
    s
}

pub fn cc7(n: u64) -> Value {
    json!({"type": "cc7", "deviceId": 1, "parameterNumber": n})
}
pub fn cc14(n: u64) -> Value {
    json!({"type": "cc14", "deviceId": 1, "parameterNumber": n})
}
pub fn nrpn(n: u64) -> Value {
    json!({"type": "nrpn", "deviceId": 1, "parameterNumber": n})
}
pub fn rpn(n: u64) -> Value {
    json!({"type": "rpn", "deviceId": 1, "parameterNumber": n})
}
pub fn sysex(bytes: &[u64]) -> Value {
    json!({"type": "sysex", "deviceId": 1, "data": bytes})
}
/// A message of any other kind.
pub fn other(kind: &str) -> Value {
    json!({"type": kind, "deviceId": 1, "parameterNumber": 1})
}

/// A control of one value: its look (page, slot, colour, bounds, shape) and its message.
pub fn control(id: u64, name: &str, group: Option<u64>, message: Value, range: Option<(i64, i64)>) -> Value {
    let mut value = json!({"id": "value", "message": message});
    if let Some((lo, hi)) = range {
        value["min"] = json!(lo);
        value["max"] = json!(hi);
    }
    let mut c = json!({
        "id": id, "type": "fader", "name": name, "color": "FF0000", "bounds": [0, 0, 146, 56],
        "pageId": 1, "controlSetId": 1, "inputs": [{"potId": 1, "valueId": "value"}], "values": [value],
    });
    if let Some(g) = group {
        c["groupId"] = json!(g);
    }
    c
}

/// A preset of version 3: its groups (id, name) on page 1 and its controls.
pub fn preset(groups: &[(u64, &str)], controls: Vec<Value>) -> Value {
    let groups: Vec<Value> =
        groups.iter().map(|(id, name)| json!({"id": id, "pageId": 1, "name": name, "bounds": [0, 0, 300, 100]})).collect();
    json!({
        "version": 3, "name": "A preset", "projectId": "p1",
        "pages": [{"id": 1, "name": "Page 1"}], "groups": groups, "overlays": [],
        "devices": [{"id": 1, "name": "Device", "port": 1, "channel": 1}],
        "controls": controls,
    })
}

/// A device from the JSON of its entries and commands: one source, 0.5.
pub fn device(entries: Value, commands: Value) -> osd::Resolved {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.5/device.schema.json",
        "_sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "osc_prefix": "/trundler"},
        "cc_params": {"entries": entries},
        "commands": commands,
    });
    super::resolved(super::device_from(&doc.to_string()))
}

/// What an import returned, as the JSON a reader sees.
pub fn json_of(d: &osd::Device) -> Value {
    let text = osd::write(&osd::Descriptor::Device(Box::new(d.clone())));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "the import is not read strictly: {text}");
    serde_json::from_str(&text).unwrap()
}

/// The violations of the maker's documented rules in a preset (each rule from the page).
pub fn violations(preset: &Value) -> Vec<String> {
    let mut out = Vec::new();
    // Version: 2 or 3; the export writes 3.
    if preset["version"] != json!(3) {
        out.push("version: the export writes 3".to_string());
    }
    let controls = preset["controls"].as_array().cloned().unwrap_or_default();
    // At most 432 controls and 32 devices.
    if controls.len() > 432 {
        out.push(format!("{} controls, at most 432", controls.len()));
    }
    if preset["devices"].as_array().map_or(0, |d| d.len()) > 32 {
        out.push("more than 32 devices".to_string());
    }
    let (mut ids, mut slots) = (Vec::new(), Vec::new());
    for (n, c) in controls.iter().enumerate() {
        ids.push(c["id"].clone());
        let (page, set) = (c["pageId"].as_u64().unwrap_or(0), c["controlSetId"].as_u64().unwrap_or(0));
        if !(1..=12).contains(&page) {
            out.push(format!("/controls/{n}/pageId {page}: 1 to 12"));
        }
        if !(1..=3).contains(&set) {
            out.push(format!("/controls/{n}/controlSetId {set}: 1 to 3"));
        }
        for (m, input) in c["inputs"].as_array().into_iter().flatten().enumerate() {
            let pot = input["potId"].as_u64().unwrap_or(0);
            if !(1..=12).contains(&pot) {
                out.push(format!("/controls/{n}/inputs/{m}/potId {pot}: 1 to 12"));
            }
            slots.push((page, set, pot));
        }
        for (m, v) in c["values"].as_array().into_iter().flatten().enumerate() {
            let kind = v["message"]["type"].as_str().unwrap_or("");
            if !KINDS.contains(&kind) {
                out.push(format!("/controls/{n}/values/{m}/message/type {kind:?}: not a documented type"));
            }
        }
    }
    let mut sorted = slots.clone();
    sorted.sort();
    sorted.dedup();
    if sorted.len() != slots.len() {
        out.push("two controls share a page, control set and pot".to_string());
    }
    let unique = |v: &[Value]| {
        let mut s: Vec<String> = v.iter().map(|x| x.to_string()).collect();
        s.sort();
        s.dedup();
        s.len() == v.len()
    };
    if !unique(&ids) {
        out.push("control ids are not unique".to_string());
    }
    let groups = preset["groups"].as_array().cloned().unwrap_or_default();
    let group_ids: Vec<Value> = groups.iter().map(|g| g["id"].clone()).collect();
    if !unique(&group_ids) {
        out.push("group ids are not unique".to_string());
    }
    let page_ids: Vec<Value> = preset["pages"].as_array().into_iter().flatten().map(|g| g["id"].clone()).collect();
    if !unique(&page_ids) {
        out.push("page ids are not unique".to_string());
    }
    for (n, c) in controls.iter().enumerate() {
        if let Some(g) = c.get("groupId") {
            if !group_ids.contains(g) {
                out.push(format!("/controls/{n}/groupId {g}: no such group"));
            }
        }
    }
    out
}
