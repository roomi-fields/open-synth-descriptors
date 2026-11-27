//! The formats read the ways of a control (0.7): a format that writes one message per control
//! writes the first way, in the order written, that it can carry, and names every other way in the
//! losses at its pointer; a way of a kind the format does not say (`note`, `sysex`, a pressure or a
//! bend for MIDNAM) is a loss, never a refusal. A format that says a kind of way writes and reads it.

use crate::common::electra::device6;
use crate::common::midi2::{cc_device, exported as exported_list, imported, list};
use crate::common::midnam::{device_with, exported as exported_midnam, lost_at};
use crate::common::{parse, HEADER};
use osd_formats::{export, Format};
use serde_json::{json, Value};

fn way(kind: &str, index: Value) -> Value {
    json!({"ctrlType": kind, "ctrlIndex": index})
}

fn nrpn(msb: i64, lsb: i64) -> Value {
    json!({"ctrlType": "nrpn", "ctrlIndex": [msb, lsb], "value_bits": 14})
}

fn lost_places(lost: &[osd_formats::Loss], entry: usize) -> Vec<String> {
    let mut places: Vec<String> = lost.iter().map(|l| l.place.clone()).filter(|p| p.starts_with(&format!("/cc_params/entries/{entry}/ways/"))).collect();
    places.sort();
    places.dedup();
    places
}

fn entry_ways(ways: Vec<Value>) -> Value {
    json!([{"osc": "/filter/cutoff", "ways": ways}])
}

const PLACES_0_AND_2: [&str; 2] = ["/cc_params/entries/0/ways/0", "/cc_params/entries/0/ways/2"];

// ---- MIDNAM: one message per control

#[test]
fn midnam_writes_the_first_way_it_can_carry_and_names_each_other_way_in_the_losses() {
    let d = device_with(json!({"cc_params": {"entries": entry_ways(vec![json!({"ctrlType": "pBend"}), way("cc", json!([74])), nrpn(1, 8)])}}));
    let (text, lost) = exported_midnam(&d);
    assert!(text.contains("Number=\"74\""), "{text}");
    assert!(!text.contains("Type=\"NRPN\""),"the NRPN is not the first way carried: {text}");
    assert_eq!(lost_places(&lost, 0), PLACES_0_AND_2, "{lost:?}");
    assert!(lost.iter().filter(|l| l.place.starts_with("/cc_params/entries/0/ways/")).all(|l| !l.reason.is_empty()));
}

#[test]
fn midnam_names_a_note_or_a_sysex_or_a_pressure_way_in_the_losses_and_never_refuses() {
    for first in [way("note", json!([36])), json!({"ctrlType": "chPress"}), way("pPress", json!([60]))] {
        let d = device_with(json!({"cc_params": {"entries": entry_ways(vec![first.clone(), way("cc", json!([74]))])}}));
        let (text, lost) = exported_midnam(&d);
        assert!(text.contains("Number=\"74\""), "{first}: {text}");
        assert_eq!(lost_at(&lost, "/cc_params/entries/0/ways/0").len(), 1, "{first}: {lost:?}");
        assert!(lost_at(&lost, "/cc_params/entries/0/ways/1").is_empty(), "{first}: the written way is not lost: {lost:?}");
    }
    let only = device_with(json!({"cc_params": {"entries": entry_ways(vec![way("note", json!([36]))])}}));
    let (_, lost) = exported_midnam(&only);
    assert_eq!(lost_at(&lost, "/cc_params/entries/0/ways/0").len(), 1, "{lost:?}");
}

#[test]
fn midnam_names_the_sysex_way_of_a_params_entry_in_the_losses() {
    let d = device_with(json!({
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/p", "ways": [{"ctrlType": "sysex", "id": 5}]}]}
    }));
    let (_, lost) = exported_midnam(&d);
    assert!(lost.iter().any(|l| l.place.starts_with("/params/entries/0")), "{lost:?}");
}

#[test]
fn midnam_keeps_the_losses_it_names_today_for_the_flat_ways_of_0_6() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 74, "range": [0, 127]}]}}));
    let (text, lost) = exported_midnam(&d);
    assert!(text.contains("Number=\"74\""), "{text}");
    assert!(lost_places(&lost, 0).is_empty(), "one way, nothing lost on it: {lost:?}");
}

// ---- MIDI 2.0 Property Exchange: the six kinds are its own words

#[test]
fn the_controller_list_writes_the_first_way_it_carries_and_names_the_others() {
    let d = cc_device(entry_ways(vec![way("note", json!([36])), way("cc", json!([74])), nrpn(1, 8)]));
    let (v, lost) = exported_list(&d);
    let entries = list(&v);
    assert_eq!(entries.len(), 1);
    assert_eq!((entries[0]["ctrlType"].clone(), entries[0]["ctrlIndex"].clone()), (json!("cc"), json!([74])));
    assert_eq!(lost_places(&lost, 0), PLACES_0_AND_2, "{lost:?}");
}

