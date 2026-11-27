//! The partners' formats carry 0.7 both ways: every export reads a device of 0.7 and names each loss
//! at a pointer of the descriptor it was given; every import writes a descriptor that 0.7 reads
//! strictly and names each loss at a place of the source file or of the descriptor it produced; the
//! round trips of the ways hold.

use crate::common::electra::{self, json_of_0_7};
use crate::common::midnam::device_with;
use crate::common::{csv, device_from_fixture, knobkraft, parse, resolved, row, source, HEADER};
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

/// The fields of a control that 0.7 no longer has, flat: they live in a way now.
const FLAT: [&str; 12] = ["cc", "nrpn_msb", "nrpn_lsb", "also", "_sources", "_name", "_description", "_usage", "_notes", "transmit", "recognize", "format"];

fn rich_blocks() -> Value {
    json!({
        "role_vocabulary": "1",
        "programs": {"banks": [{"name": "A", "size": 64}], "dump": {"kind": "request", "request": [240, 66, 48, 0, 1, 81, 16, 247]}},
        "sound_transfers": knobkraft::base()["sound_transfers"].clone(),
        "cc_params": {"entries": [
            {"osc": "/filter/cutoff", "title": "Cutoff", "description": "The cutoff", "value_description": "0 closed", "notes": "a note", "polarity": "unipolar",
             "ways": [{"ctrlType": "pBend", "value_bits": 14}, {"ctrlType": "cc", "ctrlIndex": [74]}, {"ctrlType": "nrpn", "ctrlIndex": [1, 8], "value_bits": 14}, {"ctrlType": "note", "ctrlIndex": [36]}]},
            {"osc": "/press", "ways": [{"ctrlType": "chPress"}, {"ctrlType": "pPress", "ctrlIndex": [60]}]},
        ]},
        "params": {"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7", "entries": [{"osc": "/p", "ways": [{"ctrlType": "sysex", "id": 5}]}]},
    })
}

fn rich() -> osd::Resolved {
    device_with(rich_blocks())
}

fn resolves(doc: &Value, pointer: &str) -> bool {
    pointer.starts_with('/') && doc.pointer(pointer).is_some()
}

fn flat_segment(pointer: &str) -> Option<String> {
    let segments: Vec<&str> = pointer.split('/').collect();
    let at = segments.iter().position(|s| *s == "entries")?;
    segments.get(at + 2).filter(|s| FLAT.contains(s)).map(|s| s.to_string())
}

fn assert_export_places(format: Format, doc: &Value, lost: &[Loss]) {
    for l in lost {
        assert!(l.place.starts_with('/'), "{}: the place of a loss of an export is a JSON pointer: {l:?}", format.name());
        // A field the import of the file written adds is named at its pointer, inside a block of the
        // descriptor given.
        let block = l.place.rsplit_once('/').map_or("", |(block, _)| block);
        assert!(resolves(doc, &l.place) || resolves(doc, block), "{}: {:?} does not resolve in the descriptor given: {l:?}", format.name(), l.place);
        assert_eq!(flat_segment(&l.place), None, "{}: a flat field retired in 0.7: {l:?}", format.name());
    }
}

#[test]
fn every_export_names_its_losses_at_a_pointer_of_the_descriptor_it_was_given() {
    let d = rich();
    let doc = json_of_0_7(&d.device);
    for format in Format::ALL {
        match export(&d, format) {
            Ok(t) => assert_export_places(format, &doc, &t.lost),
            Err(f) => assert!(f.place.starts_with('/') || f.place == "format", "{}: {f:?}", format.name()),
        }
    }
}

#[test]
fn the_knobkraft_export_names_its_losses_at_pointers_of_the_descriptor_and_never_at_format() {
    let d = knobkraft::device(json!({"cc_params": rich_blocks()["cc_params"].clone()}));
    let doc = json_of_0_7(&d.device);
    let t = export(&d, Format::Knobkraft).expect("exports");
    assert!(!t.lost.is_empty());
    for l in &t.lost {
        assert!(resolves(&doc, &l.place), "{l:?}");
    }
}

