//! A list imported then exported gives back the same entries, compared after parsing, field by
//! field on what the format carries; the title and the numbering of the map ids are ignored.

use crate::common::midi2::*;
use crate::common::{electra::json_of, midnam::device_with, resolved};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

/// An entry as the format carries it: without its title, its map id replaced by the map.
fn canon(v: &Value) -> Vec<Value> {
    list(v)
        .iter()
        .map(|e| {
            let mut e = e.clone();
            let map = named(v, &e);
            e.as_object_mut().unwrap().remove("title");
            e.as_object_mut().unwrap().remove("ctrlMapId");
            if !map.is_empty() {
                e["map"] = json!(map);
            }
            e
        })
        .collect()
}

fn back(text: &str) -> Value {
    let t = import(text, Format::Midi2Controllers, &source()).unwrap_or_else(|f| panic!("{f}\n{text}"));
    let d = resolved(t.value);
    let out = export(&d, Format::Midi2Controllers).unwrap_or_else(|f| panic!("{f}"));
    serde_json::from_str(&out.value).unwrap()
}

fn sample() -> Value {
    json!({
        "ChCtrlList": [
            {"title": "Cutoff", "ctrlType": "cc", "ctrlIndex": [74], "numSigBits": 7, "minMax": [167772160u64, 4056358001u64], "default": 2352005900u64, "priority": 2,
             "transmit": "absolute", "recognize": "both", "stepCount": 91, "channel": 3, "typeHint": "continuous", "paramPath": "/Filter/Cutoff"},
            {"title": "Fine", "ctrlType": "cc", "ctrlIndex": [1], "numSigBits": 14, "minMax": [0u64, 4294967295u64], "default": 2147483648u64},
            {"title": "Wide", "ctrlType": "cc", "ctrlIndex": [20], "numSigBits": 32, "minMax": [0u64, 4294967295u64], "default": 3000000000u64},
            {"title": "Wave", "ctrlType": "nrpn", "ctrlIndex": [3, 9], "numSigBits": 14, "minMax": [0u64, 4294967295u64], "typeHint": "valueSelect", "ctrlMapId": "w",
             "paramPath": "/Mystery/Wave"},
            {"title": "Alt", "ctrlType": "rpn", "ctrlIndex": [0, 2], "numSigBits": 14, "minMax": [0u64, 4294705152u64], "typeHint": "valueSelect", "ctrlMapId": "x"},
            {"title": "On", "ctrlType": "cc", "ctrlIndex": [9], "numSigBits": 7, "minMax": [0u64, 4294967295u64], "typeHint": "toggle", "paramPath": "/Oscillator/1/Level"},
        ],
        "CtrlMapList": [
            {"ctrlMapId": "w", "ctrlMap": [{"value": 0, "title": "Saw"}, {"value": 1, "title": "Sq"}, {"value": 2, "title": "Tri"}]},
            {"ctrlMapId": "x", "ctrlMap": [{"value": 0, "title": "Saw"}, {"value": 1, "title": "Sq"}, {"value": 2, "title": "Tri"}]},
        ],
    })
}

#[test]
fn a_list_imported_then_exported_gives_back_the_same_entries_in_the_same_order() {
    let text = sample().to_string();
    assert_eq!(canon(&back(&text)), canon(&sample()));
}

#[test]
fn the_same_holds_for_a_bare_array() {
    let s = sample();
    let bare = s["ChCtrlList"].clone();
    // the maps are in the object only: keep the entries that need none
    let plain: Vec<Value> = bare.as_array().unwrap().iter().filter(|e| e.get("ctrlMapId").is_none()).cloned().collect();
    let original = json!({"ChCtrlList": plain});
    assert_eq!(canon(&back(&json!(plain).to_string())), canon(&original));
}

#[test]
fn a_list_exported_twice_through_an_import_is_stable() {
    let once = back(&sample().to_string());
    let again = back(&once.to_string());
    assert_eq!(canon(&again), canon(&once));
}

