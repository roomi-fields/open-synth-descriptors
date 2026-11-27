//! An OSD device exports to an Electra One preset valid by the maker's documented rules
//! (see common/electra.rs), laid out by section, with what it cannot carry in the losses.

use crate::common::electra::*;
use crate::common::{device_from, resolved};
use osd_formats::{export, FaultKind, Format};
use serde_json::{json, Value};

fn exported(device: &osd::Resolved) -> (Value, Vec<osd_formats::Loss>) {
    let t = export(device, Format::ElectraOne).expect("the device exports");
    (serde_json::from_str(&t.value).expect("the export is JSON"), t.lost)
}

fn cc_entry(osc: &str, section: Option<&str>, cc: u64) -> Value {
    let mut e = json!({"osc": osc, "cc": cc});
    if let Some(s) = section {
        e["section"] = json!(s);
    }
    e
}

#[test]
fn the_export_is_a_version_3_preset_valid_by_the_rules_of_the_maker() {
    let d = device(json!([cc_entry("/a", Some("S"), 1), cc_entry("/b", None, 2)]), json!([{"osc": "/dump", "frame": [240, 247]}]));
    let (p, _) = exported(&d);
    assert_eq!(p["version"], 3);
    assert_eq!(p["name"], "Trundler");
    assert_eq!(violations(&p), Vec::<String>::new());
}

#[test]
fn the_example_device_exports_a_valid_preset() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/0.5/synth.device.json");
    let text = std::fs::read_to_string(path).unwrap();
    let (p, _) = exported(&resolved(device_from(&text)));
    assert_eq!(violations(&p), Vec::<String>::new());
}

#[test]
fn each_kind_of_control_is_written_with_its_message_and_parameter_number() {
    let entries = json!([
        {"osc": "/a", "cc": 74},
        {"osc": "/b", "cc": 5, "cc_lsb": 37},
        {"osc": "/c", "nrpn_msb": 2, "nrpn_lsb": 44},
        {"osc": "/d", "rpn_msb": 1, "rpn_lsb": 2},
    ]);
    let (p, _) = exported(&device(entries, json!([])));
    let messages: Vec<(Value, Value)> =
        p["controls"].as_array().unwrap().iter().map(|c| (c["values"][0]["message"]["type"].clone(), c["values"][0]["message"]["parameterNumber"].clone())).collect();
    assert_eq!(messages, vec![(json!("cc7"), json!(74)), (json!("cc14"), json!(5)), (json!("nrpn"), json!(300)), (json!("rpn"), json!(130))]);
}

#[test]
fn the_range_of_a_control_is_the_range_of_its_value_and_none_gives_none() {
    let entries = json!([{"osc": "/a", "cc": 1, "range": [10, 100]}, {"osc": "/b", "cc": 2}]);
    let (p, _) = exported(&device(entries, json!([])));
    let v = |n: usize| p["controls"][n]["values"][0].clone();
    assert_eq!((v(0)["min"].clone(), v(0)["max"].clone()), (json!(10), json!(100)));
    assert!(v(1).get("min").is_none() && v(1).get("max").is_none(), "{}", v(1));
}

#[test]
fn a_sysex_command_is_written_as_a_sysex_control_with_its_bytes() {
    let d = device(json!([]), json!([{"osc": "/dump", "frame": [240, 126, 6, 1, 247]}]));
    let (p, _) = exported(&d);
    let m = &p["controls"][0]["values"][0]["message"];
    assert_eq!(m["type"], "sysex");
    let bytes: Vec<u64> = m["data"].as_array().unwrap().iter().map(|b| b.as_u64().unwrap()).collect();
    assert_eq!(bytes, vec![240, 126, 6, 1, 247]);
}

