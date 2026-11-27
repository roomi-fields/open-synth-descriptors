//! What the Electra One tests share: presets built control by control, the rules of the
//! maker's page "Preset format description" (docs.electra.one/developers/presetformat.html,
//! documentation 5.0.0) written as checks, and a device as its JSON.

#![allow(dead_code)]

use serde_json::{json, Value};

/// The page of the maker the list of attributes is copied from, by hand, and its version.
pub const PAGE_ADDRESS: &str = "docs.electra.one/developers/presetformat.html";
pub const PAGE_VERSION: &str = "5.0.0";

/// Every attribute of a preset that page documents, named by its path: an item of a list is
/// `[]`, an element of a SysEx template `template.<type>`.
pub const ATTRIBUTES: [&str; 92] = [
    "version",
    "name",
    "projectId",
    "pages",
    "devices",
    "overlays",
    "groups",
    "controls",
    "devices[].id",
    "devices[].name",
    "devices[].port",
    "devices[].channel",
    "devices[].interfaces",
    "devices[].runningStatus",
    "devices[].rate",
    "devices[].patch",
    "devices[].messages",
    "devices[].messages[].id",
    "devices[].messages[].direction",
    "devices[].messages[].data",
    "overlays[].id",
    "overlays[].items",
    "overlays[].items[].value",
    "overlays[].items[].label",
    "overlays[].items[].bitmap",
    "overlays[].items[].color",
    "groups[].id",
    "groups[].pageId",
    "groups[].name",
    "groups[].bounds",
    "groups[].color",
    "groups[].variant",
    "groups[].font",
    "groups[].values",
    "controls[].id",
    "controls[].type",
    "controls[].name",
    "controls[].color",
    "controls[].variant",
    "controls[].mode",
    "controls[].font",
    "controls[].bounds",
    "controls[].pageId",
    "controls[].controlSetId",
    "controls[].groupId",
    "controls[].visible",
    "controls[].inputs",
    "controls[].values",
    "controls[].events",
    "controls[].valueRefs",
    "controls[].values[].id",
    "controls[].values[].name",
    "controls[].values[].min",
    "controls[].values[].max",
    "controls[].values[].defaultValue",
    "controls[].values[].placeholder",
    "controls[].values[].sensitivity",
    "controls[].values[].formatter",
    "controls[].values[].function",
    "controls[].values[].overlayId",
    "controls[].values[].message",
    "controls[].values[].message.deviceId",
    "controls[].values[].message.type",
    "controls[].values[].message.id",
    "controls[].values[].message.parameterNumber",
    "controls[].values[].message.min",
    "controls[].values[].message.max",
    "controls[].values[].message.onValue",
    "controls[].values[].message.offValue",
    "controls[].values[].message.value",
    "controls[].values[].message.lsbFirst",
    "controls[].values[].message.resetRpn",
    "controls[].values[].message.signMode",
    "controls[].values[].message.bitWidth",
    "controls[].values[].message.relative",
    "controls[].values[].message.relativeMode",
    "controls[].values[].message.accelerated",
    "controls[].values[].message.data",
    "controls[].values[].message.direction",
    "template.value.rules[].type",
    "template.value.rules[].parameterNumber",
    "template.value.rules[].parameterBitPosition",
    "template.value.rules[].byteBitPosition",
    "template.value.rules[].bitWidth",
    "template.parameter.parameterBitPosition",
    "template.parameter.byteBitPosition",
    "template.parameter.bitWidth",
    "template.checksum.algorithm",
    "template.checksum.start",
    "template.checksum.length",
    "template.function.name",
    "template.any",
];

/// Every kind of message the maker's page documents.
pub const KINDS: [&str; 18] = [
    "cc7", "cc14", "nrpn", "rpn", "sysex", "note", "program", "start", "stop", "tune", "atpoly", "atchannel", "pitchbend",
    "spp", "relcc", "macro", "virtual", "none",
];

/// The kinds carried as controls; then the kind carried as a command.
pub const CONTROL_KINDS: [&str; 5] = ["cc7", "cc14", "nrpn", "rpn", "relcc"];
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

