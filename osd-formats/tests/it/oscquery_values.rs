//! The named values of an argument cross OSCQuery as the VALS of its RANGE, the list of
//! permitted values the proposal says (integers for an integer argument, labels for a text
//! argument); what a list cannot hold is named in the losses. A controller without a range is
//! exported with the span MIDI fixes for its kind.

use crate::common::midnam::{device_with, lost_at};
use crate::common::oscquery::*;
use serde_json::{json, Value};

const OSC: &str = "/CONTENTS/synth/CONTENTS/x";

fn command_with(arg: Value) -> Value {
    json!({"commands": [{"osc": "/mode", "args": [arg], "frame": [240, 247]}]})
}

fn vals(ns: &Value, address: &str) -> Value {
    node_at(ns, address).unwrap_or_else(|| panic!("no node {address}: {ns}"))["RANGE"][0]["VALS"].clone()
}

fn sorted_texts(v: &Value) -> Vec<String> {
    let mut out: Vec<String> = v.as_array().unwrap_or_else(|| panic!("VALS is a list: {v}")).iter().map(|s| s.as_str().unwrap().to_string()).collect();
    out.sort();
    out
}

fn range_of(ns: &Value, address: &str) -> Value {
    node_at(ns, address).unwrap_or_else(|| panic!("no node {address}: {ns}"))["RANGE"].clone()
}

// ---- export: VALS ----

#[test]
fn an_integer_argument_with_values_writes_the_list_of_its_integers_in_ascending_order_as_vals() {
    let (ns, _) = exported(&device_with(command_with(json!({"name": "wave", "type": "int", "range": [0, 5], "values": {"Saw": 2, "Sine": 0, "Square": 1, "Noise": 5}}))));
    let range = range_of(&ns, "/trundler/mode");
    assert_eq!(range[0], json!({"MIN": 0, "MAX": 5, "VALS": [0, 1, 2, 5]}));
}

#[test]
fn the_labels_of_an_integer_argument_are_named_in_the_losses_with_their_reason() {
    let (_, lost) = exported(&device_with(command_with(json!({"name": "wave", "type": "u7", "values": {"Saw": 2, "Sine": 0}}))));
    let found = lost_at(&lost, "/commands/0/args/0/values");
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(found[0].reason.to_lowercase().contains("label"), "{:?}", found[0]);
}

#[test]
fn an_integer_argument_with_values_and_no_range_has_a_range_of_its_vals_alone() {
    let (ns, _) = exported(&device_with(command_with(json!({"name": "wave", "type": "int", "values": {"B": 3, "A": 1}}))));
    assert_eq!(range_of(&ns, "/trundler/mode"), json!([{"VALS": [1, 3]}]));
}

#[test]
fn a_text_argument_with_values_writes_the_list_of_its_labels_as_vals_and_names_the_integers_in_the_losses() {
    let (ns, lost) = exported(&device_with(command_with(json!({"name": "wave", "type": "string", "values": {"Saw": 2, "Sine": 0, "Square": 1}}))));
    assert_eq!(node_at(&ns, "/trundler/mode").unwrap()["TYPE"], "s");
    assert_eq!(sorted_texts(&vals(&ns, "/trundler/mode")), vec!["Saw", "Sine", "Square"]);
    let found = lost_at(&lost, "/commands/0/args/0/values");
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(found[0].reason.to_lowercase().contains("integer"), "{:?}", found[0]);
}

#[test]
fn an_enum_argument_is_a_text_argument_and_writes_its_labels() {
    let (ns, _) = exported(&device_with(command_with(json!({"name": "wave", "type": "enum", "values": {"x": 0, "y": 1}}))));
    assert_eq!(sorted_texts(&vals(&ns, "/trundler/mode")), vec!["x", "y"]);
}

#[test]
fn an_argument_without_values_writes_no_vals() {
    let (ns, _) = exported(&device_with(command_with(json!({"name": "level", "type": "int", "range": [0, 10]}))));
    assert_eq!(range_of(&ns, "/trundler/mode"), json!([{"MIN": 0, "MAX": 10}]));
}