#[test]
fn a_command_whose_frame_has_an_argument_or_none_is_a_loss_at_its_pointer_and_not_written() {
    let commands = json!([
        {"osc": "/set", "frame": [240, {"arg": "v"}, 247]},
        {"osc": "/osc-only"},
        {"osc": "/ok", "frame": [240, 247]},
    ]);
    let (p, lost) = exported(&device(json!([]), commands));
    assert_eq!(p["controls"].as_array().unwrap().len(), 1);
    let places: Vec<&str> = lost.iter().map(|l| l.place.as_str()).collect();
    assert!(places.contains(&"/commands/0/frame"), "{places:?}");
    assert!(places.contains(&"/commands/1"), "{places:?}");
    assert!(lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn the_name_of_a_control_is_its_title_else_its_readable_address() {
    let entries = json!([{"osc": "/filter/cutoff", "cc": 1, "title": "Brightness"}, {"osc": "/filter/res", "cc": 2}]);
    let (p, _) = exported(&device6(entries));
    assert_eq!(p["controls"][0]["name"], "Brightness");
    assert_eq!(p["controls"][1]["name"], "Res");
}

#[test]
fn the_controls_are_laid_out_by_section_in_the_order_of_the_descriptor() {
    let entries = json!([
        cc_entry("/a", Some("Osc"), 1),
        cc_entry("/b", Some("Osc"), 2),
        cc_entry("/c", Some("Filter"), 3),
        cc_entry("/d", None, 4),
    ]);
    let (p, _) = exported(&device(entries, json!([])));
    let numbers: Vec<u64> = p["controls"].as_array().unwrap().iter().map(|c| c["values"][0]["message"]["parameterNumber"].as_u64().unwrap()).collect();
    assert_eq!(numbers, vec![1, 2, 3, 4]);
    let group_name = |n: usize| {
        let id = p["controls"][n].get("groupId")?.clone();
        p["groups"].as_array()?.iter().find(|g| g["id"] == id).map(|g| g["name"].clone())
    };
    assert_eq!(group_name(0), Some(json!("Osc")));
    assert_eq!(group_name(1), Some(json!("Osc")));
    assert_eq!(group_name(2), Some(json!("Filter")));
    assert_eq!(group_name(3), None);
    assert_eq!(p["controls"][0]["groupId"], p["controls"][1]["groupId"]);
    assert_ne!(p["controls"][0]["groupId"], p["controls"][2]["groupId"]);
}

#[test]
fn the_pages_and_slots_fill_in_order_12_pots_3_control_sets_and_12_pages() {
    let entries: Vec<Value> = (0..40).map(|n| cc_entry(&format!("/c{n}"), None, n % 128)).collect();
    let (p, _) = exported(&device(json!(entries), json!([])));
    let slot = |n: usize| {
        let c = &p["controls"][n];
        (c["pageId"].as_u64().unwrap(), c["controlSetId"].as_u64().unwrap(), c["inputs"][0]["potId"].as_u64().unwrap())
    };
    assert_eq!(slot(0), (1, 1, 1));
    assert_eq!(slot(11), (1, 1, 12));
    assert_eq!(slot(12), (1, 2, 1));
    assert_eq!(slot(35), (1, 3, 12));
    assert_eq!(slot(36), (2, 1, 1));
    assert_eq!(slot(39), (2, 1, 4));
    assert_eq!(violations(&p), Vec::<String>::new());
    assert!(p["pages"].as_array().unwrap().iter().any(|g| g["id"] == 2));
}

#[test]
fn a_device_with_more_controls_than_a_preset_holds_exports_the_first_432_and_names_the_rest() {
    // One message each: a control that repeats the message of an earlier one does not come back.
    let entries: Vec<Value> = (0..440).map(|n| json!({"osc": format!("/c{n}"), "nrpn_msb": n / 128, "nrpn_lsb": n % 128})).collect();
    let (p, lost) = exported(&device(json!(entries), json!([])));
    assert_eq!(p["controls"].as_array().unwrap().len(), 432);
    assert_eq!(violations(&p), Vec::<String>::new());
    for n in 432..440 {
        let place = format!("/cc_params/entries/{n}");
        assert!(lost.iter().any(|l| l.place == place && !l.reason.is_empty()), "{place} not in the losses");
    }
    assert!(lost.iter().all(|l| l.place != "/cc_params/entries/431"));
}

#[test]
fn a_control_that_repeats_the_message_of_an_earlier_one_is_named_since_the_import_does_not_give_it_back() {
    let entries = json!([cc_entry("/a", None, 20), cc_entry("/b", None, 20), cc_entry("/c", None, 21)]);
    let (_, lost) = exported(&device(entries, json!([])));
    assert!(lost.iter().any(|l| l.place == "/cc_params/entries/1" && !l.reason.is_empty()), "{lost:?}");
    assert!(lost.iter().all(|l| l.place != "/cc_params/entries/0" && l.place != "/cc_params/entries/2"), "{lost:?}");
}

#[test]
fn the_commands_after_the_controls_that_overflow_are_named_in_the_losses() {
    let entries: Vec<Value> = (0..431).map(|n| cc_entry(&format!("/c{n}"), None, n % 128)).collect();
    let commands = json!([{"osc": "/one", "frame": [240, 247]}, {"osc": "/two", "frame": [240, 1, 247]}]);
    let (p, lost) = exported(&device(json!(entries), commands));
    assert_eq!(p["controls"].as_array().unwrap().len(), 432);
    assert!(lost.iter().any(|l| l.place == "/commands/1"), "{lost:?}");
    assert!(lost.iter().all(|l| l.place != "/commands/0"));
}

#[test]
fn what_the_preset_cannot_carry_is_in_the_losses_by_its_json_pointer() {
    let way = |n: u64| json!([{"ctrlType": "cc", "ctrlIndex": [n], "value_bits": 7}]);
    let entries = json!([{"osc": "/a", "title": "A", "ways": way(1), "polarity": "bipolar"}, {"osc": "/b", "title": "B", "ways": way(2)}]);
    let (_, lost) = exported(&device6(entries));
    let loss = lost.iter().find(|l| l.place == "/cc_params/entries/0/polarity").unwrap_or_else(|| panic!("{lost:?}"));
    assert!(!loss.reason.is_empty());
    assert!(lost.iter().all(|l| !l.place.starts_with("/cc_params/entries/1")), "{lost:?}");
}

#[test]
fn the_same_device_exports_the_same_text_twice_with_unique_ids() {
    let entries: Vec<Value> = (0..50).map(|n| cc_entry(&format!("/c{n}"), Some(["A", "B", "C"][n % 3]), n as u64)).collect();
    let d = device(json!(entries), json!([{"osc": "/x", "frame": [240, 247]}]));
    let a = export(&d, Format::ElectraOne).unwrap();
    let b = export(&d, Format::ElectraOne).unwrap();
    assert_eq!(a.value, b.value);
    assert_eq!(a.lost, b.lost);
    let p: Value = serde_json::from_str(&a.value).unwrap();
    assert_eq!(violations(&p), Vec::<String>::new());
}

#[test]
fn a_device_without_sources_is_refused() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let f = export(&resolved(device_from(text)), Format::ElectraOne).unwrap_err();
    assert_eq!(f.kind, FaultKind::NoSources);
}
