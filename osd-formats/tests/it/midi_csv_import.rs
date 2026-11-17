//! The import of a pencilresearch/midi CSV: the descriptor, its source, its refusals and its losses.

use crate::common::{csv, entries, header_line, row, source};
use osd_formats::{import, FaultKind, Format};
use serde_json::Value;

fn ok(text: &str) -> osd_formats::Translated<osd::Device> {
    import(text, Format::MidiCsv, &source()).unwrap_or_else(|f| panic!("refused: {f:?}"))
}

fn refused(text: &str) -> osd_formats::Fault {
    match import(text, Format::MidiCsv, &source()) {
        Ok(_) => panic!("the file is accepted"),
        Err(f) => f,
    }
}

fn glide() -> Vec<String> {
    row(&[
        ("manufacturer", "Lofty"),
        ("device", "Trundler"),
        ("section", "Oscillators"),
        ("parameter_name", "Glide switch"),
        ("parameter_description", "Enables or disables glide"),
        ("cc_msb", "65"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("orientation", "0-based"),
        ("notes", "Typo in manual (says CC 66; 65 is correct)"),
        ("usage", "0-63: Off; 64-127: On"),
    ])
}

fn remark(e: &osd::CcEntry, key: &str) -> Option<String> {
    e.remarks.get(key).and_then(Value::as_str).map(str::to_string)
}

#[test]
fn a_row_with_a_cc_becomes_a_control_with_its_name_section_and_range() {
    let t = ok(&csv(&[glide()]));
    let list = entries(&t.value);
    assert_eq!(list.len(), 1);
    let e = &list[0];
    assert_eq!(e.cc, Some(65));
    assert_eq!(e.cc_lsb, None);
    assert_eq!(e.range, Some([0, 127]));
    assert_eq!(e.section.as_deref(), Some("Oscillators"));
    assert_eq!(e.orientation, Some(osd::Orientation::ZeroBased));
    assert_eq!(remark(e, "_name").as_deref(), Some("Glide switch"));
    assert_eq!(remark(e, "_description").as_deref(), Some("Enables or disables glide"));
    assert_eq!(remark(e, "_notes").as_deref(), Some("Typo in manual (says CC 66; 65 is correct)"));
    assert!(e.osc.starts_with('/'));
}

#[test]
fn the_usage_text_is_carried_whole_whatever_its_length() {
    let long = "0-63: Off; 64-127: On; ".repeat(30);
    let mut r = glide();
    r[17] = long.clone();
    let t = ok(&csv(&[r]));
    assert_eq!(remark(&entries(&t.value)[0], "_usage"), Some(long));
}

#[test]
fn the_manufacturer_and_device_name_the_descriptor_device() {
    let t = ok(&csv(&[glide()]));
    assert_eq!(t.value.device.vendor.as_deref(), Some("Lofty"));
    assert_eq!(t.value.device.name, "Trundler");
    assert!(t.value.device.osc_prefix.starts_with('/'));
}

#[test]
fn the_descriptor_is_of_the_latest_version_and_reads_strictly() {
    let t = ok(&csv(&[glide()]));
    assert_eq!(t.value.schema, Some(osd::Version::V0_5));
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
    match osd::read(&text, osd::Kind::Device) {
        Ok(osd::Descriptor::Device(back)) => assert_eq!(*back, t.value),
        other => panic!("not read back strictly: {other:?}"),
    }
}

#[test]
fn a_cc_with_an_lsb_becomes_a_14_bit_pair_with_the_range_of_the_cc_columns() {
    let r = row(&[
        ("parameter_name", "Cutoff"),
        ("cc_msb", "1"),
        ("cc_lsb", "33"),
        ("cc_min_value", "0"),
        ("cc_max_value", "16383"),
        ("orientation", "centered"),
    ]);
    let e = &entries(&ok(&csv(&[r])).value)[0];
    assert_eq!((e.cc, e.cc_lsb), (Some(1), Some(33)));
    assert_eq!(e.range, Some([0, 16383]));
    assert_eq!(e.orientation, Some(osd::Orientation::Centered));
}

#[test]
fn an_nrpn_becomes_nrpn_msb_and_lsb_with_the_range_of_the_nrpn_columns() {
    let r = row(&[
        ("parameter_name", "Depth"),
        ("nrpn_msb", "3"),
        ("nrpn_lsb", "9"),
        ("nrpn_min_value", "10"),
        ("nrpn_max_value", "200"),
    ]);
    let list = entries(&ok(&csv(&[r])).value);
    assert_eq!(list.len(), 1);
    let e = &list[0];
    assert_eq!((e.cc, e.nrpn_msb, e.nrpn_lsb), (None, Some(3), Some(9)));
    assert_eq!(e.range, Some([10, 200]));
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_gives_two_controls_with_their_own_ranges() {
    let r = row(&[
        ("section", "Filter"),
        ("parameter_name", "Cutoff"),
        ("cc_msb", "74"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("nrpn_msb", "1"),
        ("nrpn_lsb", "2"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "16383"),
    ]);
    let list = entries(&ok(&csv(&[r])).value);
    assert_eq!(list.len(), 2);
    assert_eq!((list[0].cc, list[0].nrpn_msb, list[0].range), (Some(74), None, Some([0, 127])));
    assert_eq!((list[1].cc, list[1].nrpn_msb, list[1].nrpn_lsb, list[1].range), (None, Some(1), Some(2), Some([0, 16383])));
    for e in &list {
        assert_eq!(remark(e, "_name").as_deref(), Some("Cutoff"));
        assert_eq!(e.section.as_deref(), Some("Filter"));
    }
}

#[test]
fn each_control_address_is_unique_even_for_rows_of_the_same_name() {
    let same = |cc: &str| row(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", cc), ("nrpn_msb", "5"), ("nrpn_lsb", "6")]);
    let list = entries(&ok(&csv(&[same("1"), same("2"), same("3")])).value);
    assert_eq!(list.len(), 6);
    let mut seen: Vec<&str> = list.iter().map(|e| e.osc.as_str()).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 6, "addresses repeat: {:?}", list.iter().map(|e| &e.osc).collect::<Vec<_>>());
}

#[test]
fn an_empty_cell_gives_no_value_the_source_does_not_give() {
    let r = row(&[("parameter_name", "Bare"), ("cc_msb", "7")]);
    let e = &entries(&ok(&csv(&[r])).value)[0];
    assert_eq!((e.range, e.orientation, e.section.clone(), e.cc_lsb), (None, None, None, None));
    assert_eq!(remark(e, "_description"), None);
    assert_eq!(remark(e, "_notes"), None);
    assert_eq!(remark(e, "_usage"), None);
}

#[test]
fn the_descriptor_holds_the_source_as_given() {
    let src = source();
    let t = import(&csv(&[glide()]), Format::MidiCsv, &src).unwrap();
    assert_eq!(t.value.sources, Some(vec![src]));
}

#[test]
fn an_import_without_license_is_refused_naming_the_license() {
    let mut src = source();
    src.license = None;
    let f = import(&csv(&[glide()]), Format::MidiCsv, &src).expect_err("refused");
    assert_eq!(f.kind, FaultKind::SourceIncomplete);
    assert!(f.place.contains("license"), "{:?}", f.place);
}

#[test]
fn an_import_without_url_author_or_attribution_is_refused_naming_them() {
    let mut src = source();
    (src.url, src.author, src.attribution) = (None, None, None);
    let f = import(&csv(&[glide()]), Format::MidiCsv, &src).expect_err("refused");
    assert_eq!(f.kind, FaultKind::SourceIncomplete);
    for field in ["url", "author", "attribution"] {
        assert!(f.place.contains(field), "{:?} does not name {field}", f.place);
    }
}

#[test]
fn one_of_url_author_or_attribution_is_enough() {
    for keep in 0..3 {
        let mut src = source();
        (src.url, src.author, src.attribution) = (None, None, None);
        match keep {
            0 => src.url = Some("https://example.org/x".into()),
            1 => src.author = Some("Someone".into()),
            _ => src.attribution = Some("Someone, 2026".into()),
        }
        assert!(import(&csv(&[glide()]), Format::MidiCsv, &src).is_ok(), "case {keep}");
    }
}

#[test]
fn a_file_without_its_header_is_refused_at_line_1() {
    for text in ["", "Lofty,Trundler,Oscillators\n", "manufacturer,device\nLofty,Trundler\n"] {
        let f = refused(text);
        assert_eq!(f.kind, FaultKind::InvalidFile, "{text:?}");
        assert!(f.place.starts_with("line 1"), "{text:?}: {:?}", f.place);
    }
}

#[test]
fn a_row_whose_column_count_differs_from_the_header_is_refused_at_its_line() {
    let text = format!("{}\n{}\nLofty,Trundler,only three\n", header_line(), crate::common::line(&glide().iter().map(|c| c.as_str()).collect::<Vec<_>>()));
    let f = refused(&text);
    assert_eq!(f.kind, FaultKind::InvalidFile);
    assert!(f.place.starts_with("line 3"), "{:?}", f.place);
}

#[test]
fn a_cc_or_nrpn_number_outside_0_to_127_is_refused_at_its_line() {
    let cases = [("cc_msb", "128"), ("cc_lsb", "200"), ("nrpn_msb", "128"), ("nrpn_lsb", "-1"), ("cc_msb", "x")];
    for (column, value) in cases {
        let mut cells = vec![("cc_msb", "1"), ("nrpn_msb", "1"), ("nrpn_lsb", "1")];
        cells.retain(|(c, _)| *c != column);
        cells.push((column, value));
        let f = refused(&csv(&[glide(), row(&cells)]));
        assert_eq!(f.kind, FaultKind::InvalidFile, "{column}={value}");
        assert!(f.place.starts_with("line 3"), "{column}={value}: {:?}", f.place);
    }
}

#[test]
fn a_minimum_above_its_maximum_is_refused_at_its_line() {
    for (min, max) in [("cc_min_value", "cc_max_value"), ("nrpn_min_value", "nrpn_max_value")] {
        let r = row(&[("cc_msb", "1"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), (min, "100"), (max, "50")]);
        let f = refused(&csv(&[glide(), r]));
        assert_eq!(f.kind, FaultKind::InvalidFile, "{min}");
        assert!(f.place.starts_with("line 3"), "{min}: {:?}", f.place);
    }
}

#[test]
fn a_row_with_neither_cc_nor_nrpn_is_no_control_and_is_in_the_losses_with_its_line() {
    let none = row(&[("section", "Misc"), ("parameter_name", "Sysex only"), ("notes", "SysEx only")]);
    let t = ok(&csv(&[glide(), none]));
    assert_eq!(entries(&t.value).len(), 1);
    let lost: Vec<_> = t.lost.iter().filter(|l| l.place == "line 3").collect();
    assert_eq!(lost.len(), 1, "{:?}", t.lost);
    assert!(!lost[0].reason.is_empty());
}

#[test]
fn a_default_value_is_in_the_losses_by_its_line_and_column() {
    let r = row(&[
        ("parameter_name", "Both"),
        ("cc_msb", "1"),
        ("cc_default_value", "64"),
        ("nrpn_msb", "2"),
        ("nrpn_lsb", "3"),
        ("nrpn_default_value", "10"),
    ]);
    let t = ok(&csv(&[glide(), r]));
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, ["line 3, column cc_default_value", "line 3, column nrpn_default_value"]);
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn an_empty_default_cell_is_not_a_loss() {
    assert!(ok(&csv(&[glide()])).lost.is_empty());
}

#[test]
fn an_orientation_the_format_does_not_hold_is_a_loss_not_a_value() {
    let r = row(&[("parameter_name", "Odd"), ("cc_msb", "1"), ("orientation", "upside-down")]);
    let t = ok(&csv(&[r]));
    assert_eq!(entries(&t.value)[0].orientation, None);
    assert_eq!(t.lost.iter().map(|l| l.place.as_str()).collect::<Vec<_>>(), ["line 2, column orientation"]);
}

#[test]
fn a_device_or_manufacturer_that_changes_between_rows_is_a_loss_at_its_cell() {
    let mut other = glide();
    other[1] = "Other".into();
    let t = ok(&csv(&[glide(), other]));
    assert_eq!(t.value.device.name, "Trundler");
    assert_eq!(t.lost.iter().map(|l| l.place.as_str()).collect::<Vec<_>>(), ["line 3, column device"]);
}

#[test]
fn the_losses_come_in_the_order_of_the_source_and_are_the_same_on_each_import() {
    let a = row(&[("cc_msb", "1"), ("cc_default_value", "1")]);
    let none = row(&[("parameter_name", "Nothing")]);
    let b = row(&[("cc_msb", "2"), ("cc_default_value", "2"), ("orientation", "weird")]);
    let text = csv(&[a, none, b]);
    let first = ok(&text);
    let places: Vec<&str> = first.lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, ["line 2, column cc_default_value", "line 3", "line 4, column cc_default_value", "line 4, column orientation"]);
    assert_eq!(first.lost, ok(&text).lost);
}
