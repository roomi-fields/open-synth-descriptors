//! What the MIDI 2.0 controller list tests share: the properties of M2-117-UM table 5, the
//! lists and devices the tests build, and the reading of an export and of an import.

#![allow(dead_code)]

use serde_json::{json, Value};

/// The properties of a control of MIDI-CI Property Exchange Controller Resources
/// (M2-117-UM v1.0.1, table 5).
pub const PROPERTIES: [&str; 15] = [
    "title",
    "description",
    "channel",
    "ctrlType",
    "ctrlIndex",
    "priority",
    "default",
    "transmit",
    "recognize",
    "numSigBits",
    "paramPath",
    "typeHint",
    "ctrlMapId",
    "stepCount",
    "minMax",
];

/// A source with the credit and licence of a list.
pub fn source() -> osd::Source {
    let mut s = super::source();
    s.url = Some("https://example.org/midi2/device".to_string());
    s.author = Some("A maker".to_string());
    s.license = Some("CC-BY-4.0".to_string());
    s
}

/// The device the tests export: its blocks are the argument.
pub fn device(extra: Value) -> osd::Resolved {
    let mut doc = json!({"role_vocabulary": "1"});
    for (k, v) in extra.as_object().expect("an object of blocks") {
        doc[k] = v.clone();
    }
    super::midnam::device_with(doc)
}

pub fn cc_device(entries: Value) -> osd::Resolved {
    device(json!({"cc_params": {"entries": entries}}))
}

/// What an export wrote: the object, and the list of losses.
pub fn exported(d: &osd::Resolved) -> (Value, Vec<osd_formats::Loss>) {
    let t = osd_formats::export(d, osd_formats::Format::Midi2Controllers).unwrap_or_else(|f| panic!("{f}"));
    let v: Value = serde_json::from_str(&t.value).unwrap_or_else(|e| panic!("{e}: {}", t.value));
    (v, t.lost)
}

/// The entries of the list an export wrote.
pub fn list(v: &Value) -> Vec<Value> {
    v["ChCtrlList"].as_array().unwrap_or_else(|| panic!("no ChCtrlList in {v}")).clone()
}

/// The named values a control points at, from the maps of the object, sorted.
pub fn named(v: &Value, entry: &Value) -> Vec<(i64, String)> {
    let Some(id) = entry.get("ctrlMapId") else { return Vec::new() };
    let map = v["CtrlMapList"].as_array().into_iter().flatten().find(|m| &m["ctrlMapId"] == id).unwrap_or_else(|| panic!("no map {id} in {v}"));
    let mut out: Vec<(i64, String)> =
        map["ctrlMap"].as_array().unwrap_or_else(|| panic!("no ctrlMap in {map}")).iter().map(|e| (e["value"].as_i64().unwrap(), e["title"].as_str().unwrap().to_string())).collect();
    out.sort();
    out
}

/// What an import returned: the device as JSON, and the list of losses.
pub fn imported(text: &str) -> (Value, Vec<osd_formats::Loss>) {
    let t = osd_formats::import(text, osd_formats::Format::Midi2Controllers, &source()).unwrap_or_else(|f| panic!("{f}\n{text}"));
    (super::electra::json_of(&t.value), t.lost)
}

pub fn refused(text: &str) -> osd_formats::Fault {
    osd_formats::import(text, osd_formats::Format::Midi2Controllers, &source()).expect_err("the list is refused")
}

/// The entries of a device the import made, controls then parameters.
pub fn controls(d: &Value) -> Vec<Value> {
    ["cc_params", "params"].iter().flat_map(|k| d[k]["entries"].as_array().cloned().unwrap_or_default()).collect()
}

/// A list entry of a continuous 7-bit CC.
pub fn cc_entry(n: i64) -> Value {
    json!({"title": format!("Control {n}"), "ctrlType": "cc", "ctrlIndex": [n], "numSigBits": 7, "minMax": [0, 127]})
}