#[test]
fn the_controller_list_writes_a_pressure_a_polyphonic_pressure_and_a_pitch_bend_as_ctrltype() {
    let d = cc_device(json!([
        {"osc": "/press", "ways": [{"ctrlType": "chPress", "transmit": "absolute", "recognize": "absolute"}]},
        {"osc": "/poly", "ways": [way("pPress", json!([60]))]},
        {"osc": "/bend", "ways": [{"ctrlType": "pBend", "value_bits": 14}]},
    ]));
    let (v, lost) = exported_list(&d);
    let entries = list(&v);
    let kinds: Vec<Value> = entries.iter().map(|e| e["ctrlType"].clone()).collect();
    assert_eq!(kinds, vec![json!("chPress"), json!("pPress"), json!("pBend")]);
    assert_eq!(entries[1]["ctrlIndex"], json!([60]));
    assert_eq!((entries[0]["transmit"].clone(), entries[0]["recognize"].clone()), (json!("absolute"), json!("absolute")));
    assert!(lost.iter().all(|l| !l.place.contains("/ways/")), "a kind the format says is no loss: {lost:?}");
}

#[test]
fn the_controller_list_gives_each_way_its_own_direction_and_writes_the_first_ones() {
    let d = cc_device(entry_ways(vec![
        json!({"ctrlType": "nrpn", "ctrlIndex": [0, 5], "value_bits": 14, "transmit": "none", "recognize": "absolute"}),
        json!({"ctrlType": "cc", "ctrlIndex": [7], "transmit": "absolute", "recognize": "none"}),
    ]));
    let (v, lost) = exported_list(&d);
    let e = &list(&v)[0];
    assert_eq!((e["ctrlType"].clone(), e["transmit"].clone(), e["recognize"].clone()), (json!("nrpn"), json!("none"), json!("absolute")));
    assert_eq!(lost_places(&lost, 0), vec!["/cc_params/entries/0/ways/1".to_string()], "{lost:?}");
}

#[test]
fn a_pressure_and_a_bend_of_a_list_come_back_as_the_same_ways_after_an_export_then_an_import() {
    let d = cc_device(json!([
        {"osc": "/press", "ways": [{"ctrlType": "chPress"}]},
        {"osc": "/poly", "ways": [way("pPress", json!([60]))]},
        {"osc": "/bend", "ways": [{"ctrlType": "pBend", "value_bits": 14}]},
    ]));
    let (v, _) = exported_list(&d);
    let (back, _) = imported(&v.to_string());
    let kinds: Vec<(Value, Value)> = crate::common::midi2::controls(&back).iter().map(|e| (e["ways"][0]["ctrlType"].clone(), e["ways"][0]["ctrlIndex"].clone())).collect();
    assert_eq!(kinds, vec![(json!("chPress"), Value::Null), (json!("pPress"), json!([60])), (json!("pBend"), Value::Null)]);
}

// ---- MIDI CSV and Electra One

#[test]
fn the_csv_writes_the_first_cc_way_and_names_the_ways_it_has_no_column_for() {
    let d = device_with(json!({"cc_params": {"entries": entry_ways(vec![json!({"ctrlType": "pBend"}), way("cc", json!([74])), way("cc", json!([75]))])}}));
    let t = export(&d, Format::MidiCsv).expect("a way the format does not say is a loss, not a refusal");
    let records = parse(&t.value);
    assert_eq!(records.len(), 2);
    let cell = |column: &str| records[1][HEADER.iter().position(|h| *h == column).unwrap()].clone();
    assert_eq!(cell("cc_msb"), "74");
    assert_eq!(lost_places(&t.lost, 0), PLACES_0_AND_2, "{:?}", t.lost);
}

#[test]
fn the_electra_preset_writes_the_first_way_it_carries_and_names_the_others() {
    let cc74 = json!({"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7});
    let d = device6(json!([{"osc": "/filter/cutoff", "ways": [json!({"ctrlType": "chPress"}), cc74, way("cc", json!([75]))]}]));
    let t = export(&d, Format::ElectraOne).expect("a way the format does not say is a loss, not a refusal");
    let p: Value = serde_json::from_str(&t.value).unwrap();
    let message = &p["controls"][0]["values"][0]["message"];
    assert_eq!((message["type"].clone(), message["parameterNumber"].clone()), (json!("cc7"), json!(74)));
    assert_eq!(lost_places(&t.lost, 0), PLACES_0_AND_2, "{:?}", t.lost);
}
