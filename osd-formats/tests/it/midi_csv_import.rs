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
fn the_descriptor_is_a_0_6_descriptor_and_reads_strictly() {
    let t = ok(&csv(&[glide()]));
    assert_eq!(t.value.schema, Some(osd::Version::V0_6));
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
fn an_nrpn_of_a_csv_row_is_read_with_the_14_bits_of_the_data_msb_and_lsb() {
    let r = row(&[("parameter_name", "Depth"), ("nrpn_msb", "3"), ("nrpn_lsb", "9")]);
    let list = entries(&ok(&csv(&[r])).value);
    assert_eq!(list[0].value_bits, Some(14));
    let cc = row(&[("parameter_name", "Cutoff"), ("cc_msb", "74")]);
    assert_eq!(entries(&ok(&csv(&[cc])).value)[0].value_bits, None);
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_gives_one_control_with_the_cc_first_and_the_nrpn_in_also_each_with_its_own_range() {
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
    assert_eq!(list.len(), 1);
    let e = &list[0];
    assert_eq!((e.cc, e.nrpn_msb, e.range), (Some(74), None, Some([0, 127])));
    let also = e.also.as_ref().expect("the nrpn is a further way");
    assert_eq!(also.len(), 1);
    assert_eq!((also[0].cc, also[0].nrpn_msb, also[0].nrpn_lsb, also[0].range), (None, Some(1), Some(2), Some([0, 16383])));
    let ways = e.ways();
    assert_eq!((ways.len(), ways[0].cc, ways[1].nrpn_msb), (2, Some(74), Some(1)));
    assert_eq!(remark(e, "_name").as_deref(), Some("Cutoff"));
    assert_eq!(e.section.as_deref(), Some("Filter"));
    assert_eq!(e.osc, "/filter/cutoff");
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_and_one_range_gives_that_range_to_both_ways() {
    let r = row(&[("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("nrpn_msb", "1"), ("nrpn_lsb", "2")]);
    let t = ok(&csv(&[r]));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
    let ways = entries(&t.value)[0].ways();
    assert_eq!((ways.len(), ways[0].range, ways[1].range), (2, Some([0, 127]), Some([0, 127])));
}

#[test]
fn a_row_with_a_14_bit_cc_and_an_nrpn_keeps_the_pair_in_the_first_way() {
    let r = row(&[("parameter_name", "Cutoff"), ("cc_msb", "1"), ("cc_lsb", "33"), ("nrpn_msb", "1"), ("nrpn_lsb", "2")]);
    let t = ok(&csv(&[r]));
    let list = entries(&t.value);
    assert_eq!(list.len(), 1);
    let ways = list[0].ways();
    assert_eq!((ways[0].cc, ways[0].cc_lsb, ways[1].nrpn_msb, ways[1].nrpn_lsb), (Some(1), Some(33), Some(1), Some(2)));
    // The imported descriptor stays valid: it reads back strictly.
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    assert!(osd::read(&text, osd::Kind::Device).is_ok());
}

#[test]
fn each_control_address_is_unique_even_for_rows_of_the_same_name() {
    let same = |n: &str| row(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", n), ("nrpn_msb", n), ("nrpn_lsb", "6")]);
    let t = ok(&csv(&[same("1"), same("2"), same("3")]));
    let list = entries(&t.value);
    assert_eq!(list.len(), 3);
    let mut seen: Vec<&str> = list.iter().map(|e| e.osc.as_str()).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 3, "addresses repeat: {:?}", list.iter().map(|e| &e.osc).collect::<Vec<_>>());
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    assert!(osd::read(&text, osd::Kind::Device).is_ok());
}

#[test]
fn rows_that_claim_the_same_cc_give_a_descriptor_that_reads_and_name_the_loss() {
    let same = |name: &str| row(&[("parameter_name", name), ("cc_msb", "74")]);
    let t = ok(&csv(&[same("Cutoff"), same("Brightness")]));
    assert!(!t.lost.is_empty(), "the claimed message is dropped without a word");
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "the import does not read in 0.6");
}

#[test]
fn rows_that_claim_the_same_nrpn_give_a_descriptor_that_reads_and_name_the_loss() {
    let same = |name: &str| row(&[("parameter_name", name), ("nrpn_msb", "5"), ("nrpn_lsb", "6")]);
    let t = ok(&csv(&[same("Cutoff"), same("Brightness"), same("Drive")]));
    assert!(!t.lost.is_empty(), "the claimed message is dropped without a word");
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "the import does not read in 0.6");
}

#[test]
fn rows_with_a_cc_and_the_same_nrpn_give_a_descriptor_that_reads() {
    let r = |cc: &str| row(&[("parameter_name", "Cutoff"), ("cc_msb", cc), ("nrpn_msb", "5"), ("nrpn_lsb", "6")]);
    let t = ok(&csv(&[r("1"), r("2")]));
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "the import does not read in 0.6");
}

#[test]
fn rows_claiming_a_message_already_in_the_base_give_a_descriptor_that_reads() {
    let base = ok(&csv(&[row(&[("parameter_name", "Cutoff"), ("cc_msb", "74")])])).value;
    let more = csv(&[row(&[("parameter_name", "Brightness"), ("cc_msb", "74")])]);
    let t = osd_formats::import_into(base, &more, Format::MidiCsv, &source()).unwrap();
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value)));
    assert!(osd::read(&text, osd::Kind::Device).is_ok(), "the import does not read in 0.6");
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
fn a_default_value_of_the_row_becomes_the_default_of_the_control_of_its_address() {
    let r = row(&[
        ("parameter_name", "Both"),
        ("cc_msb", "1"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("cc_default_value", "64"),
        ("nrpn_msb", "2"),
        ("nrpn_lsb", "3"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "16383"),
    ]);
    let nrpn = row(&[("parameter_name", "Depth"), ("nrpn_msb", "4"), ("nrpn_lsb", "5"), ("nrpn_min_value", "0"), ("nrpn_max_value", "200"), ("nrpn_default_value", "10")]);
    let t = ok(&csv(&[glide(), r, nrpn]));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
    let list = entries(&t.value);
    assert_eq!(list.len(), 3);
    assert_eq!((list[0].default, list[1].default, list[2].default), (None, Some(64), Some(10)));
    assert_eq!((list[1].cc, list[2].cc), (Some(1), None));
}

#[test]
fn a_default_without_a_range_is_carried_and_one_equal_to_a_bound_too() {
    let alone = row(&[("parameter_name", "Alone"), ("cc_msb", "1"), ("cc_default_value", "300")]);
    let bound = row(&[("parameter_name", "Bound"), ("cc_msb", "2"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("cc_default_value", "127")]);
    let t = ok(&csv(&[alone, bound]));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
    let list = entries(&t.value);
    assert_eq!((list[0].default, list[1].default), (Some(300), Some(127)));
}

#[test]
fn a_14_bit_default_is_carried_in_the_range_of_the_14_bits() {
    let r = row(&[("parameter_name", "Cutoff"), ("cc_msb", "1"), ("cc_lsb", "33"), ("cc_min_value", "0"), ("cc_max_value", "16383"), ("cc_default_value", "8192")]);
    let t = ok(&csv(&[r]));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
    assert_eq!(entries(&t.value)[0].default, Some(8192));
}

#[test]
fn a_default_outside_the_range_of_its_control_is_not_written_and_stays_in_the_losses_with_its_cell_and_reason() {
    let r = row(&[
        ("parameter_name", "Both"),
        ("cc_msb", "1"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("cc_default_value", "200"),
        ("nrpn_msb", "2"),
        ("nrpn_lsb", "3"),
        ("nrpn_min_value", "10"),
        ("nrpn_max_value", "20"),
        ("nrpn_default_value", "9"),
    ]);
    let t = ok(&csv(&[glide(), r]));
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, ["line 3, column cc_default_value", "line 3, column nrpn_default_value"]);
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
    assert!(entries(&t.value).iter().all(|e| e.default.is_none()));
    // The descriptor stays valid: it reads back strictly.
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
    assert!(osd::read(&text, osd::Kind::Device).is_ok());
}

#[test]
fn a_default_of_an_address_the_row_does_not_have_is_in_the_losses_by_its_cell() {
    let r = row(&[("parameter_name", "Cc only"), ("cc_msb", "1"), ("nrpn_default_value", "10")]);
    let t = ok(&csv(&[r]));
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, ["line 2, column nrpn_default_value"]);
    assert!(!t.lost[0].reason.is_empty());
    assert_eq!(entries(&t.value)[0].default, None);
}

#[test]
fn an_import_without_a_default_cell_gives_a_control_without_a_default() {
    assert_eq!(entries(&ok(&csv(&[glide()])).value)[0].default, None);
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
    let cc = other.iter().position(|c| c == "65").unwrap();
    other[cc] = "66".into();
    let t = ok(&csv(&[glide(), other]));
    assert_eq!(t.value.device.name, "Trundler");
    assert_eq!(t.lost.iter().map(|l| l.place.as_str()).collect::<Vec<_>>(), ["line 3, column device"]);
}

#[test]
fn the_losses_come_in_the_order_of_the_source_and_are_the_same_on_each_import() {
    let a = row(&[("cc_msb", "1"), ("cc_min_value", "0"), ("cc_max_value", "10"), ("cc_default_value", "11")]);
    let none = row(&[("parameter_name", "Nothing")]);
    let b = row(&[("cc_msb", "2"), ("cc_min_value", "0"), ("cc_max_value", "10"), ("cc_default_value", "12"), ("orientation", "weird")]);
    let text = csv(&[a, none, b]);
    let first = ok(&text);
    let places: Vec<&str> = first.lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, ["line 2, column cc_default_value", "line 3", "line 4, column cc_default_value", "line 4, column orientation"]);
    assert_eq!(first.lost, ok(&text).lost);
}

#[test]
fn a_row_whose_only_default_is_in_the_nrpn_cell_and_whose_ranges_differ_has_the_nrpn_as_first_way() {
    let r = row(&[
        ("parameter_name", "Both"),
        ("cc_msb", "1"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("nrpn_msb", "2"),
        ("nrpn_lsb", "3"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "16383"),
        ("nrpn_default_value", "8192"),
    ]);
    let t = ok(&csv(&[glide(), r]));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
    let e = &entries(&t.value)[1];
    assert_eq!((e.cc, e.nrpn_msb, e.nrpn_lsb, e.range, e.default), (None, Some(2), Some(3), Some([0, 16383]), Some(8192)));
    let ways = e.ways();
    assert_eq!((ways.len(), ways[1].cc, ways[1].range), (2, Some(1), Some([0, 127])));
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_of_one_range_or_a_default_in_the_cc_cell_keeps_the_cc_as_first_way() {
    let same = row(&[
        ("parameter_name", "Same"),
        ("cc_msb", "1"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("nrpn_msb", "2"),
        ("nrpn_lsb", "3"),
        ("nrpn_default_value", "64"),
    ]);
    let in_cc = row(&[
        ("parameter_name", "InCc"),
        ("cc_msb", "4"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("cc_default_value", "10"),
        ("nrpn_msb", "5"),
        ("nrpn_lsb", "6"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "16383"),
    ]);
    let list = entries(&ok(&csv(&[glide(), same, in_cc])).value);
    assert_eq!((list[1].cc, list[1].default, list[1].range), (Some(1), Some(64), Some([0, 127])));
    assert_eq!((list[2].cc, list[2].default, list[2].range), (Some(4), Some(10), Some([0, 127])));
}
