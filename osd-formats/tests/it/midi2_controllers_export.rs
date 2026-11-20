//! An OSD device exports to the Property Exchange controller resources (M2-117-UM): one
//! `ChCtrlList` entry per control, the named values in a `CtrlMapList`, and what the format has
//! no property for named in the losses.

use crate::common::midi2::*;
use crate::common::midnam::{device_with, lost_at};
use osd_formats::{export, FaultKind, Format};
use serde_json::json;

#[test]
fn a_cc_becomes_an_entry_with_its_title_type_index_range_default_priority_and_modes() {
    let d = cc_device(json!([{
        "osc": "/filter/cutoff", "cc": 74, "range": [10, 100], "default": 50, "priority": 2,
        "transmit": "absolute", "recognize": "both", "step_count": 91, "channel": 3,
        "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}
    }]));
    let (v, _) = exported(&d);
    let e = &list(&v)[0];
    assert_eq!(e["title"], "/Filter/Cutoff");
    assert_eq!(e["paramPath"], "/Filter/Cutoff");
    assert_eq!(e["ctrlType"], "cc");
    assert_eq!(e["ctrlIndex"], json!([74]));
    assert_eq!(e["minMax"], json!([10, 100]));
    assert_eq!(e["default"], 50);
    assert_eq!(e["priority"], 2);
    assert_eq!(e["transmit"], "absolute");
    assert_eq!(e["recognize"], "both");
    assert_eq!(e["stepCount"], 91);
    assert!(e["channel"].is_number(), "{e}");
}

#[test]
fn the_object_written_holds_a_chctrllist_and_a_ctrlmaplist() {
    let (v, _) = exported(&cc_device(json!([{"osc": "/a", "cc": 1}])));
    let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["ChCtrlList", "CtrlMapList"]);
    assert!(v["CtrlMapList"].as_array().unwrap().is_empty());
}

#[test]
fn each_kind_of_control_is_an_entry_of_its_type_with_its_index() {
    let d = device(json!({
        "cc_params": {"entries": [
            {"osc": "/a", "cc": 7},
            {"osc": "/b", "cc": 1, "cc_lsb": 33, "range": [0, 16383]},
            {"osc": "/c", "nrpn_msb": 3, "nrpn_lsb": 9},
            {"osc": "/d", "rpn_msb": 0, "rpn_lsb": 2},
        ]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
                   "entries": [{"osc": "/e", "id": 12, "range": [0, 63]}]},
    }));
    let (v, _) = exported(&d);
    let l = list(&v);
    assert_eq!(l.len(), 4, "the SysEx param is not written: {v}");
    let kinds: Vec<(&str, &serde_json::Value)> = l.iter().map(|e| (e["ctrlType"].as_str().unwrap(), &e["ctrlIndex"])).collect();
    assert_eq!(kinds[0], ("cc", &json!([7])));
    assert_eq!(kinds[1], ("cc", &json!([1])));
    assert_eq!(kinds[2], ("nrpn", &json!([3, 9])));
    assert_eq!(kinds[3], ("rpn", &json!([0, 2])));
}

const STANDARD_TYPES: [&str; 9] = ["cc", "chPress", "pPress", "nrpn", "rpn", "pBend", "pnrc", "pnac", "pnp"];

fn device_with_three_params() -> osd::Resolved {
    device(json!({
        "cc_params": {"entries": [{"osc": "/a", "cc": 7}]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
                   "entries": [
                       {"osc": "/p1", "id": 1, "range": [0, 63]},
                       {"osc": "/p2", "id": 2, "expects": "switch"},
                       {"osc": "/p3", "p": 3, "pr": 9},
                   ]},
    }))
}

#[test]
fn an_export_writes_no_entry_outside_the_nine_types_of_the_standard() {
    let (v, _) = exported(&device_with_three_params());
    for e in list(&v) {
        assert!(STANDARD_TYPES.contains(&e["ctrlType"].as_str().unwrap()), "{e}");
    }
    assert_eq!(list(&v).len(), 1, "{v}");
}

