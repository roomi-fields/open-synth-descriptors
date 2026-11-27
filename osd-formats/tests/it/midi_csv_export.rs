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

/// The losses but those of a title the export wrote from the address: these controls have none.
fn but_titles(lost: &[osd_formats::Loss]) -> impl Iterator<Item = &osd_formats::Loss> {
    lost.iter().filter(|l| !l.place.ends_with("/title"))
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
fn a_device_with_only_a_sysex_block_loses_the_block_and_what_a_csv_without_a_row_does_not_give_back() {
    let t = export(&device(r#", "sysex": {"header": [240], "footer": [247]}"#), Format::MidiCsv).unwrap();
    let mut places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    places.sort();
    // No row says the device's name, manufacturer or prefix, and the import adds `cc_params`.
    assert_eq!(places, ["/cc_params", "/device/name", "/device/osc_prefix", "/device/vendor", "/sources", "/sysex"]);
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()), "{:?}", t.lost);
}

#[test]
fn the_sources_of_a_device_are_in_the_losses_once_with_the_reason_the_csv_has_no_column_for_the_credit() {
    let t = export(&device(""), Format::MidiCsv).unwrap();
    let lost: Vec<_> = t.lost.iter().filter(|l| l.place == "/sources").collect();
    assert_eq!(lost.len(), 1, "{:?}", t.lost);
    let reason = lost[0].reason.to_lowercase();
    assert!(reason.contains("column") && reason.contains("credit") && reason.contains("licence"), "{reason}");
}

#[test]
fn several_sources_still_give_one_loss_of_the_sources() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "_sources": [
        {"type": "community-re", "url": "https://example.org/a", "license": "CC0-1.0"},
        {"type": "vendor-doc", "author": "Lofty", "license": "CC-BY-4.0"},
        {"type": "community-re", "url": "https://example.org/c", "license": "MIT"}],
        "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let t = export(&resolved(device_from(text)), Format::MidiCsv).unwrap();
    assert_eq!(t.lost.iter().filter(|l| l.place == "/sources").count(), 1, "{:?}", t.lost);
    assert_eq!(parse(&t.value), vec![HEADER.to_vec()]);
}

#[test]
fn a_device_without_sources_is_refused_and_gives_no_loss() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "device": {"name": "T", "osc_prefix": "/t"}}"#;
    assert_eq!(export(&resolved(device_from(text)), Format::MidiCsv).expect_err("refused").kind, FaultKind::NoSources);
}

#[test]
fn the_credit_of_the_sources_is_written_in_no_cell_of_the_csv() {
    let d = device(r#", "cc_params": {"entries": [{"osc": "/a", "cc": 1}]}"#);
    let t = export(&d, Format::MidiCsv).unwrap();
    assert!(!t.value.contains("example.org") && !t.value.contains("CC0-1.0"), "{}", t.value);
    assert_eq!(parse(&t.value).len(), 2);
    assert!(parse(&t.value).iter().all(|r| r.len() == 18));
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
    assert_eq!(f.place, "/sources");
}

#[test]
fn a_device_with_an_empty_list_of_sources_is_refused_too() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.5/device.schema.json", "_sources": [], "device": {"name": "T", "osc_prefix": "/t"}}"#;
    let f = export(&resolved(device_from(text)), Format::MidiCsv).expect_err("refused");
    assert_eq!(f.kind, FaultKind::NoSources);
}

/// A 0.6 device, the version that gives a control its default.
fn device_06(body: &str) -> osd::Resolved {
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.6/device.schema.json", {SOURCES}, "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}}{body}}}"#
    );
    resolved(device_from(&text))
}

#[test]
fn the_default_of_a_control_with_a_cc_goes_to_the_cc_default_cell() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "cc": 74, "range": [0, 127], "default": 64},
        {"osc": "/b", "cc": 1, "cc_lsb": 33, "range": [0, 16383], "default": 8192},
        {"osc": "/c", "cc": 5, "default": 0}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!(cell(&records[1], "cc_default_value"), "64");
    assert_eq!(cell(&records[2], "cc_default_value"), "8192");
    assert_eq!(cell(&records[3], "cc_default_value"), "0");
    assert!(records[1..].iter().all(|r| cell(r, "nrpn_default_value").is_empty()));
    assert!(!t.lost.iter().any(|l| l.place.ends_with("/default")), "{:?}", t.lost);
}

