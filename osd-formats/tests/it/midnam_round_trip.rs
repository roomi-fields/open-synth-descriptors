//! A device exported to MIDNAM then imported keeps everything the format carries (the elements
//! the document of destiny marks `carried`), and the export lists the rest in its losses.

use crate::common::midnam::*;
use crate::common::{credited_devices, electra::json_of, resolved, source};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

/// What MIDNAM carries of a device: its vendor and name, the binding, range and default of each
/// control, each note and each bank.
fn carried(d: &Value) -> Value {
    let pick = |v: &Value, keys: &[&str]| {
        let mut o = serde_json::Map::new();
        for k in keys {
            if let Some(x) = v.get(*k) {
                o.insert(k.to_string(), x.clone());
            }
        }
        Value::Object(o)
    };
    let controls: Vec<Value> = d["cc_params"]["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| {
            let mut c = pick(e, &["cc", "cc_lsb", "nrpn_msb", "nrpn_lsb", "rpn_msb", "rpn_lsb", "range", "default", "values"]);
            // The range of the span MIDI fixes for the kind is not written back.
            let span = if e.get("cc").is_some() && e.get("cc_lsb").is_none() { 127 } else { 16383 };
            if c.get("range") == Some(&json!([0, span])) {
                c.as_object_mut().unwrap().remove("range");
            }
            c
        })
        .collect();
    let notes: Vec<Value> = d["triggers"]["entries"].as_array().into_iter().flatten().map(|t| pick(t, &["sound", "note"])).collect();
    let banks: Vec<Value> =
        d["programs"]["banks"].as_array().into_iter().flatten().map(|b| pick(b, &["name", "msb", "lsb", "size", "first_number", "first_pc"])).collect();
    json!({"vendor": d["device"]["vendor"], "name": d["device"]["name"], "controls": controls, "notes": notes, "banks": banks})
}

fn back(device: &osd::Resolved) -> (Value, Vec<osd_formats::Loss>) {
    let out = export(device, Format::Midnam).expect("exports");
    let t = import(&out.value, Format::Midnam, &source()).unwrap_or_else(|f| panic!("{f}\n{}", out.value));
    (json_of(&t.value), out.lost)
}

fn original(device: &osd::Resolved) -> Value {
    json_of(&device.device)
}

#[test]
fn every_control_note_and_bank_comes_back_with_its_binding_range_default_and_names() {
    let d = device_with(json!({
        "cc_params": {"entries": [
            {"osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "default": 64},
            {"osc": "/wide", "cc": 5, "cc_lsb": 37, "range": [0, 16383]},
            {"osc": "/fine", "nrpn_msb": 2, "nrpn_lsb": 44, "range": [10, 900], "default": 10},
            {"osc": "/bend", "rpn_msb": 0, "rpn_lsb": 0, "range": [0, 24]},
        ]},
        "triggers": {"entries": [{"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "note": 36}]},
        "programs": {"program_change": true, "banks": [
            {"name": "Factory", "msb": 3, "lsb": 4, "size": 3, "first_pc": 10, "first_number": 1},
            {"name": "User", "size": 2, "first_pc": 0, "first_number": 0},
        ]},
    }));
    let (b, _) = back(&d);
    assert_eq!(carried(&b), carried(&original(&d)));
}

#[test]
fn the_remark_name_of_a_control_comes_back_and_the_name_built_from_the_address_too() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/filter/cutoff", "cc": 74, "range": [0, 127]},
        {"osc": "/env/a", "cc": 73, "range": [0, 127], "_name": "Attack time"},
    ]}}));
    let (b, _) = back(&d);
    let names: Vec<&str> = b["cc_params"]["entries"].as_array().unwrap().iter().map(|e| e["_name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Filter Cutoff", "Attack time"]);
}

#[test]
fn a_control_without_a_range_comes_back_without_one() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1}, {"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 1}]}}));
    let (b, _) = back(&d);
    assert!(b["cc_params"]["entries"][0].get("range").is_none());
    assert!(b["cc_params"]["entries"][1].get("range").is_none());
}

#[test]
fn the_devices_of_the_cases_and_the_examples_with_their_controls_come_back_as_the_format_carries_them() {
    let mut with_controls = 0;
    for (name, d) in credited_devices() {
        let o = original(&d);
        let (b, _) = back(&d);
        if o["cc_params"]["entries"].as_array().is_some_and(|e| !e.is_empty()) {
            with_controls += 1;
        }
        let (want, got) = (carried(&o), carried(&b));
        assert_eq!(got["controls"], want["controls"], "{name}");
        assert_eq!(got["vendor"], want["vendor"], "{name}");
        assert_eq!(got["name"], want["name"], "{name}");
    }
    assert!(with_controls >= 2, "{with_controls} devices with controls");
}

#[test]
fn the_export_lists_every_element_present_that_the_format_does_not_carry() {
    let d = device_with(json!({
        "sysex": {"header": [240, 1], "footer": [247]},
        "commands": [{"osc": "/dump", "frame": [240, 1, 247]}],
        "midi_out": {"default_channel": 3},
        "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127], "section": "Filter"}]},
    }));
    let (_, lost) = back(&d);
    for place in ["/sysex", "/commands/0", "/midi_out", "/cc_params/entries/0/section", "/device/osc_prefix"] {
        assert_eq!(lost_at(&lost, place).len(), 1, "{place}: {lost:?}");
    }
}

#[test]
fn what_is_lost_in_the_export_is_absent_from_what_comes_back() {
    let d = device_with(json!({
        "commands": [{"osc": "/dump", "frame": [240, 1, 247]}],
        "midi_out": {"default_channel": 3},
        "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127], "section": "Filter", "expects": "switch"}]},
    }));
    let (b, _) = back(&d);
    assert!(b.get("commands").is_none() && b.get("midi_out").is_none(), "{b}");
    let e = &b["cc_params"]["entries"][0];
    assert!(e.get("section").is_none() && e.get("expects").is_none(), "{e}");
}

#[test]
fn a_second_round_trip_gives_the_same_file() {
    let d = device_with(json!({
        "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127], "default": 3}, {"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]}]},
        "programs": {"program_change": true, "banks": [{"name": "Factory", "size": 2, "first_pc": 0, "first_number": 0}]},
    }));
    let first = export(&d, Format::Midnam).unwrap().value;
    let imported = import(&first, Format::Midnam, &source()).unwrap().value;
    let second = export(&resolved(imported), Format::Midnam).unwrap().value;
    // The author carries the credit of the source given to the import, which differs.
    let (a, b) = (parsed(&first), parsed(&second));
    let written = |n: &crate::common::xml::Node, name: &str| n.all(name).iter().map(|c| c.to_xml()).collect::<Vec<_>>();
    assert_eq!(written(&a, "Control"), written(&b, "Control"));
    assert_eq!(written(&a, "PatchBank"), written(&b, "PatchBank"));
}
