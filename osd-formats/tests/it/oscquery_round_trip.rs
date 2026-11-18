//! A device exported to OSCQuery then imported keeps everything the namespace carries: the
//! address of each control, parameter and command, the type of its arguments and their range.

use crate::common::midnam::{device_with, lost_at};
use crate::common::oscquery::{methods, source};
use crate::common::{credited_devices, electra::json_of};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

fn tag_of(ty: &str) -> &'static str {
    match ty {
        "float" => "f",
        "string" | "enum" => "s",
        "bool" => "T",
        _ => "i",
    }
}

/// What a namespace carries of a device: for each address, the tags and ranges of its arguments.
/// A control or a parameter is a method of one integer.
fn carried(d: &Value) -> Vec<(String, String, Vec<Value>)> {
    let prefix = d["device"]["osc_prefix"].as_str().unwrap().to_string();
    let mut out = Vec::new();
    for key in ["cc_params", "params"] {
        for e in d[key]["entries"].as_array().into_iter().flatten() {
            let range = e.get("range").map_or(Value::Null, |r| json!({"MIN": r[0], "MAX": r[1]}));
            out.push((format!("{prefix}{}", e["osc"].as_str().unwrap()), "i".to_string(), vec![range]));
        }
    }
    for c in d["commands"].as_array().into_iter().flatten() {
        let Some(osc) = c["osc"].as_str() else { continue };
        let args = c["args"].as_array().cloned().unwrap_or_default();
        let tags: String = args.iter().map(|a| tag_of(a["type"].as_str().unwrap())).collect();
        let ranges = args.iter().map(|a| a.get("range").map_or(Value::Null, |r| json!({"MIN": r[0], "MAX": r[1]}))).collect();
        out.push((format!("{prefix}{osc}"), tags, ranges));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn back(device: &osd::Resolved) -> (Value, Vec<osd_formats::Loss>) {
    let out = export(device, Format::OscQuery).expect("exports");
    let t = import(&out.value, Format::OscQuery, &source()).unwrap_or_else(|f| panic!("{f}\n{}", out.value));
    (json_of(&t.value), out.lost)
}

#[test]
fn every_control_parameter_and_command_comes_back_at_its_address_with_its_argument_types_and_ranges() {
    let d = device_with(json!({
        "cc_params": {"entries": [
            {"osc": "/filter/cutoff", "cc": 74, "range": [10, 100]},
            {"osc": "/filter/resonance", "cc": 71},
        ]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
            "entries": [{"osc": "/p/one", "pr": 1, "p": 2, "c": 3, "r": 4, "range": [0, 63]}]},
        "commands": [
            {"osc": "/mix", "args": [{"name": "level", "type": "int", "range": [0, 10]}, {"name": "pan", "type": "float", "range": [-1, 1]},
                {"name": "label", "type": "string"}, {"name": "mute", "type": "bool"}], "frame": [240, 247]},
            {"osc": "/panic", "frame": [240, 247]},
        ],
    }));
    let original = json_of(&d.device);
    let (b, _) = back(&d);
    assert_eq!(carried(&b), carried(&original));
}

#[test]
fn the_names_of_the_arguments_of_a_command_come_back() {
    let d = device_with(json!({"commands": [{"osc": "/mix", "args": [{"name": "level", "type": "int"}, {"name": "pan", "type": "float"}], "frame": [240, 247]}]}));
    let (b, _) = back(&d);
    let names: Vec<&str> = b["commands"][0]["args"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["level", "pan"]);
}

#[test]
fn the_prefix_comes_back_for_a_device_of_one_segment() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}));
    let (b, _) = back(&d);
    assert_eq!(b["device"]["osc_prefix"], "/trundler");
}

#[test]
fn the_devices_of_the_cases_and_the_examples_come_back_as_the_namespace_carries_them() {
    let mut with_methods = 0;
    for (name, d) in credited_devices() {
        let original = json_of(&d.device);
        // A command whose address has a capture is not a plain address; the cases hold none.
        let (b, _) = back(&d);
        let want = carried(&original);
        if !want.is_empty() {
            with_methods += 1;
        }
        let got: Vec<_> = carried(&b);
        // A device of no method has no node to name the prefix, and gets none back.
        if !want.is_empty() {
            assert_eq!(got, want, "{name}");
        }
    }
    assert!(with_methods >= 2, "{with_methods}");
}

#[test]
fn the_export_lists_every_element_present_that_the_namespace_does_not_carry() {
    let d = device_with(json!({
        "commands": [{"osc": "/dump", "frame": [240, 1, 247]}],
        "triggers": {"entries": [{"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "note": 36}]},
        "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127], "section": "Filter"}]},
    }));
    let out = export(&d, Format::OscQuery).unwrap();
    for place in ["/commands/0/frame", "/triggers", "/cc_params/entries/0/section"] {
        assert_eq!(lost_at(&out.lost, place).len(), 1, "{place}: {:?}", out.lost);
    }
    let ns: Value = serde_json::from_str(&out.value).unwrap();
    assert_eq!(methods(&ns).len(), 2);
}

#[test]
fn what_is_lost_in_the_export_is_absent_from_what_comes_back() {
    let d = device_with(json!({
        "triggers": {"entries": [{"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "note": 36}]},
        "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127], "section": "Filter"}]},
    }));
    let (b, _) = back(&d);
    assert!(b.get("triggers").is_none() && b.get("cc_params").is_none(), "{b}");
    assert!(!b.to_string().contains("Filter") && !b.to_string().contains("Kick"), "{b}");
}