/// A device of 0.7 from the JSON of its entries: one source, no command.
pub fn device6(entries: Value) -> osd::Resolved {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "osc_prefix": "/trundler"},
        "role_vocabulary": "1",
        "cc_params": {"entries": entries},
    });
    super::resolved(super::device_from_fixture(doc))
}

/// The longest `name` of a control (page, control, attribute `name`: maxLength 40).
pub const CONTROL_NAME_MAX: usize = 40;
/// The longest `label` of an overlay item (page, overlay, `items`: maxLength 20).
pub const LABEL_MAX: usize = 20;

fn overlaps(a: &Value, b: &Value) -> bool {
    let n = |v: &Value, i: usize| v[i].as_i64().unwrap();
    let (ax, ay, aw, ah) = (n(a, 0), n(a, 1), n(a, 2), n(a, 3));
    let (bx, by, bw, bh) = (n(b, 0), n(b, 1), n(b, 2), n(b, 3));
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

fn inside(inner: &Value, outer: &Value) -> bool {
    let n = |v: &Value, i: usize| v[i].as_i64().unwrap();
    n(inner, 0) >= n(outer, 0)
        && n(inner, 1) >= n(outer, 1)
        && n(inner, 0) + n(inner, 2) <= n(outer, 0) + n(outer, 2)
        && n(inner, 1) + n(inner, 3) <= n(outer, 1) + n(outer, 3)
}

/// What makes a preset hard to play: a name too long, a list without its overlay or with a label
/// too long, a group that overlaps another or does not frame exactly its own controls.
pub fn unplayable(preset: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let controls = preset["controls"].as_array().cloned().unwrap_or_default();
    let groups = preset["groups"].as_array().cloned().unwrap_or_default();
    let overlays = preset["overlays"].as_array().cloned().unwrap_or_default();
    for (n, c) in controls.iter().enumerate() {
        let len = c["name"].as_str().unwrap_or("").chars().count();
        if len > CONTROL_NAME_MAX {
            out.push(format!("/controls/{n}/name: {len} characters, at most {CONTROL_NAME_MAX}"));
        }
        if c["type"] == "list" {
            let id = &c["values"][0]["overlayId"];
            if !overlays.iter().any(|o| o["id"] == *id) {
                out.push(format!("/controls/{n}: a list without its overlay"));
            }
        }
    }
    for (n, o) in overlays.iter().enumerate() {
        for (m, i) in o["items"].as_array().into_iter().flatten().enumerate() {
            if i["label"].as_str().unwrap_or("").chars().count() > LABEL_MAX {
                out.push(format!("/overlays/{n}/items/{m}/label: more than {LABEL_MAX} characters"));
            }
        }
    }
    for (a, g) in groups.iter().enumerate() {
        for (b, h) in groups.iter().enumerate().skip(a + 1) {
            if g["pageId"] == h["pageId"] && overlaps(&g["bounds"], &h["bounds"]) {
                out.push(format!("groups {a} and {b} overlap on page {}", g["pageId"]));
            }
        }
        for (n, c) in controls.iter().enumerate().filter(|(_, c)| c["pageId"] == g["pageId"]) {
            let own = c.get("groupId") == Some(&g["id"]);
            if own && !inside(&c["bounds"], &g["bounds"]) {
                out.push(format!("/controls/{n} is outside its group {a}"));
            }
            if !own && overlaps(&c["bounds"], &g["bounds"]) {
                out.push(format!("/controls/{n} lies in the group {a}, which is not its own"));
            }
        }
    }
    out
}

// ---- SysEx templates (page, "SysEx template") ---------------------------------------------

/// A SysEx message of a control: its parameter number and its template.
pub fn sysex_template(parameter: u64, data: Value) -> Value {
    json!({"type": "sysex", "deviceId": 1, "parameterNumber": parameter, "data": data})
}

/// `{ "type": "checksum" }` with the rules the maker documents: algorithm, start, length
/// (`Value::Null` leaves a rule out).
pub fn checksum_element(algorithm: Value, start: Value, length: Option<u64>) -> Value {
    let mut e = json!({"type": "checksum"});
    if !algorithm.is_null() {
        e["algorithm"] = algorithm;
    }
    if !start.is_null() {
        e["start"] = start;
    }
    if let Some(l) = length {
        e["length"] = json!(l);
    }
    e
}

/// `{ "type": "value" }` taking `width` bits of the value from bit `pp`, put at bit `bp` of the byte.
pub fn value_element(pp: u64, bp: u64, width: u64) -> Value {
    json!({"type": "value", "rules": [{"parameterBitPosition": pp, "byteBitPosition": bp, "bitWidth": width}]})
}

/// The bytes a frame of an imported command sends, by the rules of the maker's page: an
/// argument is its value on 7 bits (`u14_lsb` the low 7 bits of a 14-bit value, `u14_msb` the
/// high 7), a checksum covers the bytes already written from `from` (F0 is 0) on `length` bytes,
/// cut to what is written, by `roland` (128 minus the sum, on 7 bits), `waldorf` (the sum on 7
/// bits) or `fractal` (the xor on 7 bits), as `twos7`, `sum7` and `xor7`.
pub fn render_frame(frame: &Value, value: i64) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for e in frame.as_array().expect("a frame is an array") {
        if let Some(b) = e.as_u64() {
            out.push(b as u8);
        } else if e.get("arg").is_some() {
            let v = match e.get("encoding").and_then(Value::as_str) {
                None | Some("u14_lsb") => value & 127,
                Some("u14_msb") => (value >> 7) & 127,
                Some(other) => panic!("encoding {other} is not used by a template"),
            };
            out.push(v as u8);
        } else if let Some(algorithm) = e.get("checksum").and_then(Value::as_str) {
            let from = e["from"].as_u64().expect("from") as usize;
            let end = match e.get("length").and_then(Value::as_u64) {
                Some(l) => (from + l as usize).min(out.len()),
                None => out.len(),
            };
            let covered = &out[from.min(end)..end];
            let sum: u32 = covered.iter().map(|b| *b as u32).sum();
            let byte = match algorithm {
                "twos7" => (128 - sum % 128) & 0x7F,
                "sum7" => sum & 0x7F,
                "xor7" => covered.iter().fold(0u32, |a, b| a ^ *b as u32) & 0x7F,
                other => panic!("checksum {other} is not written by OSD"),
            };
            out.push(byte as u8);
        } else {
            panic!("element {e} is not one a template becomes");
        }
    }
    out
}

