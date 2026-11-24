//! A device exported to the Electra One is a preset a musician plays at once: readable names,
//! the maker's list or pad where a setting has positions, groups that frame their own controls,
//! one way per control chosen by the descriptor's order, and the lists read back on import.

use crate::common::electra::*;
use crate::common::resolved;
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

fn exported(device: &osd::Resolved) -> (Value, Vec<osd_formats::Loss>) {
    let t = export(device, Format::ElectraOne).expect("the device exports");
    (serde_json::from_str(&t.value).expect("the export is JSON"), t.lost)
}

fn names(p: &Value) -> Vec<String> {
    p["controls"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap().to_string()).collect()
}

fn overlay_of(p: &Value, control: usize) -> &Value {
    let id = &p["controls"][control]["values"][0]["overlayId"];
    p["overlays"].as_array().unwrap().iter().find(|o| o["id"] == *id).unwrap_or_else(|| panic!("no overlay {id}"))
}

fn labels(p: &Value, control: usize) -> Vec<(i64, String)> {
    overlay_of(p, control)["items"].as_array().unwrap().iter().map(|i| (i["value"].as_i64().unwrap(), i["label"].as_str().unwrap().to_string())).collect()
}

// ---- names ------------------------------------------------------------------------------

#[test]
fn a_control_without_a_name_is_labelled_by_the_last_segment_of_its_address_made_readable() {
    let entries = json!([
        {"osc": "/oscillator_1/octave", "cc": 1},
        {"osc": "/osc/frequency_mode", "cc": 2},
        {"osc": "/filter/cutoff-freq", "cc": 3},
        {"osc": "/lfo/rate", "cc": 4},
    ]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(names(&p), vec!["Octave", "Frequency mode", "Cutoff freq", "Rate"]);
}

#[test]
fn a_control_with_a_role_is_labelled_by_the_setting_of_its_role() {
    let entries = json!([
        {"osc": "/p/x1", "cc": 1, "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}},
        {"osc": "/p/x2", "cc": 2},
    ]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(names(&p), vec!["Cutoff", "X2"]);
}

#[test]
fn the_name_a_preset_gave_a_control_wins_over_its_role_and_its_address() {
    let entries = json!([{"osc": "/p/x1", "cc": 1, "_name": "Brightness", "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}}]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(names(&p), vec!["Brightness"]);
}

#[test]
fn two_controls_of_a_group_never_share_a_label_and_two_groups_may() {
    let entries = json!([
        {"osc": "/osc1/octave", "cc": 1, "section": "Oscillators"},
        {"osc": "/osc2/octave", "cc": 2, "section": "Oscillators"},
        {"osc": "/osc3/octave", "cc": 3, "section": "Oscillators"},
        {"osc": "/lfo/octave", "cc": 4, "section": "Lfo"},
    ]);
    let (p, _) = exported(&device6(entries));
    let n = names(&p);
    assert_eq!(n[3], "Octave", "{n:?}");
    let mut group: Vec<&String> = n[..3].iter().collect();
    group.sort();
    group.dedup();
    assert_eq!(group.len(), 3, "{n:?}");
    assert!(n.iter().all(|x| x.starts_with("Octave")), "{n:?}");
}

#[test]
fn a_label_is_within_the_maker_s_length_and_stays_unique_once_cut() {
    let long = "a".repeat(60);
    let entries = json!([
        {"osc": format!("/s/{long}1"), "cc": 1, "section": "S"},
        {"osc": format!("/s/{long}2"), "cc": 2, "section": "S"},
        {"osc": "/s/ok", "cc": 3, "_name": "é".repeat(70), "section": "S"},
    ]);
    let (p, _) = exported(&device6(entries));
    let n = names(&p);
    assert!(n.iter().all(|x| x.chars().count() <= CONTROL_NAME_MAX), "{n:?}");
    assert_ne!(n[0], n[1], "{n:?}");
    assert_eq!(unplayable(&p), Vec::<String>::new());
}

#[test]
fn a_sysex_command_is_labelled_the_same_way() {
    let doc = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "T", "osc_prefix": "/t"},
        "commands": [{"osc": "/dump/all_sounds", "frame": [240, 247]}],
    });
    let (p, _) = exported(&resolved(crate::common::device_from(&doc.to_string())));
    assert_eq!(names(&p), vec!["All sounds"]);
}

// ---- lists and pads ---------------------------------------------------------------------

#[test]
fn a_control_whose_values_name_its_positions_is_a_list_with_its_labels_in_an_overlay() {
    let entries = json!([{"osc": "/lfo/wave", "cc": 9, "range": [0, 3], "values": {"Sine": 0, "Square": 2, "Saw": 1}}]);
    let (p, _) = exported(&device6(entries));
    let c = &p["controls"][0];
    assert_eq!(c["type"], "list");
    assert_eq!((c["values"][0]["min"].clone(), c["values"][0]["max"].clone()), (json!(0), json!(3)));
    let mut l = labels(&p, 0);
    l.sort();
    assert_eq!(l, vec![(0, "Sine".to_string()), (1, "Saw".to_string()), (2, "Square".to_string())]);
    assert_eq!(unplayable(&p), Vec::<String>::new());
    assert_eq!(violations(&p), Vec::<String>::new());
}

#[test]
fn a_discrete_control_of_three_positions_without_values_is_a_list_labelled_by_its_numbers() {
    let entries = json!([{"osc": "/osc/frequency_mode", "cc": 20, "range": [0, 2], "expects": "discrete"}]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(p["controls"][0]["type"], "list");
    assert_eq!(labels(&p, 0), vec![(0, "0".to_string()), (1, "1".to_string()), (2, "2".to_string())]);
}

#[test]
fn a_list_holds_as_many_labels_as_the_range_has_positions() {
    let entries = json!([{"osc": "/p/wide", "cc": 20, "range": [0, 127], "expects": "discrete"}]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(p["controls"][0]["type"], "list");
    assert_eq!(labels(&p, 0).len(), 128);
}

#[test]
fn a_list_label_is_cut_to_the_maker_s_length_and_two_cut_labels_stay_apart() {
    let (a, b) = ("x".repeat(25) + "1", "x".repeat(25) + "2");
    let entries = json!([{"osc": "/p/w", "cc": 20, "range": [0, 2], "values": {a: 0, b: 1, "Short": 2}}]);
    let (p, _) = exported(&device6(entries));
    let l = labels(&p, 0);
    assert!(l.iter().all(|(_, s)| s.chars().count() <= LABEL_MAX), "{l:?}");
    let mut s: Vec<&String> = l.iter().map(|(_, s)| s).collect();
    s.sort();
    s.dedup();
    assert_eq!(s.len(), 3, "{l:?}");
}

#[test]
fn a_switch_a_momentary_a_trigger_and_a_discrete_of_two_positions_are_pads() {
    let entries = json!([
        {"osc": "/p/a", "cc": 1, "expects": "switch", "range": [0, 1]},
        {"osc": "/p/b", "cc": 2, "expects": "momentary"},
        {"osc": "/p/c", "cc": 3, "expects": "trigger"},
        {"osc": "/p/d", "cc": 4, "expects": "discrete", "range": [0, 1]},
    ]);
    let (p, _) = exported(&device6(entries));
    let types: Vec<&Value> = p["controls"].as_array().unwrap().iter().map(|c| &c["type"]).collect();
    assert_eq!(types, vec!["pad", "pad", "pad", "pad"]);
    assert!(p["overlays"].as_array().unwrap().is_empty());
}

#[test]
fn a_discrete_control_of_two_positions_with_named_values_is_a_list() {
    let entries = json!([{"osc": "/p/a", "cc": 1, "expects": "discrete", "range": [0, 1], "values": {"Off": 0, "On": 1}}]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(p["controls"][0]["type"], "list");
    assert_eq!(labels(&p, 0), vec![(0, "Off".to_string()), (1, "On".to_string())]);
}

#[test]
fn named_values_win_over_a_switch_and_every_other_control_stays_a_fader() {
    let entries = json!([
        {"osc": "/p/a", "cc": 1, "expects": "switch", "range": [0, 1], "values": {"Off": 0, "On": 1}},
        {"osc": "/p/b", "cc": 2, "expects": "continuous", "range": [0, 127]},
        {"osc": "/p/c", "cc": 3},
        {"osc": "/p/d", "cc": 4, "expects": "discrete"},
        {"osc": "/p/e", "cc": 5, "expects": "clock"},
    ]);
    let (p, _) = exported(&device6(entries));
    let types: Vec<&Value> = p["controls"].as_array().unwrap().iter().map(|c| &c["type"]).collect();
    assert_eq!(types, vec!["list", "fader", "fader", "fader", "fader"]);
}

#[test]
fn each_list_has_its_own_overlay_with_one_label_per_position() {
    let entries = json!([
        {"osc": "/p/a", "cc": 1, "range": [0, 2], "expects": "discrete"},
        {"osc": "/p/b", "cc": 2, "range": [0, 3], "expects": "discrete"},
    ]);
    let (p, _) = exported(&device6(entries));
    let (a, b) = (&p["controls"][0]["values"][0]["overlayId"], &p["controls"][1]["values"][0]["overlayId"]);
    assert_ne!(a, b);
    assert_eq!(labels(&p, 1).len(), 4);
    assert_eq!(violations(&p), Vec::<String>::new());
}

// ---- groups -----------------------------------------------------------------------------

/// Sections of the sizes of a real synthesizer's panel: some spread over two rows, one over a page.
fn panel() -> osd::Resolved {
    let sizes = [("Oscillator 1", 4), ("Oscillator 2", 8), ("Mixer", 3), ("Filter", 15), ("Envelope", 7), ("Lfo", 2), ("Misc", 14)];
    let mut entries = Vec::new();
    for (name, n) in sizes {
        for k in 0..n {
            entries.push(json!({"osc": format!("/{}/p{k}", name.to_lowercase().replace(' ', "_")), "cc": entries.len() % 128, "section": name}));
        }
    }
    entries.push(json!({"osc": "/free/one", "cc": 127}));
    device6(json!(entries))
}

#[test]
fn no_two_groups_of_a_page_overlap_and_each_frames_its_own_controls_and_no_other() {
    let (p, _) = exported(&panel());
    assert!(p["pages"].as_array().unwrap().len() >= 2);
    assert_eq!(unplayable(&p), Vec::<String>::new());
    assert_eq!(violations(&p), Vec::<String>::new());
}

#[test]
fn a_group_that_spreads_over_two_rows_leaves_a_new_row_to_the_next_group() {
    let entries: Vec<Value> = (0..9)
        .map(|n| json!({"osc": format!("/a/p{n}"), "cc": n, "section": if n < 8 { "Wide" } else { "Next" }}))
        .collect();
    let (p, _) = exported(&device6(json!(entries)));
    let groups = p["groups"].as_array().unwrap();
    let bounds = |name: &str| groups.iter().find(|g| g["name"] == name).unwrap()["bounds"].clone();
    let (wide, next) = (bounds("Wide"), bounds("Next"));
    assert!(next[1].as_i64().unwrap() >= wide[1].as_i64().unwrap() + wide[3].as_i64().unwrap(), "{wide} {next}");
    assert_eq!(unplayable(&p), Vec::<String>::new());
}

#[test]
fn a_group_is_whole_cells_of_the_grid_and_its_controls_keep_their_slots() {
    let (p, _) = exported(&panel());
    for g in p["groups"].as_array().unwrap() {
        assert!(g["bounds"].as_array().unwrap().iter().all(|n| n.as_i64().is_some_and(|n| n >= 0)), "{g}");
    }
    let mut slots: Vec<(u64, u64, u64)> = p["controls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["pageId"].as_u64().unwrap(), c["controlSetId"].as_u64().unwrap(), c["inputs"][0]["potId"].as_u64().unwrap()))
        .collect();
    let n = slots.len();
    slots.sort();
    slots.dedup();
    assert_eq!(slots.len(), n);
}

// ---- credit and ways --------------------------------------------------------------------

#[test]
fn the_credit_and_the_licence_stay_in_the_losses_because_the_preset_shows_no_text_for_them() {
    let (p, lost) = exported(&device6(json!([{"osc": "/a/b", "cc": 1}])));
    let l = lost.iter().find(|l| l.place == "/_sources").expect("the credit is named in the losses");
    assert!(l.reason.contains("credit") && l.reason.contains("licence"), "{}", l.reason);
    assert!(!p.to_string().contains("CC0-1.0"));
}

#[test]
fn a_control_with_several_ways_is_written_by_the_first_way_with_its_own_range() {
    let entries = json!([{
        "osc": "/filter/cutoff", "cc": 74, "range": [0, 127],
        "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383]}, {"cc": 5, "cc_lsb": 37, "range": [0, 16383]}],
    }]);
    let (p, lost) = exported(&device6(entries));
    let v = &p["controls"][0]["values"][0];
    assert_eq!((v["message"]["type"].clone(), v["message"]["parameterNumber"].clone()), (json!("cc7"), json!(74)));
    assert_eq!((v["min"].clone(), v["max"].clone()), (json!(0), json!(127)));
    for place in ["/cc_params/entries/0/also/0", "/cc_params/entries/0/also/1"] {
        let l = lost.iter().find(|l| l.place == place).unwrap_or_else(|| panic!("{place}: {lost:?}"));
        assert!(l.reason.contains("cc7"), "{}", l.reason);
    }
    assert!(lost.iter().any(|l| l.place == "/cc_params/entries/0/also/0" && l.reason.contains("nrpn")));
    assert!(lost.iter().any(|l| l.place == "/cc_params/entries/0/also/1" && l.reason.contains("cc14")));
}

#[test]
fn the_export_never_reorders_the_ways_a_finer_way_first_is_written_first() {
    let entries = json!([{
        "osc": "/filter/cutoff", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383],
        "also": [{"cc": 74, "range": [0, 127]}],
    }]);
    let (p, lost) = exported(&device6(entries));
    let v = &p["controls"][0]["values"][0];
    assert_eq!((v["message"]["type"].clone(), v["message"]["parameterNumber"].clone()), (json!("nrpn"), json!(130)));
    assert_eq!((v["min"].clone(), v["max"].clone()), (json!(0), json!(16383)));
    assert!(lost.iter().any(|l| l.place == "/cc_params/entries/0/also/0" && l.reason.contains("nrpn")), "{lost:?}");
}

// ---- import and round trip --------------------------------------------------------------

fn list_preset() -> Value {
    let mut p = preset(&[], vec![control(1, "Wave", None, cc7(9), Some((0, 2)))]);
    p["controls"][0]["type"] = json!("list");
    p["controls"][0]["values"][0]["overlayId"] = json!(1);
    p["overlays"] = json!([{"id": 1, "items": [{"value": 0, "label": "Sine"}, {"value": 1, "label": "Saw"}, {"value": 2, "label": "Square"}]}]);
    p
}

#[test]
fn the_labels_of_a_list_are_read_as_the_values_of_its_control_and_are_no_loss() {
    let t = import(&list_preset().to_string(), Format::ElectraOne, &source()).expect("imports");
    let e = &t.value.cc_params.as_ref().unwrap().entries[0];
    let values = e.values.as_ref().expect("the labels are the values of the control");
    assert_eq!(values.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>(), vec![("Saw", 1), ("Sine", 0), ("Square", 2)]);
    assert!(t.lost.iter().all(|l| !l.place.starts_with("/overlays") && !l.place.ends_with("/overlayId")), "{:?}", t.lost);
}

#[test]
fn a_list_no_control_uses_stays_a_loss_of_the_import() {
    let mut p = list_preset();
    p["overlays"].as_array_mut().unwrap().push(json!({"id": 2, "items": [{"value": 0, "label": "Stray"}]}));
    let t = import(&p.to_string(), Format::ElectraOne, &source()).expect("imports");
    assert!(t.lost.iter().any(|l| l.place.starts_with("/overlays")), "{:?}", t.lost);
    let mut q = list_preset();
    q["controls"][0]["values"][0].as_object_mut().unwrap().remove("overlayId");
    let t = import(&q.to_string(), Format::ElectraOne, &source()).expect("imports");
    assert!(t.lost.iter().any(|l| l.place.starts_with("/overlays")), "{:?}", t.lost);
}

#[test]
fn a_list_written_by_the_export_comes_back_through_import_and_export_with_its_labels() {
    let entries = json!([
        {"osc": "/lfo/wave", "cc": 9, "range": [0, 3], "values": {"Sine": 0, "Saw": 1, "Square": 2}},
        {"osc": "/osc/mode", "cc": 20, "range": [0, 2], "expects": "discrete"},
    ]);
    let (first, _) = exported(&device6(entries));
    let t = import(&first.to_string(), Format::ElectraOne, &source()).expect("imports");
    assert!(t.lost.iter().all(|l| !l.place.starts_with("/overlays")), "{:?}", t.lost);
    let (second, _) = exported(&resolved(t.value));
    assert_eq!(labels(&second, 0), labels(&first, 0));
    assert_eq!(labels(&second, 1), labels(&first, 1));
    assert_eq!(second["controls"][0]["type"], "list");
    assert_eq!(names(&second), names(&first));
}
