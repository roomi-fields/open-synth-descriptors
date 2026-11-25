//! The converters of the partners write and read the fields `title`, `description`,
//! `value_description` and `notes` of 0.7, and no longer the keys `_name`, `_description`,
//! `_usage` and `_notes`.

use crate::common::electra::{cc7, control, device6, preset};
use crate::common::midnam::{control_document, device_with};
use crate::common::{csv, device_from, entries, parse, resolved, row, source, trigger_csv, trigger_row, triggers, HEADER, TRIGGER_HEADER};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

const SOURCES: &str = r#""sources": [{"type": "community-re", "url": "https://example.org/x", "license": "CC0-1.0"}]"#;

fn device7(body: &str) -> osd::Resolved {
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.7/device.schema.json", {SOURCES}, "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}{body}}}"#
    );
    resolved(device_from(&text))
}

fn cell(record: &[String], column: &str) -> String {
    record[HEADER.iter().position(|h| *h == column).unwrap()].clone()
}

fn trigger_cell(record: &[String], column: &str) -> String {
    record[TRIGGER_HEADER.iter().position(|h| *h == column).unwrap()].clone()
}

// midi.guide, the CSV of the controls

#[test]
fn the_csv_of_the_controls_reads_title_description_value_description_and_notes() {
    let d = device7(
        r#", "cc_params": {"entries": [{"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}], "title": "Filter Cutoff", "description": "Opens the filter.", "value_description": "0 closes, 127 opens.", "notes": "Range checked."}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let r = &parse(&t.value)[1];
    assert_eq!(cell(r, "parameter_name"), "Filter Cutoff");
    assert_eq!(cell(r, "parameter_description"), "Opens the filter.");
    assert_eq!(cell(r, "usage"), "0 closes, 127 opens.");
    assert_eq!(cell(r, "notes"), "Range checked.");
}

#[test]
fn the_csv_of_the_controls_no_longer_reads_the_underscore_keys() {
    let d = device7(
        r#", "cc_params": {"entries": [{"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}], "_name": "Old", "_description": "Old d", "_usage": "Old u", "_notes": "Old n"}]}"#,
    );
    let r = &parse(&export(&d, Format::MidiCsv).unwrap().value)[1];
    assert_eq!(cell(r, "parameter_name"), "/filter/cutoff");
    assert_eq!((cell(r, "parameter_description"), cell(r, "usage"), cell(r, "notes")), (String::new(), String::new(), String::new()));
}

#[test]
fn a_control_without_title_exports_its_address_as_the_name_cell() {
    let d = device7(r#", "cc_params": {"entries": [{"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]}]}"#);
    let r = &parse(&export(&d, Format::MidiCsv).unwrap().value)[1];
    assert_eq!(cell(r, "parameter_name"), "/filter/cutoff");
}

#[test]
fn the_csv_of_the_controls_imports_into_the_fields_and_keeps_no_underscore_key() {
    let r = row(&[("parameter_name", "Glide"), ("parameter_description", "d"), ("cc_msb", "65"), ("notes", "n"), ("usage", "u")]);
    let t = import(&csv(&[r]), Format::MidiCsv, &source()).unwrap();
    let e = &entries(&t.value)[0];
    assert_eq!((e.title.as_deref(), e.description.as_deref(), e.notes.as_deref(), e.value_description.as_deref()), (Some("Glide"), Some("d"), Some("n"), Some("u")));
    assert!(e.remarks.is_empty(), "{:?}", e.remarks);
}

#[test]
fn a_row_without_a_name_imports_with_no_title_and_never_the_address() {
    let r = row(&[("cc_msb", "65"), ("parameter_description", "d")]);
    let t = import(&csv(&[r]), Format::MidiCsv, &source()).unwrap();
    assert_eq!(entries(&t.value)[0].title, None);
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert!(doc["cc_params"]["entries"][0].get("title").is_none(), "{doc}");
}

#[test]
fn the_csv_of_the_controls_comes_back_with_its_fields() {
    let r = row(&[("parameter_name", "Glide"), ("parameter_description", "d"), ("cc_msb", "65"), ("notes", "n"), ("usage", "u")]);
    let imported = import(&csv(std::slice::from_ref(&r)), Format::MidiCsv, &source()).unwrap();
    let out = export(&resolved(imported.value), Format::MidiCsv).unwrap();
    assert_eq!(parse(&out.value)[1], r);
}

// midi.guide, the CSV of the triggers

#[test]
fn the_csv_of_the_triggers_reads_description_and_imports_into_it() {
    let d = device7(r#", "triggers": {"entries": [{"osc": "/a", "sound": "Kick", "note_range": [36, 36], "description": "The big one", "_description": "Old"}]}"#);
    let r = &parse(&export(&d, Format::MidiTriggersCsv).unwrap().value)[1];
    assert_eq!(trigger_cell(r, "sound_description"), "The big one");
    let row = trigger_row(&[("sound_name", "Kick"), ("sound_description", "The big one"), ("note_number_min_value", "36"), ("note_number_max_value", "36")]);
    let t = import(&trigger_csv(&[row]), Format::MidiTriggersCsv, &source()).unwrap();
    assert_eq!(triggers(&t.value)[0].description.as_deref(), Some("The big one"));
}

#[test]
fn the_csv_of_the_triggers_no_longer_reads_the_underscore_description() {
    let d = device7(r#", "triggers": {"entries": [{"osc": "/a", "sound": "Kick", "note_range": [36, 36], "_description": "Old"}]}"#);
    let r = &parse(&export(&d, Format::MidiTriggersCsv).unwrap().value)[1];
    assert_eq!(trigger_cell(r, "sound_description"), "");
}

// Electra One

#[test]
fn an_electra_control_with_a_name_imports_as_a_title_and_without_a_name_gets_none() {
    let p = preset(&[(1, "Filter")], vec![control(1, "Cutoff", Some(1), cc7(74), None), control(2, "", Some(1), cc7(75), None)]);
    let t = import(&p.to_string(), Format::ElectraOne, &source()).unwrap();
    let e = entries(&t.value);
    assert_eq!(e[0].title.as_deref(), Some("Cutoff"));
    assert_eq!(e[1].title, None);
    assert!(e.iter().all(|x| !x.remarks.contains_key("_name")));
}

#[test]
fn an_electra_export_names_a_control_by_its_title_and_not_by_the_underscore_name() {
    let d = device6(json!([
        {"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}], "title": "Brightness"},
        {"osc": "/filter/res", "ways": [{"ctrlType": "cc", "ctrlIndex": [2]}], "_name": "Old"}
    ]));
    let out: Value = serde_json::from_str(&export(&d, Format::ElectraOne).unwrap().value).unwrap();
    assert_eq!(out["controls"][0]["name"], "Brightness");
    assert_eq!(out["controls"][1]["name"], "Res");
}

// MIDNAM

#[test]
fn a_midnam_control_with_a_name_imports_as_a_title_and_without_a_name_gets_none() {
    let named = import(&control_document("7bit", "74", "0", "127", "Some Control", None), Format::Midnam, &source()).unwrap();
    assert_eq!(entries(&named.value)[0].title.as_deref(), Some("Some Control"));
    let bare = import(&control_document("7bit", "74", "0", "127", "", None), Format::Midnam, &source()).unwrap();
    let e = &entries(&bare.value)[0];
    assert_eq!(e.title, None);
    assert!(!e.remarks.contains_key("_name"));
}

#[test]
fn a_midnam_export_names_a_control_by_its_title_and_not_by_the_underscore_name() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/filter/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74], "range": [0, 127]}], "title": "Brightness"},
        {"osc": "/filter/res", "ways": [{"ctrlType": "cc", "ctrlIndex": [71], "range": [0, 127]}], "_name": "Old"},
    ]}}));
    let text = export(&d, Format::Midnam).unwrap().value;
    assert!(text.contains("Name=\"Brightness\""), "{text}");
    assert!(!text.contains("Name=\"Old\""), "{text}");
}