#[test]
fn the_default_of_a_control_with_only_an_nrpn_goes_to_the_nrpn_default_cell() {
    let d = device_06(r#", "cc_params": {"entries": [{"osc": "/c", "nrpn_msb": 3, "nrpn_lsb": 9, "value_bits": 14, "range": [10, 200], "default": 100}]}"#);
    let t = export(&d, Format::MidiCsv).unwrap();
    let r = &parse(&t.value)[1];
    assert_eq!((cell(r, "nrpn_default_value"), cell(r, "cc_default_value")), ("100".into(), "".into()));
    assert!(!but_titles(&t.lost).any(|l| l.place != "/sources"), "{:?}", t.lost);
}

#[test]
fn the_default_of_a_control_with_a_cc_and_an_nrpn_goes_to_the_cc_cell_and_the_nrpn_cell_stays_empty() {
    let d = device_06(r#", "cc_params": {"entries": [{"osc": "/c", "cc": 74, "range": [0, 127], "default": 64, "also": [{"nrpn_msb": 3, "nrpn_lsb": 9, "value_bits": 14}]}]}"#);
    let t = export(&d, Format::MidiCsv).unwrap();
    let r = &parse(&t.value)[1];
    assert_eq!((cell(r, "cc_default_value"), cell(r, "nrpn_default_value")), ("64".into(), "".into()));
    assert!(!t.lost.iter().any(|l| l.place.ends_with("/default")), "{:?}", t.lost);
}

#[test]
fn a_default_on_a_control_with_neither_a_cc_nor_an_nrpn_stays_in_the_losses() {
    let d = device_06(r#", "cc_params": {"entries": [{"osc": "/c", "rpn_msb": 0, "rpn_lsb": 1, "value_bits": 14, "default": 5}]}"#);
    let t = export(&d, Format::MidiCsv).unwrap();
    let r = &parse(&t.value)[1];
    assert_eq!((cell(r, "cc_default_value"), cell(r, "nrpn_default_value")), ("".into(), "".into()));
    let lost: Vec<_> = t.lost.iter().filter(|l| l.place == "/cc_params/entries/0/default").collect();
    assert_eq!(lost.len(), 1, "{:?}", t.lost);
    assert!(!lost[0].reason.is_empty());
}

#[test]
fn a_control_without_a_default_leaves_the_default_cells_empty() {
    let d = device_06(r#", "cc_params": {"entries": [{"osc": "/a", "cc": 74, "range": [0, 127]}]}"#);
    let r = &parse(&export(&d, Format::MidiCsv).unwrap().value)[1];
    assert_eq!((cell(r, "cc_default_value"), cell(r, "nrpn_default_value")), ("".into(), "".into()));
}

#[test]
fn a_control_with_a_cc_and_an_nrpn_way_gives_one_row_with_both_in_their_columns() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "cc": 74, "range": [0, 127], "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383]}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!(records.len(), 2);
    let r = &records[1];
    assert_eq!((cell(r, "cc_msb"), cell(r, "cc_min_value"), cell(r, "cc_max_value")), ("74".into(), "0".into(), "127".into()));
    assert_eq!((cell(r, "nrpn_msb"), cell(r, "nrpn_lsb"), cell(r, "nrpn_min_value"), cell(r, "nrpn_max_value")), ("1".into(), "2".into(), "0".into(), "16383".into()));
    assert!(but_titles(&t.lost).all(|l| !l.place.starts_with("/cc_params/entries/0")), "{:?}", t.lost);
}

#[test]
fn the_nrpn_way_written_before_the_cc_way_still_gives_the_cc_and_the_nrpn_in_their_columns() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383], "also": [{"cc": 74, "range": [0, 127]}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!(records.len(), 2);
    let r = &records[1];
    assert_eq!((cell(r, "cc_msb"), cell(r, "cc_max_value")), ("74".into(), "127".into()));
    assert_eq!((cell(r, "nrpn_msb"), cell(r, "nrpn_lsb"), cell(r, "nrpn_max_value")), ("1".into(), "2".into(), "16383".into()));
    assert!(
        but_titles(&t.lost).all(|l| !l.place.starts_with("/cc_params/entries/0") || l.reason.contains("order of the ways")),
        "{:?}",
        t.lost
    );
}

#[test]
fn a_second_nrpn_way_is_a_loss_at_its_pointer_and_the_first_nrpn_still_fills_its_row() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "cc": 74, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14}, {"nrpn_msb": 3, "nrpn_lsb": 4, "value_bits": 14}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!(records.len(), 2);
    assert_eq!((cell(&records[1], "cc_msb"), cell(&records[1], "nrpn_msb"), cell(&records[1], "nrpn_lsb")), ("74".into(), "1".into(), "2".into()));
    let at: Vec<_> = but_titles(&t.lost).filter(|l| l.place.starts_with("/cc_params/entries/0")).collect();
    assert_eq!(at.len(), 1, "{:?}", t.lost);
    assert_eq!(at[0].place, "/cc_params/entries/0/ways/2");
    assert!(!at[0].reason.is_empty());
}