#[test]
fn a_control_with_values_writes_the_list_of_its_integers_and_names_its_labels_in_the_losses() {
    let (ns, lost) = exported(&device_with(json!({"cc_params": {"entries": [{"osc": "/osc/wave", "cc": 70, "range": [0, 2], "values": {"Saw": 2, "Sine": 0, "Square": 1}}]}})));
    assert_eq!(range_of(&ns, "/trundler/osc/wave"), json!([{"MIN": 0, "MAX": 2, "VALS": [0, 1, 2]}]));
    let found = lost_at(&lost, "/cc_params/entries/0/values");
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(found[0].reason.to_lowercase().contains("label"), "{:?}", found[0]);
}

#[test]
fn a_sysex_parameter_with_values_and_no_range_writes_its_vals_and_no_bound() {
    let (ns, lost) = exported(&device_with(json!({"params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
        "entries": [{"osc": "/p/one", "pr": 1, "p": 2, "c": 3, "r": 4, "values": {"Off": 0, "On": 1}}]}})));
    assert_eq!(range_of(&ns, "/trundler/p/one"), json!([{"VALS": [0, 1]}]));
    assert_eq!(lost_at(&lost, "/params/entries/0/values").len(), 1, "{lost:?}");
}

// ---- export: the bounds of a kind ----

#[test]
fn a_controller_without_a_range_is_exported_with_the_bounds_its_kind_fixes() {
    let (ns, _) = exported(&device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1},
        {"osc": "/b", "cc": 2, "cc_lsb": 34},
        {"osc": "/c", "nrpn_msb": 1, "nrpn_lsb": 1},
        {"osc": "/d", "rpn_msb": 0, "rpn_lsb": 0},
    ]}})));
    assert_eq!(range_of(&ns, "/trundler/a"), json!([{"MIN": 0, "MAX": 127}]));
    for address in ["/trundler/b", "/trundler/c", "/trundler/d"] {
        assert_eq!(range_of(&ns, address), json!([{"MIN": 0, "MAX": 16383}]), "{address}");
    }
}

#[test]
fn a_range_the_descriptor_gives_is_written_as_it_is_even_beside_the_span_of_its_kind() {
    let (ns, _) = exported(&device_with(json!({"cc_params": {"entries": [
        {"osc": "/a", "cc": 1, "range": [10, 100]},
        {"osc": "/b", "cc": 2, "range": [0, 127]},
        {"osc": "/c", "nrpn_msb": 1, "nrpn_lsb": 1, "range": [0, 127]},
    ]}})));
    assert_eq!(range_of(&ns, "/trundler/a"), json!([{"MIN": 10, "MAX": 100}]));
    assert_eq!(range_of(&ns, "/trundler/b"), json!([{"MIN": 0, "MAX": 127}]));
    assert_eq!(range_of(&ns, "/trundler/c"), json!([{"MIN": 0, "MAX": 127}]));
}

#[test]
fn an_entry_of_no_midi_kind_and_without_a_range_is_exported_without_a_bound() {
    let (ns, _) = exported(&device_with(json!({
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
            "entries": [{"osc": "/p/one", "pr": 1, "p": 2, "c": 3, "r": 4}]},
        "commands": [{"osc": "/mix", "args": [{"name": "level", "type": "int"}], "frame": [240, 247]}],
    })));
    for address in ["/trundler/p/one", "/trundler/mix"] {
        assert!(node_at(&ns, address).unwrap().get("RANGE").is_none(), "{address}: a bound the descriptor and MIDI do not give is not invented");
    }
}

// ---- import: VALS ----

fn imported_vals(ty: &str, range: Value) -> (Value, Vec<osd_formats::Loss>) {
    let node = json!({"FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": ty, "RANGE": [range], "EXTENDED_TYPE": ["x"]});
    imported(&namespace(vec![("x", node)]))
}

fn arg0(d: &Value) -> Value {
    d["commands"][0]["args"][0].clone()
}