// The losses of the exports that carry no title or description

#[test]
fn a_midnam_export_names_the_title_and_the_description_of_a_trigger_in_the_losses() {
    let d = device_with(json!({"triggers": {"entries": [
        {"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "note": 36, "title": "Kick drum", "description": "The low drum"},
        {"osc": "/snare", "sound": "Snare", "note_range": [38, 38], "note": 38},
    ]}}));
    let lost = export(&d, Format::Midnam).unwrap().lost;
    let places: Vec<&str> = lost.iter().map(|l| l.place.as_str()).collect();
    assert!(places.contains(&"/triggers/entries/0/title"), "{places:?}");
    assert!(places.contains(&"/triggers/entries/0/description"), "{places:?}");
    assert!(!places.iter().any(|p| p.starts_with("/triggers/entries/1/")), "{places:?}");
}

#[test]
fn an_oscquery_export_names_the_texts_of_a_control_in_the_losses() {
    let d = device_with(json!({"cc_params": {"entries": [
        {"osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "title": "Cutoff", "description": "d", "value_description": "v", "notes": "n"},
        {"osc": "/filter/res", "cc": 71, "range": [0, 127]},
    ]}}));
    let lost = export(&d, Format::OscQuery).unwrap().lost;
    let places: Vec<&str> = lost.iter().map(|l| l.place.as_str()).collect();
    for field in ["title", "description", "value_description", "notes"] {
        let place = format!("/cc_params/entries/0/{field}");
        assert!(places.contains(&place.as_str()), "{place} {places:?}");
    }
    assert!(!places.iter().any(|p| p.starts_with("/cc_params/entries/1/")), "{places:?}");
}

#[test]
fn an_oscquery_export_names_the_title_and_the_description_of_a_params_entry_and_a_command_in_the_losses() {
    let d = device_with(json!({
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
            "entries": [{"osc": "/p/one", "pr": 1, "p": 2, "c": 3, "r": 4, "range": [0, 63], "title": "One", "description": "d"}]},
        "commands": [{"osc": "/panic", "frame": [240, 247], "title": "Panic", "description": "d"}],
    }));
    let lost = export(&d, Format::OscQuery).unwrap().lost;
    let places: Vec<&str> = lost.iter().map(|l| l.place.as_str()).collect();
    for place in ["/params/entries/0/title", "/params/entries/0/description", "/commands/0/title", "/commands/0/description"] {
        assert!(places.contains(&place), "{place} {places:?}");
    }
}