#[test]
fn a_second_cc_way_and_an_rpn_way_are_losses_at_their_pointers() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "cc": 74, "also": [{"cc": 1, "cc_lsb": 33}, {"rpn_msb": 0, "rpn_lsb": 2, "value_bits": 14}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!((records.len(), cell(&records[1], "cc_msb"), cell(&records[1], "cc_lsb")), (2, "74".into(), "".into()));
    let mut places: Vec<&str> = but_titles(&t.lost).map(|l| l.place.as_str()).filter(|p| p.starts_with("/cc_params/entries/0")).collect();
    places.sort();
    assert_eq!(places, ["/cc_params/entries/0/ways/1", "/cc_params/entries/0/ways/2"]);
}

#[test]
fn the_default_of_a_control_whose_first_way_is_an_nrpn_goes_to_the_nrpn_cell_when_the_ranges_differ() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383], "default": 8192, "also": [{"cc": 74, "range": [0, 127]}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let r = &parse(&t.value)[1];
    assert_eq!((cell(r, "nrpn_default_value"), cell(r, "cc_default_value")), ("8192".into(), "".into()));
    assert!(!t.lost.iter().any(|l| l.place.ends_with("/default")), "{:?}", t.lost);
}

#[test]
fn the_default_stays_in_the_cc_cell_when_the_cc_is_first_or_the_two_ways_share_a_range() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "cc": 74, "range": [0, 127], "default": 64, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383]}]},
        {"osc": "/b", "nrpn_msb": 1, "nrpn_lsb": 3, "value_bits": 14, "range": [0, 127], "default": 5, "also": [{"cc": 75, "range": [0, 127]}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    assert_eq!((cell(&records[1], "cc_default_value"), cell(&records[1], "nrpn_default_value")), ("64".into(), "".into()));
    assert_eq!((cell(&records[2], "cc_default_value"), cell(&records[2], "nrpn_default_value")), ("5".into(), "".into()));
    assert!(!t.lost.iter().any(|l| l.place.ends_with("/default")), "{:?}", t.lost);
}

#[test]
fn the_default_of_a_control_whose_first_way_is_an_rpn_is_a_loss_when_a_cc_or_an_nrpn_comes_later() {
    let d = device_06(
        r#", "cc_params": {"entries": [
        {"osc": "/a", "rpn_msb": 0, "rpn_lsb": 1, "value_bits": 14, "default": 5, "also": [{"cc": 74, "range": [0, 127]}]},
        {"osc": "/b", "rpn_msb": 0, "rpn_lsb": 2, "value_bits": 14, "default": 6, "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14}]}]}"#,
    );
    let t = export(&d, Format::MidiCsv).unwrap();
    let records = parse(&t.value);
    for r in &records[1..] {
        assert_eq!((cell(r, "cc_default_value"), cell(r, "nrpn_default_value")), ("".into(), "".into()));
    }
    for i in 0..2 {
        let place = format!("/cc_params/entries/{i}/default");
        let lost: Vec<_> = t.lost.iter().filter(|l| l.place == place).collect();
        assert_eq!(lost.len(), 1, "{:?}", t.lost);
        assert!(!lost[0].reason.is_empty());
    }
}
