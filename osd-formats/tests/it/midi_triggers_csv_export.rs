//! The export of a device as a pencilresearch/midi triggers CSV: its rows, its refusals and its
//! losses.

use crate::common::{device_from, parse, resolved, source, trigger_header_line, trigger_row, trigger_csv, TRIGGER_HEADER};
use osd_formats::{export, import, FaultKind, Format};

const SOURCES: &str = r#""_sources": [{"type": "community-re", "url": "https://example.org/x", "license": "CC0-1.0"}]"#;

fn device(body: &str) -> osd::Resolved {
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.5/device.schema.json", {SOURCES}, "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}{body}}}"#
    );
    resolved(device_from(&text))
}

fn cell(record: &[String], column: &str) -> String {
    record[TRIGGER_HEADER.iter().position(|h| *h == column).unwrap()].clone()
}

#[test]
fn a_device_exports_the_header_and_one_row_per_trigger() {
    let d = device(
        r#", "triggers": {"entries": [
        {"osc": "/a", "sound": "Kick", "section": "Drums", "channel": 9, "note_range": [36, 38], "note": 37, "velocity_usage": "soft", "comments": "choke"},
        {"osc": "/b", "sound": "Snare", "note_range": [40, 40]}]}"#,
    );
    let t = export(&d, Format::MidiTriggersCsv).unwrap();
    assert!(t.value.starts_with(&format!("{}\n", trigger_header_line())));
    let records = parse(&t.value);
    assert_eq!(records.len(), 3);
    assert_eq!(records[0], TRIGGER_HEADER.to_vec());
    let (a, b) = (&records[1], &records[2]);
    assert_eq!((cell(a, "manufacturer"), cell(a, "device"), cell(a, "section"), cell(a, "sound_name")), ("Lofty".into(), "Trundler".into(), "Drums".into(), "Kick".into()));
    assert_eq!((cell(a, "midi_channel"), cell(a, "note_number_min_value"), cell(a, "note_number_max_value"), cell(a, "note_number_default_value")), ("10".into(), "36".into(), "38".into(), "37".into()));
    assert_eq!((cell(a, "comments"), cell(a, "velocity_usage")), ("choke".into(), "soft".into()));
    assert_eq!((cell(b, "midi_channel"), cell(b, "note_number_min_value"), cell(b, "note_number_max_value")), ("".into(), "40".into(), "40".into()));
    assert_eq!((cell(b, "note_number_default_value"), cell(b, "section"), cell(b, "comments")), ("".into(), "".into(), "".into()));
}

#[test]
fn the_channel_is_written_from_1_to_16() {
    let d = device(
        r#", "triggers": {"entries": [
        {"osc": "/a", "sound": "A", "channel": 0, "note_range": [1, 2]},
        {"osc": "/b", "sound": "B", "channel": 15, "note_range": [1, 2]}]}"#,
    );
    let records = parse(&export(&d, Format::MidiTriggersCsv).unwrap().value);
    assert_eq!((cell(&records[1], "midi_channel"), cell(&records[2], "midi_channel")), ("1".into(), "16".into()));
}

#[test]
fn equal_bounds_are_written_twice() {
    let d = device(r#", "triggers": {"entries": [{"osc": "/a", "sound": "A", "note_range": [60, 60], "note": 60}]}"#);
    let r = &parse(&export(&d, Format::MidiTriggersCsv).unwrap().value)[1];
    assert_eq!((cell(r, "note_number_min_value"), cell(r, "note_number_max_value"), cell(r, "note_number_default_value")), ("60".into(), "60".into(), "60".into()));
}

#[test]
fn a_cell_with_a_comma_a_quote_or_a_line_break_is_quoted_and_reads_back() {
    let r = trigger_row(&[("sound_name", "Cut, \"off\""), ("note_number_min_value", "1"), ("note_number_max_value", "2"), ("comments", "line one\nline two"), ("velocity_usage", "a, b")]);
    let imported = import(&trigger_csv(std::slice::from_ref(&r)), Format::MidiTriggersCsv, &source()).unwrap();
    let out = export(&resolved(imported.value), Format::MidiTriggersCsv).unwrap();
    assert_eq!(parse(&out.value)[1], r);
}

#[test]
fn a_device_without_triggers_gives_a_header_only_file() {
    let t = export(&device(""), Format::MidiTriggersCsv).unwrap();
    assert_eq!(t.value, format!("{}\n", trigger_header_line()));
}

#[test]
fn every_block_a_device_holds_that_the_csv_cannot_carry_is_in_the_losses_by_its_pointer() {
    let d = device(
        r#", "sysex": {"header": [240], "footer": [247]},
        "cc_params": {"entries": [{"osc": "/c", "cc": 1}]},
        "triggers": {"entries": [{"osc": "/a", "sound": "A", "note_range": [1, 2]}]}"#,
    );
    let t = export(&d, Format::MidiTriggersCsv).unwrap();
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    for pointer in ["/sysex", "/cc_params"] {
        assert!(places.contains(&pointer), "{pointer} not in {places:?}");
    }
    assert!(!places.contains(&"/triggers") && !places.contains(&"/device"), "{places:?}");
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn the_identity_beyond_vendor_and_name_is_in_the_losses_by_its_pointer() {
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.5/device.schema.json", {SOURCES}, "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler", "revision": "1.2"}}}}"#
    );
    let t = export(&resolved(device_from(&text)), Format::MidiTriggersCsv).unwrap();
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    assert!(places.iter().any(|p| p.starts_with("/device/")), "{places:?}");
    assert!(!places.contains(&"/device/name") && !places.contains(&"/device/vendor"), "{places:?}");
}

#[test]
fn a_channel_mode_is_in_the_losses_by_its_pointer() {
    let d = device(r#", "triggers": {"entries": [{"osc": "/a", "sound": "A", "note_range": [1, 2]}, {"osc": "/b", "sound": "B", "channel_mode": "omni", "note_range": [3, 4]}]}"#);
    let t = export(&d, Format::MidiTriggersCsv).unwrap();
    assert!(t.lost.iter().any(|l| l.place == "/triggers/entries/1/channel_mode"), "{:?}", t.lost);
    assert_eq!(parse(&t.value).len(), 3);
}

#[test]
fn the_losses_are_the_same_on_each_export() {
    let d = device(r#", "sysex": {"header": [240], "footer": [247]}, "triggers": {"entries": [{"osc": "/a", "sound": "A", "channel_mode": "omni", "note_range": [1, 2]}]}"#);
    assert_eq!(export(&d, Format::MidiTriggersCsv).unwrap(), export(&d, Format::MidiTriggersCsv).unwrap());
}

#[test]
fn a_device_without_sources_is_refused_with_a_typed_fault() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let f = export(&resolved(device_from(text)), Format::MidiTriggersCsv).expect_err("refused");
    assert_eq!(f.kind, FaultKind::NoSources);
    assert!(f.place.contains("_sources"), "{:?}", f.place);
}