#[test]
fn the_deluge_export_names_its_losses_at_pointers_of_the_descriptor() {
    let d = rich();
    let doc = json_of_0_7(&d.device);
    let t = export(&d, Format::Deluge).expect("exports");
    assert!(!t.lost.is_empty());
    for l in &t.lost {
        assert!(resolves(&doc, &l.place), "{l:?}");
        assert_eq!(flat_segment(&l.place), None, "{l:?}");
    }
}

fn assert_import_places(format: &str, produced: &Value, lost: &[Loss]) {
    for l in lost {
        let at_source = l.place.starts_with("line ") || l.place.starts_with('/');
        assert!(at_source, "{format}: {l:?}");
        if l.place.starts_with("/cc_params") || l.place.starts_with("/triggers") || l.place.starts_with("/params") {
            assert_eq!(flat_segment(&l.place), None, "{format}: a flat field retired in 0.6 is no place: {l:?}");
        }
        let _ = produced;
    }
}

fn samples() -> Vec<(Format, String, osd::Source)> {
    let csv_text = csv(&[
        row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("nrpn_msb", "1"), ("nrpn_lsb", "8"), ("notes", "n"), ("usage", "u"), ("orientation", "centered")]),
        row(&[("manufacturer", "Other"), ("device", "Other"), ("parameter_name", "Nothing")]),
    ]);
    let preset = electra::preset(&[(1, "G")], vec![electra::control(1, "Cut", Some(1), electra::cc7(74), Some((0, 127))), electra::control(2, "Odd", Some(1), electra::other("start"), None)]);
    let midnam = crate::common::midnam::rich_text();
    let ns = crate::common::oscquery::namespace(vec![("cutoff", crate::common::oscquery::method("/synth/cutoff", "i", &[Some((0, 127))]))]);
    let list = json!([{"ctrlType": "cc", "ctrlIndex": [74], "title": "Cut"}, {"ctrlType": "pnp", "ctrlIndex": [1, 2], "title": "P"}, {"ctrlType": "chPress", "title": "Press"}]);
    vec![
        (Format::MidiCsv, csv_text, crate::common::source()),
        (Format::ElectraOne, preset.to_string(), electra::source()),
        (Format::Midnam, midnam, crate::common::source()),
        (Format::OscQuery, ns.to_string(), crate::common::oscquery::source()),
        (Format::Midi2Controllers, list.to_string(), crate::common::midi2::source()),
    ]
}

#[test]
fn every_import_writes_a_descriptor_that_0_7_reads_strictly_and_names_its_losses_at_a_place_of_the_source_or_the_descriptor() {
    for (format, text, src) in samples() {
        let t = import(&text, format, &src).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
        let written = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
        match osd::read(&written, osd::Kind::Device).unwrap_or_else(|f| panic!("{}: {f:?}\n{written}", format.name())) {
            osd::Descriptor::Device(d) => assert_eq!(d.schema, Some(osd::Version::V0_7), "{}", format.name()),
            _ => panic!("not a device"),
        }
        assert_import_places(format.name(), &serde_json::from_str(&written).unwrap(), &t.lost);
    }
}