#[test]
fn a_vals_of_integers_is_a_list_of_permitted_values_the_descriptor_cannot_hold_without_labels_and_names_it() {
    let (d, lost) = imported_vals("i", json!({"MIN": 0, "MAX": 5, "VALS": [0, 2, 5]}));
    assert!(arg0(&d).get("values").is_none(), "no label is invented: {d}");
    assert_eq!(arg0(&d)["range"], json!([0, 5]));
    let found = lost_at(&lost, &format!("{OSC}/RANGE/0/VALS"));
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(found[0].reason.to_lowercase().contains("label"), "{:?}", found[0]);
}

#[test]
fn a_vals_of_texts_is_a_list_the_descriptor_cannot_hold_without_integers_and_names_it() {
    let (d, lost) = imported_vals("s",json!({"VALS": ["Saw", "Sine"]}));
    assert!(arg0(&d).get("values").is_none(), "no integer is invented: {d}");
    let found = lost_at(&lost, &format!("{OSC}/RANGE/0/VALS"));
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(found[0].reason.to_lowercase().contains("integer"), "{:?}", found[0]);
}

#[test]
fn a_vals_that_is_not_a_list_is_named_whole_in_the_losses_with_its_reason_and_nothing_is_carried_of_it() {
    for (what, vals) in [("an object", json!({"Saw": 2, "Sine": 0})), ("a text", json!("Saw")), ("a number", json!(3)), ("a boolean", json!(true))] {
        let (d, lost) = imported_vals("i", json!({"MIN": 0, "MAX": 5, "VALS": vals}));
        assert!(arg0(&d).get("values").is_none(), "{what}: {d}");
        let found = lost_at(&lost, &format!("{OSC}/RANGE/0/VALS"));
        assert_eq!(found.len(), 1, "{what}: {lost:?}");
        assert!(found[0].reason.to_lowercase().contains("list"), "{what}: {:?}", found[0]);
    }
}

#[test]
fn a_vals_of_numbers_that_are_not_integers_is_named_in_the_losses() {
    let (d, lost) = imported_vals("f",json!({"VALS": [0.5, 1.5]}));
    assert!(arg0(&d).get("values").is_none(), "{d}");
    let found = lost_at(&lost, &format!("{OSC}/RANGE/0/VALS"));
    assert_eq!(found.len(), 1, "{lost:?}");
    assert!(!found[0].reason.is_empty());
}

#[test]
fn a_range_with_vals_alone_gives_no_bound_and_a_loss_only_for_the_vals() {
    let (d, lost) = imported_vals("i", json!({"VALS": [1, 2]}));
    assert!(arg0(&d).get("range").is_none(), "{d}");
    assert!(lost_at(&lost, &format!("{OSC}/RANGE/0")).is_empty() && lost_at(&lost, &format!("{OSC}/RANGE")).is_empty(), "{lost:?}");
    assert_eq!(lost_at(&lost, &format!("{OSC}/RANGE/0/VALS")).len(), 1, "{lost:?}");
}

// ---- the round trip of the permitted values ----

#[test]
fn an_integer_argument_with_values_comes_back_with_its_bounds_and_the_loss_of_its_labels_is_named() {
    use crate::common::electra::json_of;
    use osd_formats::{export, import, Format};
    let d = device_with(command_with(json!({"name": "wave", "type": "int", "range": [0, 2], "values": {"Saw": 2, "Sine": 0, "Square": 1}})));
    let out = export(&d, Format::OscQuery).unwrap();
    let t = import(&out.value, Format::OscQuery, &source()).unwrap();
    let back = json_of(&t.value);
    assert_eq!(back["commands"][0]["args"][0]["range"], json!([0, 2]));
    assert!(back["commands"][0]["args"][0].get("values").is_none(), "no label comes back: {back}");
    assert_eq!(lost_at(&out.lost, "/commands/0/args/0/values").len(), 1, "{:?}", out.lost);
    assert_eq!(t.lost.iter().filter(|l| l.place.ends_with("/RANGE/0/VALS")).count(), 1, "{:?}", t.lost);
}