#[test]
fn the_sysex_params_are_one_loss_at_params_with_their_count_and_none_per_entry() {
    let (_, lost) = exported(&device_with_three_params());
    let at: Vec<&osd_formats::Loss> = lost.iter().filter(|l| l.place == "/params" || l.place.starts_with("/params/")).collect();
    assert_eq!(at.len(), 1, "{lost:?}");
    assert_eq!(at[0].place, "/params");
    assert!(at[0].reason.contains('3'), "the count of entries not written: {:?}", at[0]);
}

#[test]
fn a_device_without_params_names_no_loss_for_them() {
    let (_, lost) = exported(&cc_device(json!([{"osc": "/a", "cc": 7}])));
    assert!(!lost.iter().any(|l| l.place.starts_with("/params")), "{lost:?}");
}

#[test]
fn the_number_of_significant_bits_follows_the_range_and_the_pair_of_a_14_bit_cc() {
    let d = cc_device(json!([
        {"osc": "/a", "cc": 7, "range": [0, 127]},
        {"osc": "/b", "cc": 8, "range": [0, 255]},
        {"osc": "/c", "cc": 9, "cc_lsb": 41, "range": [0, 16383]},
        {"osc": "/d", "nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]},
    ]));
    let (v, _) = exported(&d);
    let bits: Vec<i64> = list(&v).iter().map(|e| e["numSigBits"].as_i64().unwrap()).collect();
    assert_eq!(bits, [7, 8, 14, 14]);
}

#[test]
fn the_type_hint_follows_the_expects_word_by_the_table_of_the_controls_document() {
    let d = cc_device(json!([
        {"osc": "/a", "cc": 1, "expects": "continuous"},
        {"osc": "/b", "cc": 2, "expects": "switch"},
        {"osc": "/c", "cc": 3, "expects": "momentary"},
        {"osc": "/d", "cc": 4, "expects": "discrete", "values": {"Off": 0, "On": 1}},
        {"osc": "/e", "cc": 5, "expects": "trigger"},
        {"osc": "/f", "cc": 6, "expects": "clock"},
    ]));
    let (v, _) = exported(&d);
    let l = list(&v);
    let hints: Vec<Option<&str>> = l.iter().map(|e| e["typeHint"].as_str()).collect();
    assert_eq!(hints, [Some("continuous"), Some("toggle"), Some("momentary"), Some("valueSelect"), Some("momentary"), None]);
}

#[test]
fn a_control_with_values_gets_a_map_and_equal_value_lists_share_one_id() {
    let d = cc_device(json!([
        {"osc": "/a", "cc": 1, "range": [0, 2], "values": {"Saw": 0, "Square": 1, "Tri": 2}},
        {"osc": "/b", "cc": 2, "range": [0, 2], "values": {"Tri": 2, "Saw": 0, "Square": 1}},
        {"osc": "/c", "cc": 3, "range": [0, 1], "values": {"Off": 0, "On": 1}},
        {"osc": "/d", "cc": 4},
    ]));
    let (v, _) = exported(&d);
    let l = list(&v);
    assert_eq!(v["CtrlMapList"].as_array().unwrap().len(), 2);
    assert_eq!(l[0]["ctrlMapId"], l[1]["ctrlMapId"]);
    assert_ne!(l[0]["ctrlMapId"], l[2]["ctrlMapId"]);
    assert!(l[3].get("ctrlMapId").is_none(), "{}", l[3]);
    assert_eq!(named(&v, &l[0]), [(0, "Saw".to_string()), (1, "Square".to_string()), (2, "Tri".to_string())]);
    assert_eq!(named(&v, &l[2]), [(0, "Off".to_string()), (1, "On".to_string())]);
}

#[test]
fn the_title_is_the_role_path_else_a_section_starting_with_a_slash_else_the_osc_address() {
    let d = cc_device(json!([
        {"osc": "/a", "cc": 1, "section": "/Amp/Gain", "role": {"parts": [{"part": "oscillator", "index": 1}], "setting": "level"}},
        {"osc": "/b", "cc": 2, "section": "/Mod/Depth"},
        {"osc": "/c", "cc": 3, "section": "Filter"},
        {"osc": "/d", "cc": 4},
    ]));
    let (v, _) = exported(&d);
    let l = list(&v);
    let titles: Vec<&str> = l.iter().map(|e| e["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["/Oscillator/1/Level", "/Mod/Depth", "/c", "/d"]);
}

#[test]
fn a_section_is_written_back_as_a_param_path_only_for_a_control_without_a_role_and_only_when_it_starts_with_a_slash() {
    let d = cc_device(json!([
        {"osc": "/a", "cc": 1, "section": "/Amp/Gain", "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}},
        {"osc": "/b", "cc": 2, "section": "/Mod/Depth"},
        {"osc": "/c", "cc": 3, "section": "Filter"},
    ]));
    let (v, _) = exported(&d);
    let l = list(&v);
    assert_eq!(l[0]["paramPath"], "/Filter/Cutoff");
    assert_eq!(l[1]["paramPath"], "/Mod/Depth");
    assert!(l[2].get("paramPath").is_none(), "a free section is never a paramPath: {}", l[2]);
}

#[test]
fn a_role_of_two_parts_is_written_as_its_param_path() {
    let d = cc_device(json!([
        {"osc": "/a", "cc": 1, "role": {"parts": [{"part": "amplifier"}, {"part": "envelope"}], "setting": "attack-time"}},
    ]));
    let (v, _) = exported(&d);
    assert_eq!(list(&v)[0]["paramPath"], "/Amplifier/Envelope/AttackTime");
}

#[test]
fn the_losses_name_the_trigger_the_clock_and_each_field_the_format_has_no_property_for() {
    let d = device(json!({"capabilities": ["lua"], "cc_params": {"entries": [
        {"osc": "/a", "cc": 1, "expects": "trigger"},
        {"osc": "/b", "cc": 2, "expects": "clock"},
        {"osc": "/c", "cc": 3, "orientation": "centered", "transform": "x*2", "transform_reverse": "x/2"},
        {"osc": "/d", "cc": 4},
    ]}}));
    let (_, lost) = exported(&d);
    let trigger = lost_at(&lost, "/cc_params/entries/0/expects");
    assert_eq!(trigger.len(), 1, "{lost:?}");
    assert!(trigger[0].reason.contains("momentary"), "{:?}", trigger[0]);
    let clock = lost_at(&lost, "/cc_params/entries/1/expects");
    assert_eq!(clock.len(), 1, "{lost:?}");
    assert!(clock[0].reason.contains("typeHint"), "{:?}", clock[0]);
    for field in ["orientation", "transform", "transform_reverse"] {
        assert_eq!(lost_at(&lost, &format!("/cc_params/entries/2/{field}")).len(), 1, "{field}: {lost:?}");
    }
    assert!(!lost.iter().any(|l| l.place.starts_with("/cc_params/entries/3")), "{lost:?}");
}

#[test]
fn the_credit_and_licence_are_in_the_text_or_the_losses_list_the_sources() {
    let d = cc_device(json!([{"osc": "/a", "cc": 1}]));
    let out = export(&d, Format::Midi2Controllers).unwrap();
    let in_text = out.value.contains("CC-BY-4.0");
    let in_losses = out.lost.iter().any(|l| l.place == "/_sources");
    assert!(in_text || in_losses, "{out:?}");
    if in_losses {
        assert_eq!(osd_formats::credit_to_carry(&d, &out.lost).len(), 1);
    }
}

#[test]
fn a_device_without_sources_is_refused() {
    let d = device_with(json!({"_sources": null, "cc_params": {"entries": [{"osc": "/a", "cc": 1}]}}));
    let fault = export(&d, Format::Midi2Controllers).expect_err("refused");
    assert_eq!(fault.kind, FaultKind::NoSources);
}

#[test]
fn the_entries_follow_the_order_of_the_controls_and_a_device_without_controls_gives_an_empty_list() {
    let d = cc_device(json!([{"osc": "/z", "cc": 9}, {"osc": "/a", "cc": 3}, {"osc": "/m", "cc": 5}]));
    let (v, _) = exported(&d);
    let idx: Vec<i64> = list(&v).iter().map(|e| e["ctrlIndex"][0].as_i64().unwrap()).collect();
    assert_eq!(idx, [9, 3, 5]);
    let (empty, _) = exported(&device(json!({})));
    assert!(list(&empty).is_empty());
}