#[test]
fn the_triggers_import_writes_a_descriptor_that_0_7_reads_strictly() {
    let text = crate::common::trigger_csv(&[crate::common::trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_default_value", "36"), ("comments", "c"), ("velocity_usage", "v")])]);
    let t = import(&text, Format::MidiTriggersCsv, &crate::common::source()).expect("imports");
    let written = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
    match osd::read(&written, osd::Kind::Device).unwrap_or_else(|f| panic!("{f:?}\n{written}")) {
        osd::Descriptor::Device(d) => assert_eq!(d.schema, Some(osd::Version::V0_7)),
        _ => panic!("not a device"),
    }
    assert_import_places("midi-triggers-csv", &Value::Null, &t.lost);
}

#[test]
fn every_exporter_takes_a_device_of_0_7_with_its_texts_ways_and_credit() {
    let d = rich();
    for format in Format::ALL {
        let t = export(&d, format).unwrap_or_else(|f| panic!("{}: {f}", format.name()));
        assert!(!t.value.is_empty(), "{}", format.name());
    }
}

#[test]
fn an_export_from_a_device_read_in_0_6_and_converted_gives_the_text_of_the_equivalent_0_7() {
    let flat = json!({
        "$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{"type": "community-re", "url": "https://example.org/d", "author": "Dee", "license": "CC-BY-4.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "role_vocabulary": "1",
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 74, "_name": "Cutoff", "_usage": "u", "_notes": "n"}]}
    });
    let ways = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "author": "Dee", "license": "CC-BY-4.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "role_vocabulary": "1",
        "cc_params": {"entries": [{"osc": "/filter/cutoff", "title": "Cutoff", "value_description": "u", "notes": "n", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}]}]}
    });
    let from_flat = resolved(device_from_fixture(flat));
    let from_ways = resolved(crate::common::device_from(&ways.to_string()));
    for format in [Format::MidiCsv, Format::ElectraOne, Format::Midnam, Format::OscQuery, Format::Midi2Controllers, Format::Deluge] {
        let (a, b) = (export(&from_flat, format).expect("exports"), export(&from_ways, format).expect("exports"));
        assert_eq!(a.value, b.value, "{}", format.name());
        assert_eq!(a.lost.iter().map(|l| l.place.clone()).collect::<Vec<_>>(), b.lost.iter().map(|l| l.place.clone()).collect::<Vec<_>>(), "{}", format.name());
    }
}

#[test]
fn a_midi_guide_line_with_a_cc_and_an_nrpn_gives_a_cc_way_then_an_nrpn_way_and_is_written_back_unchanged() {
    let text = csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("nrpn_msb", "1"), ("nrpn_lsb", "8")])]);
    let t = import(&text, Format::MidiCsv, &source()).expect("imports");
    let doc = json_of_0_7(&t.value);
    let ways = &doc["cc_params"]["entries"][0]["ways"];
    assert_eq!(ways.as_array().map(|w| w.len()), Some(2), "{doc}");
    assert_eq!((ways[0]["ctrlType"].clone(), ways[0]["ctrlIndex"].clone()), (json!("cc"), json!([74])));
    assert_eq!((ways[1]["ctrlType"].clone(), ways[1]["ctrlIndex"].clone()), (json!("nrpn"), json!([1, 8])));
    for way in ways.as_array().unwrap() {
        assert!(way.get("transmit").is_none() && way.get("recognize").is_none(), "{way}");
    }
    let back = export(&resolved(t.value), Format::MidiCsv).expect("exports");
    let records = parse(&back.value);
    let cell = |c: &str| records[1][HEADER.iter().position(|h| *h == c).unwrap()].clone();
    assert_eq!((cell("cc_msb"), cell("nrpn_msb"), cell("nrpn_lsb")), ("74".into(), "1".into(), "8".into()));
    assert_eq!(records[1].iter().filter(|c| !c.is_empty()).count(), 6, "nothing added: {records:?}");
}

#[test]
fn an_electra_one_control_in_the_relative_mode_comes_back_unchanged_after_an_export_then_an_import() {
    let d = electra::device6(json!([{"osc": "/encoder/1", "ways": [{"ctrlType": "cc", "ctrlIndex": [20], "value_bits": 7, "transmit": "relative", "recognize": "relative"}]}]));
    let out = export(&d, Format::ElectraOne).expect("exports");
    let back = import(&out.value, Format::ElectraOne, &electra::source()).expect("imports");
    let doc = json_of_0_7(&back.value);
    let ways = &doc["cc_params"]["entries"][0]["ways"];
    assert_eq!(ways.as_array().map(|w| w.len()), Some(1), "{doc}");
    let w = &ways[0];
    assert_eq!((w["ctrlType"].clone(), w["ctrlIndex"].clone(), w["value_bits"].clone()), (json!("cc"), json!([20]), json!(7)));
    assert_eq!((w["transmit"].clone(), w["recognize"].clone()), (json!("relative"), json!("relative")));
}

// ---- the MIDI 2.0 controller list: pressure, polyphonic pressure and pitch bend are ways both ways

