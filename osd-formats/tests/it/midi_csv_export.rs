//! The export of a device as a pencilresearch/midi CSV: its rows, its refusals and its losses.

use crate::common::{csv, device_from, header_line, parse, resolved, row, source, HEADER};
use osd_formats::{export, import, FaultKind, Format};

const SOURCES: &str = r#""_sources": [{"type": "community-re", "url": "https://example.org/x", "license": "CC0-1.0"}]"#;

fn device(body: &str) -> osd::Resolved {
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.5/device.schema.json", {SOURCES}, "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}{body}}}"#
    );
    resolved(device_from(&text))
}

fn cell(record: &[String], column: &str) -> String {
    record[HEADER.iter().position(|h| *h == column).unwrap()].clone()
}

#[test]
fn a_device_exports_the_header_and_one_row_per_cc_params_entry() {
    let d = device(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "cc": 74, "range": [0, 127], "section": "Filter"},
        {"osc": "/b", "cc": 1, "cc_lsb": 33, "range": [0, 16383], "orientation": "centered"},
        {"osc": "/c", "nrpn_msb": 3, "nrpn_lsb": 9, "range": [10, 200]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!(records.len(), 4);
    assert_eq!(records[0], HEADER.to_vec());
    assert!(t.value.starts_with(&format!("{}\n", header_line())));
    let (a, b, c) = (&records[1], &records[2], &records[3]);
    assert_eq!((cell(a, "manufacturer"), cell(a, "device")), ("Lofty".into(), "Trundler".into()));
    assert_eq!((cell(a, "cc_msb"), cell(a, "cc_min_value"), cell(a, "cc_max_value"), cell(a, "section")), ("74".into(), "0".into(), "127".into(), "Filter".into()));
    assert_eq!((cell(b, "cc_msb"), cell(b, "cc_lsb"), cell(b, "cc_max_value"), cell(b, "orientation")), ("1".into(), "33".into(), "16383".into(), "centered".into()));
    assert_eq!((cell(c, "cc_msb"), cell(c, "nrpn_msb"), cell(c, "nrpn_lsb")), ("".into(), "3".into(), "9".into()));
    assert_eq!((cell(c, "nrpn_min_value"), cell(c, "nrpn_max_value"), cell(c, "cc_min_value")), ("10".into(), "200".into(), "".into()));
}

#[test]
fn a_cell_with_a_comma_a_quote_or_a_line_break_is_quoted_and_reads_back() {
    let r = row(&[("parameter_name", "Cut, \"off\""), ("cc_msb", "1"), ("notes", "line one\nline two"), ("usage", "0-63: Off, 64-127: On")]);
    let imported = import(&csv(std::slice::from_ref(&r)), Format::MidiCsv, &source()).unwrap();
    let out = export(&resolved(imported.value), Format::MidiCsv).unwrap();
    assert_eq!(parse(&out.value)[1], r);
}

#[test]
fn a_device_without_cc_params_gives_a_header_only_file() {
    let t = export(&device(""), Format::MidiCsv).unwrap();
    assert_eq!(t.value, format!("{}\n", header_line()));
}

#[test]
fn every_block_a_device_holds_that_the_csv_cannot_carry_is_in_the_losses_by_its_pointer() {
    let example = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/0.5/synth.device.json")).unwrap();
    let t = export(&resolved(device_from(&example)), Format::MidiCsv).unwrap();
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    for pointer in ["/sysex", "/triggers"] {
        assert!(places.contains(&pointer), "{pointer} not in {places:?}");
    }
    assert!(!places.contains(&"/cc_params") && !places.contains(&"/device"), "{places:?}");
    let at = |p: &str| places.iter().position(|x| *x == p).unwrap();
    assert!(at("/sysex") < at("/triggers"), "{places:?}");
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn a_device_with_only_a_sysex_block_loses_exactly_that_block() {
    let t = export(&device(r#", "sysex": {"header": [240], "footer": [247]}"#), Format::MidiCsv).unwrap();
    assert_eq!(t.lost.iter().map(|l| l.place.as_str()).collect::<Vec<_>>(), ["/sysex"]);
}

#[test]
fn a_transform_on_a_control_is_in_the_losses_by_its_pointer() {
    let d = device(r#", "capabilities": ["lua"], "cc_params": {"entries": [{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 2, "transform": "x * 2"}]}"#);
    let t = export(&d, Format::MidiCsv).unwrap();
    assert_eq!(parse(&t.value).len(), 3);
    assert!(t.lost.iter().any(|l| l.place == "/cc_params/entries/1/transform"), "{:?}", t.lost);
}

#[test]
fn a_control_field_the_csv_has_no_column_for_is_in_the_losses_by_its_pointer() {
    let d = device(r#", "cc_params": {"entries": [{"osc": "/a", "cc": 1, "channel": 3}]}"#);
    let t = export(&d, Format::MidiCsv).unwrap();
    assert!(t.lost.iter().any(|l| l.place == "/cc_params/entries/0/channel"), "{:?}", t.lost);
}

#[test]
fn the_losses_are_the_same_on_each_export() {
    let d = device(r#", "sysex": {"header": [240], "footer": [247]}, "cc_params": {"entries": [{"osc": "/a", "cc": 1, "channel": 3}]}"#);
    assert_eq!(export(&d, Format::MidiCsv).unwrap(), export(&d, Format::MidiCsv).unwrap());
}

#[test]
fn a_device_without_sources_is_refused_with_a_typed_fault() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let f = export(&resolved(device_from(text)), Format::MidiCsv).expect_err("refused");
    assert_eq!(f.kind, FaultKind::NoSources);
    assert!(f.place.contains("_sources"), "{:?}", f.place);
}

#[test]
fn a_device_with_an_empty_list_of_sources_is_refused_too() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "_sources": [], "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let f = export(&resolved(device_from(text)), Format::MidiCsv).expect_err("refused");
    assert_eq!(f.kind, FaultKind::NoSources);
}