/// The behaviours of SyntHub's former Electra One importer that are a conversion: each a row of
/// the table "What the former importer kept" in docs/electra-one.md.
pub const FORMER_BEHAVIOURS: [&str; 11] = [
    "checksum",
    "checksum without a known algorithm or start",
    "byte computed from the value",
    "byte computed from the parameter number",
    "lua function of one return",
    "lua function with a branch",
    "lua function calling an API of the controller",
    "lua function not defined",
    "parameter number of a command",
    "date of the preset",
    "revision and hash of the preset",
];

/// The behaviour of the former importer that is the policy of a re-import tool, not a conversion.
pub const FORMER_POLICY: &str = "silent address";

/// What an import returned, as the JSON a reader sees.
pub fn json_of(d: &osd::Device) -> Value {
    let text = osd::write(&osd::Descriptor::Device(Box::new(d.clone())));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "the import is not read strictly: {text}");
    serde_json::from_str(&text).unwrap()
}

/// A device as the JSON of 0.7 (its controls as ways), whatever version it was read in: the
/// device of an older case is carried to 0.7 by the converter before it is compared.
pub fn json_of_0_7(d: &osd::Device) -> Value {
    let text = osd::write(&osd::Descriptor::Device(Box::new(d.clone())));
    let text = osd::convert_to(&text, osd::Kind::Device, osd::Version::V0_7).unwrap_or_else(|f| panic!("the device does not convert: {f:?}\n{text}"));
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