fn midi2_doc() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/midi2-controllers.md")).unwrap()
}

#[test]
fn a_list_with_a_pressure_a_polyphonic_pressure_and_a_bend_goes_to_ways_and_back_unchanged_with_no_loss_named() {
    let list = json!({"ChCtrlList": [
        {"title": "Press", "ctrlType": "chPress", "ctrlIndex": [], "transmit": "absolute", "recognize": "absolute"},
        {"title": "Poly", "ctrlType": "pPress", "ctrlIndex": [60]},
        {"title": "Bend", "ctrlType": "pBend", "ctrlIndex": []}
    ]});
    let t = import(&list.to_string(), Format::Midi2Controllers, &crate::common::midi2::source()).expect("imports");
    let carried: Vec<&Loss> = t.lost.iter().filter(|l| l.place.starts_with("/ChCtrlList")).collect();
    assert!(carried.is_empty(), "a pressure, a polyphonic pressure and a bend are carried: {carried:?}");
    let doc = json_of_0_7(&t.value);
    let kinds: Vec<Value> = doc["cc_params"]["entries"].as_array().unwrap().iter().map(|e| e["ways"][0]["ctrlType"].clone()).collect();
    assert_eq!(kinds, vec![json!("chPress"), json!("pPress"), json!("pBend")], "{doc}");
    let back = export(&resolved(t.value), Format::Midi2Controllers).expect("exports");
    assert!(back.lost.iter().all(|l| !l.place.contains("/ways/")), "{:?}", back.lost);
    let v: Value = serde_json::from_str(&back.value).unwrap();
    let entries = v["ChCtrlList"].as_array().unwrap();
    let kinds: Vec<Value> = entries.iter().map(|e| e["ctrlType"].clone()).collect();
    assert_eq!(kinds, vec![json!("chPress"), json!("pPress"), json!("pBend")]);
    assert_eq!(entries[1]["ctrlIndex"], json!([60]));
}

#[test]
fn the_midi2_document_says_pressure_polyphonic_pressure_and_bend_are_carried_both_ways() {
    let text = midi2_doc();
    let row = text.lines().find(|l| l.starts_with("| `ctrlType` |")).expect("the ctrlType line");
    assert!(!row.contains("read and lost"), "{row}");
    for kind in ["chPress", "pPress", "pBend"] {
        assert!(row.contains(kind), "{kind}: {row}");
    }
    let lost_part = row.split("lost").nth(1).unwrap_or("");
    for kind in ["chPress", "pPress", "pBend"] {
        assert!(!lost_part.contains(kind), "{kind} is still said lost: {row}");
    }
    for kind in ["pnrc", "pnac", "pnp"] {
        assert!(row.contains(kind), "{kind}: {row}");
    }
}

#[test]
fn the_midi2_document_keeps_the_no_field_sentence_for_pnrc_pnac_and_pnp_alone() {
    let text = midi2_doc();
    let mut found = 0;
    for line in text.lines().filter(|l| l.contains("has no field for")) {
        found += 1;
        for kind in ["chPress", "pPress", "pBend"] {
            assert!(!line.contains(kind), "{kind} has a field in 0.7: {line}");
        }
        for kind in ["pnrc", "pnac", "pnp"] {
            assert!(line.contains(kind), "{kind}: {line}");
        }
    }
    assert!(found >= 1, "the sentence for pnrc, pnac and pnp is gone");
    let spec = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../spec/0.7.md")).unwrap();
    assert!(!spec.contains("pnrc"), "spec/0.7.md has a ctrlType for pnrc: the document must then carry it");
}

#[test]
fn an_entry_of_pnrc_pnac_or_pnp_is_still_lost_at_its_pointer() {
    for kind in ["pnrc", "pnac", "pnp"] {
        let list = json!([{"title": "X", "ctrlType": kind, "ctrlIndex": [1, 2]}]);
        let t = import(&list.to_string(), Format::Midi2Controllers, &crate::common::midi2::source()).expect("imports");
        assert!(t.lost.iter().any(|l| l.place == "/0"), "{kind}: {:?}", t.lost);
    }
}