#[test]
fn a_device_exported_then_imported_keeps_what_the_format_carries() {
    let d = device_with(json!({
        "role_vocabulary": "1",
        "cc_params": {"entries": [
            {"osc": "/a", "cc": 74, "range": [10, 100], "default": 50, "priority": 2, "transmit": "absolute", "recognize": "relative",
             "step_count": 91, "channel": 3, "expects": "continuous", "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}},
            {"osc": "/b", "cc": 5, "cc_lsb": 37, "range": [0, 16383], "section": "/Free/Path"},
            {"osc": "/c", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 2], "expects": "discrete", "values": {"A": 0, "B": 1, "C": 2}},
        ]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
                   "entries": [{"osc": "/d", "id": 7, "range": [0, 63], "expects": "switch"}]},
    }));
    let out = export(&d, Format::Midi2Controllers).unwrap();
    let v: Value = serde_json::from_str(&out.value).unwrap();
    let t = import(&out.value, Format::Midi2Controllers, &source()).unwrap();
    let json = json_of(&t.value);
    let kept = controls(&json);
    assert_eq!(kept.len(), 3);
    assert!(json.get("params").is_none(), "no params block with made-up frames: {json}");
    assert_eq!(kept[0]["ways"][0]["ctrlIndex"], json!([74]));
    assert_eq!(kept[0]["ways"][0]["range"], json!([10, 100]));
    assert_eq!(kept[0]["default"], 50);
    assert_eq!(kept[0]["channel"], 3);
    assert_eq!(kept[0]["role"]["setting"], "cutoff");
    assert_eq!(kept[0]["ways"][0]["recognize"], "relative");
    assert_eq!(kept[1]["ways"][0]["ctrlIndex"], json!([5, 37]));
    assert_eq!(kept[1]["section"], "/Free/Path");
    let mut values = kept[2]["values"].as_array().unwrap().clone();
    values.sort_by_key(|v| v["value"].as_i64());
    assert_eq!(values, vec![json!({"value": 0, "title": "A"}), json!({"value": 1, "title": "B"}), json!({"value": 2, "title": "C"})]);
    assert_eq!(canon(&back(&out.value)), canon(&v));
}

#[test]
fn the_title_and_the_numbering_of_the_map_ids_are_the_only_things_a_round_trip_may_change() {
    let original = sample();
    let again = back(&original.to_string());
    let (a, b) = (list(&original), list(&again));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        let mut keys_x: Vec<&String> = x.as_object().unwrap().keys().filter(|k| !["title", "ctrlMapId"].contains(&k.as_str())).collect();
        let mut keys_y: Vec<&String> = y.as_object().unwrap().keys().filter(|k| !["title", "ctrlMapId"].contains(&k.as_str())).collect();
        keys_x.sort();
        keys_y.sort();
        assert_eq!(keys_x, keys_y);
    }
    // the two maps of the sample hold the same values: they come back as one
    assert_eq!(again["CtrlMapList"].as_array().unwrap().len(), 1);
}

#[test]
fn a_title_and_a_description_survive_an_import_then_an_export() {
    let text = json!([
        {"title": "Cutoff", "description": "Opens the filter", "ctrlType": "cc", "ctrlIndex": [74], "numSigBits": 7},
        {"title": "Rate", "description": "", "ctrlType": "cc", "ctrlIndex": [75], "numSigBits": 7, "paramPath": "/Mystery/Rate"},
    ])
    .to_string();
    let l = list(&back(&text));
    assert_eq!(l[0]["title"], "Cutoff");
    assert_eq!(l[0]["description"], "Opens the filter");
    assert_eq!(l[1]["title"], "Rate");
    assert_eq!(l[1]["description"], "");
}

#[test]
fn a_control_without_a_title_comes_back_with_its_derived_title_as_an_explicit_title() {
    let d = cc_device(json!([{"osc": "/c", "cc": 3, "section": "/Mod/Depth"}, {"osc": "/d", "cc": 4}]));
    let (v, _) = exported(&d);
    let t = import(&v.to_string(), Format::Midi2Controllers, &source()).unwrap();
    let json = json_of(&t.value);
    let kept = controls(&json);
    assert_eq!(kept[0]["title"], "/Mod/Depth");
    assert_eq!(kept[1]["title"], "/d");
    assert!(kept[1].get("description").is_none());
    let again = list(&back(&v.to_string()));
    assert_eq!(again[0]["title"], "/Mod/Depth");
    assert_eq!(again[1]["title"], "/d");
}

#[test]
fn the_channel_range_and_default_of_7_14_and_32_bit_controls_survive_an_export_then_an_import() {
    let d = device_with(json!({
        "role_vocabulary": "1",
        "cc_params": {"entries": [
            {"osc": "/a", "cc": 7, "range": [0, 127], "default": 100, "channel": 0},
            {"osc": "/b", "cc": 1, "cc_lsb": 33, "range": [0, 16383], "default": 12000, "channel": 15},
            {"osc": "/c", "cc": 20, "range": [0, 4294967295u64], "default": 3000000000u64, "channel": 7},
            {"osc": "/d", "rpn_msb": 0, "rpn_lsb": 2, "value_bits": 7, "range": [0, 127], "default": 87, "channel": 1},
            {"osc": "/e", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383], "default": 9000},
        ]},
    }));
    let out = export(&d, Format::Midi2Controllers).unwrap();
    let t = import(&out.value, Format::Midi2Controllers, &source()).unwrap();
    let kept = controls(&json_of(&t.value));
    let got: Vec<(Value, Value, Value)> = kept.iter().map(|k| (k["channel"].clone(), k["ways"][0]["range"].clone(), k["default"].clone())).collect();
    assert_eq!(
        got,
        [
            (json!(0), json!([0, 127]), json!(100)),
            (json!(15), json!([0, 16383]), json!(12000)),
            (json!(7), json!([0u64, 4294967295u64]), json!(3000000000u64)),
            (json!(1), json!([0, 127]), json!(87)),
            (json!(null), json!([0, 16383]), json!(9000)),
        ]
    );
}
