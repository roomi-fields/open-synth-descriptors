//! A control with no message (an entry whose `ways` is empty) in the exports: the CSV writes its
//! row without a CC or an NRPN, OSCQuery writes its node with the entry's `range`, and each names
//! in the losses what it cannot carry of it.

use crate::common::oscquery::{exported, node_at};
use crate::common::{device_from, parse, resolved, source, HEADER};
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

/// A device of 0.7 with one source, holding the blocks of `extra`.
fn device(extra: Value) -> osd::Resolved {
    let mut doc = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "sources": [{"type": "community-re", "url": "https://example.org/x", "license": "CC0-1.0"}],
    });
    for (k, v) in extra.as_object().unwrap() {
        doc[k.as_str()] = v.clone();
    }
    resolved(device_from(&doc.to_string()))
}

/// The `cc_params` of the CSV and OSCQuery tests: a control with every field an entry without a way
/// carries, a bare one, and one with a way.
fn controls() -> Value {
    json!({"entries": [
        {"osc": "/bank", "title": "Bank", "ways": [], "scale": [0, 7], "range": [0, 3], "transmit": "absolute", "recognize": "none", "nrpn_lsb_range": [0, 15]},
        {"osc": "/bare", "title": "Bare", "ways": []},
        {"osc": "/cutoff", "title": "Cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74], "range": [0, 127]}]},
    ]})
}

fn places_of(lost: &[Loss], entry: &str) -> Vec<String> {
    let under = format!("{entry}/");
    let mut places: Vec<String> = lost.iter().map(|l| l.place.clone()).filter(|p| p == entry || p.starts_with(&under)).collect();
    places.sort();
    places
}

fn sorted(entry: &str, fields: &[&str]) -> Vec<String> {
    let mut places: Vec<String> = fields.iter().map(|f| format!("{entry}/{f}")).collect();
    places.push(entry.to_string());
    places.sort();
    places
}

// ---- the CSV

#[test]
fn the_csv_writes_the_row_of_a_control_without_a_way_with_no_cc_and_no_nrpn() {
    let t = export(&device(json!({"cc_params": controls()})), Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!(records.len(), 4, "{}", t.value);
    for (record, name) in [(&records[1], "Bank"), (&records[2], "Bare")] {
        for (column, cell) in HEADER.iter().zip(record) {
            if column.starts_with("cc_") || column.starts_with("nrpn_") {
                assert_eq!(cell, "", "{name}: {column}");
            }
            if *column == "parameter_name" {
                assert_eq!(cell, name);
            }
        }
    }
}

#[test]
fn the_csv_names_in_the_losses_the_control_without_a_way_and_each_field_it_holds() {
    let lost = export(&device(json!({"cc_params": controls()})), Format::MidiCsv).unwrap().lost;
    let bank = "/cc_params/entries/0";
    assert_eq!(places_of(&lost, bank), sorted(bank, &["nrpn_lsb_range", "range", "recognize", "scale", "transmit"]), "{lost:?}");
    assert_eq!(places_of(&lost, "/cc_params/entries/1"), ["/cc_params/entries/1"], "{lost:?}");
    assert!(places_of(&lost, "/cc_params/entries/2").is_empty(), "{lost:?}");
    assert!(lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn the_csv_names_only_the_fields_a_control_without_a_way_holds() {
    let entries = json!({"entries": [{"osc": "/a", "ways": [], "range": [0, 3]}, {"osc": "/b", "ways": [], "recognize": "both"}]});
    let lost = export(&device(json!({"cc_params": entries})), Format::MidiCsv).unwrap().lost;
    for (n, field) in [(0, "range"), (1, "recognize")] {
        let at = format!("/cc_params/entries/{n}");
        assert_eq!(places_of(&lost, &at), sorted(&at, &[field]), "{lost:?}");
    }
}

#[test]
fn an_import_of_the_csv_drops_the_row_of_a_control_without_a_way() {
    let text = export(&device(json!({"cc_params": controls()})), Format::MidiCsv).unwrap().value;
    let t = import(&text, Format::MidiCsv, &source()).unwrap();
    let titles: Vec<Option<String>> = t.value.cc_params.iter().flat_map(|c| &c.entries).map(|e| e.title.clone()).collect();
    assert_eq!(titles, [Some("Cutoff".to_string())]);
}

// ---- OSCQuery

#[test]
fn oscquery_writes_the_node_of_a_control_without_a_way_with_the_range_of_the_entry() {
    let (ns, _) = exported(&device(json!({"cc_params": controls()})));
    let bank = node_at(&ns, "/trundler/bank").unwrap_or_else(|| panic!("{ns}"));
    assert_eq!(bank["TYPE"], "i");
    assert_eq!(bank["RANGE"], json!([{"MIN": 0, "MAX": 3}]));
    let bare = node_at(&ns, "/trundler/bare").unwrap_or_else(|| panic!("{ns}"));
    assert_eq!(bare["TYPE"], "i");
    assert!(bare.get("RANGE").is_none(), "{bare}");
}

#[test]
fn oscquery_names_in_the_losses_each_field_of_a_control_without_a_way_but_its_range() {
    let (_, lost) = exported(&device(json!({"cc_params": controls()})));
    let places = places_of(&lost, "/cc_params/entries/0");
    for field in ["scale", "transmit", "recognize", "nrpn_lsb_range"] {
        let place = format!("/cc_params/entries/0/{field}");
        assert_eq!(places.iter().filter(|p| **p == place).count(), 1, "{place}: {lost:?}");
    }
    assert!(!places.iter().any(|p| p.ends_with("/range")), "the range is carried: {lost:?}");
    // A bare control loses nothing of a message it does not give: its title only, as every control.
    assert_eq!(places_of(&lost, "/cc_params/entries/1"), ["/cc_params/entries/1/title"], "{lost:?}");
    assert!(lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn oscquery_writes_a_params_entry_without_a_way_with_its_range_and_names_its_other_fields_in_the_losses() {
    let params = json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [
        {"osc": "/pitch", "ways": [], "scale": [-64, 63], "range": [-24, 24], "transmit": "absolute", "recognize": "both"},
        {"osc": "/shape", "ways": []},
    ]});
    let (ns, lost) = exported(&device(json!({"params": params})));
    assert_eq!(node_at(&ns, "/trundler/pitch").unwrap_or_else(|| panic!("{ns}"))["RANGE"], json!([{"MIN": -24, "MAX": 24}]));
    assert!(node_at(&ns, "/trundler/shape").unwrap_or_else(|| panic!("{ns}")).get("RANGE").is_none(), "{ns}");
    let places = places_of(&lost, "/params/entries/0");
    assert_eq!(places, ["/params/entries/0/recognize", "/params/entries/0/scale", "/params/entries/0/transmit"], "{lost:?}");
    assert!(places_of(&lost, "/params/entries/1").is_empty(), "{lost:?}");
}
