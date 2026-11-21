//! A CSV imported then exported gives back the same rows.

use crate::common::{csv, parse, resolved, row, source};
use osd_formats::{export, import, Format};

fn base(extra: &[(&'static str, &'static str)]) -> Vec<String> {
    let mut cells = vec![("manufacturer", "Lofty"), ("device", "Trundler")];
    cells.extend_from_slice(extra);
    row(&cells)
}

fn through(text: &str) -> String {
    let imported = import(text, Format::MidiCsv, &source()).expect("imports");
    export(&resolved(imported.value), Format::MidiCsv).expect("exports").value
}

#[test]
fn rows_with_a_cc_alone_or_an_nrpn_alone_come_back_the_same_in_order() {
    let rows = vec![
        base(&[
            ("section", "Oscillators"),
            ("parameter_name", "Glide switch"),
            ("parameter_description", "Enables or disables glide"),
            ("cc_msb", "65"),
            ("cc_min_value", "0"),
            ("cc_max_value", "127"),
            ("orientation", "0-based"),
            ("notes", "Typo in manual (says CC 66; 65 is correct)"),
            ("usage", "0-63: Off; 64-127: On"),
        ]),
        base(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "1"), ("cc_lsb", "33"), ("cc_min_value", "0"), ("cc_max_value", "16383"), ("orientation", "centered")]),
        base(&[("section", "Filter"), ("parameter_name", "Resonance, \"Q\""), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "16383")]),
        base(&[("parameter_name", "Bare"), ("cc_msb", "7")]),
        base(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74")]),
    ];
    assert_eq!(parse(&through(&csv(&rows))), parse(&csv(&rows)));
}

#[test]
fn a_row_with_both_gives_one_row_back_and_a_row_with_neither_is_dropped_into_the_losses() {
    let both = base(&[("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "0"), ("nrpn_max_value", "16383"), ("notes", "n")]);
    let neither = base(&[("parameter_name", "Sysex only")]);
    let alone = base(&[("parameter_name", "Alone"), ("cc_msb", "5")]);
    let text = csv(&[alone.clone(), neither, both.clone()]);
    let imported = import(&text, Format::MidiCsv, &source()).unwrap();
    assert!(imported.lost.iter().any(|l| l.place == "line 3"));
    let out = parse(&export(&resolved(imported.value), Format::MidiCsv).unwrap().value);
    let one_row = row(&[
        ("manufacturer", "Lofty"),
        ("device", "Trundler"),
        ("section", "Filter"),
        ("parameter_name", "Cutoff"),
        ("cc_msb", "74"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("nrpn_msb", "1"),
        ("nrpn_lsb", "2"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "16383"),
        ("notes", "n"),
    ]);
    assert_eq!(out[1..].to_vec(), vec![alone, one_row]);
}

#[test]
fn the_default_columns_come_back_in_the_same_cell_after_an_import_and_an_export() {
    let rows = vec![
        base(&[("parameter_name", "Cc"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("cc_default_value", "64")]),
        base(&[("parameter_name", "Wide"), ("cc_msb", "1"), ("cc_lsb", "33"), ("cc_min_value", "0"), ("cc_max_value", "16383"), ("cc_default_value", "8192"), ("orientation", "centered")]),
        base(&[("parameter_name", "Nrpn"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_min_value", "10"), ("nrpn_max_value", "200"), ("nrpn_default_value", "10")]),
        base(&[("parameter_name", "No range"), ("cc_msb", "7"), ("cc_default_value", "0")]),
        base(&[("parameter_name", "No range nrpn"), ("nrpn_msb", "7"), ("nrpn_lsb", "8"), ("nrpn_default_value", "300")]),
    ];
    assert_eq!(parse(&through(&csv(&rows))), parse(&csv(&rows)));
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_gives_its_cc_default_back_in_the_cc_cell() {
    let both = base(&[("parameter_name", "Both"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("cc_default_value", "64"), ("nrpn_msb", "1"), ("nrpn_lsb", "2")]);
    let out = parse(&through(&csv(&[both])));
    let at = |name: &str| crate::common::HEADER.iter().position(|h| *h == name).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[1][at("cc_default_value")], "64");
    assert_eq!(out[1][at("nrpn_default_value")], "");
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_gives_one_row_back_and_loses_nothing() {
    let both = base(&[
        ("section", "Filter"),
        ("parameter_name", "Cutoff"),
        ("parameter_description", "d"),
        ("cc_msb", "74"),
        ("cc_min_value", "0"),
        ("cc_max_value", "127"),
        ("nrpn_msb", "1"),
        ("nrpn_lsb", "2"),
        ("nrpn_min_value", "0"),
        ("nrpn_max_value", "16383"),
        ("orientation", "centered"),
        ("notes", "n"),
        ("usage", "u"),
    ]);
    let alone = base(&[("parameter_name", "Alone"), ("cc_msb", "5")]);
    let mut last = alone.clone();
    last[base(&[("parameter_name", "Alone"), ("cc_msb", "5")]).iter().position(|c| c == "5").unwrap()] = "6".into();
    let text = csv(&[alone, both, last]);
    let imported = import(&text, Format::MidiCsv, &source()).unwrap();
    assert!(imported.lost.is_empty(), "{:?}", imported.lost);
    let out = export(&resolved(imported.value), Format::MidiCsv).unwrap();
    assert_eq!(parse(&out.value), parse(&text));
    assert!(out.lost.iter().all(|l| !l.place.starts_with("/cc_params")), "{:?}", out.lost);
}

#[test]
fn a_row_with_a_cc_and_an_nrpn_and_one_range_comes_back_with_that_range_in_the_cc_cells_only() {
    let one_range = base(&[("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127"), ("nrpn_msb", "1"), ("nrpn_lsb", "2")]);
    assert_eq!(parse(&through(&csv(std::slice::from_ref(&one_range)))), parse(&csv(&[one_range])));
}

#[test]
fn a_14_bit_cc_and_an_nrpn_in_one_row_come_back_in_one_row() {
    let r = base(&[("parameter_name", "Cutoff"), ("cc_msb", "1"), ("cc_lsb", "33"), ("cc_min_value", "0"), ("cc_max_value", "16383"), ("nrpn_msb", "1"), ("nrpn_lsb", "2")]);
    assert_eq!(parse(&through(&csv(std::slice::from_ref(&r)))), parse(&csv(&[r])));
}

#[test]
fn a_control_with_a_cc_and_an_nrpn_way_comes_back_as_the_same_control_after_the_csv() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{"type": "community-re", "url": "https://github.com/pencilresearch/midi", "license": "CC-BY-SA-4.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": [
            {"osc": "/filter/cutoff", "cc": 74, "range": [0, 127], "section": "Filter",
             "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]}]}]}}"#;
    let device = crate::common::device_from(text);
    let csv_text = export(&resolved(device.clone()), Format::MidiCsv).unwrap().value;
    let back = import(&csv_text, Format::MidiCsv, &source()).unwrap();
    let (before, after) = (crate::common::entries(&device), crate::common::entries(&back.value));
    assert_eq!(after.len(), 1);
    let (a, b) = (before[0].ways(), after[0].ways());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((x.cc, x.cc_lsb, x.nrpn_msb, x.nrpn_lsb, x.range), (y.cc, y.cc_lsb, y.nrpn_msb, y.nrpn_lsb, y.range));
    }
}

fn default_after_the_csv(ways: &str, default: i64) -> Option<i64> {
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{{"type": "community-re", "url": "https://github.com/pencilresearch/midi", "license": "CC-BY-SA-4.0"}}],
        "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}},
        "cc_params": {{"entries": [{{"osc": "/a", "default": {default}, {ways}}}]}}}}"#
    );
    let device = crate::common::device_from(&text);
    let csv_text = export(&resolved(device), Format::MidiCsv).unwrap().value;
    let back = import(&csv_text, Format::MidiCsv, &source()).unwrap();
    crate::common::entries(&back.value)[0].default
}

#[test]
fn the_default_of_a_control_whose_first_way_is_an_nrpn_comes_back_after_the_csv() {
    // Different ranges: the NRPN stays first and the default is in its range.
    let different = r#""nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383], "also": [{"cc": 74, "range": [0, 127]}]"#;
    assert_eq!(default_after_the_csv(different, 8192), Some(8192));
    // Same range: the CC comes first after the CSV, the default is the same.
    let same = r#""nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 127], "also": [{"cc": 74, "range": [0, 127]}]"#;
    assert_eq!(default_after_the_csv(same, 64), Some(64));
}

/// The export and the import of a one-control device whose ways are `ways` (and, when given, its
/// default): the way order after the import (true when the NRPN is first), and the export's losses.
fn order_after_the_csv(ways: &str, default: Option<i64>) -> (bool, Vec<osd_formats::Loss>) {
    let default = default.map_or(String::new(), |v| format!(r#""default": {v}, "#));
    let text = format!(
        r#"{{"$schema": "https://w3id.org/osd/0.6/device.schema.json",
        "_sources": [{{"type": "community-re", "url": "https://github.com/pencilresearch/midi", "license": "CC-BY-SA-4.0"}}],
        "device": {{"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}},
        "cc_params": {{"entries": [{{"osc": "/a", {default}{ways}}}]}}}}"#
    );
    let out = export(&resolved(crate::common::device_from(&text)), Format::MidiCsv).unwrap();
    let back = import(&out.value, Format::MidiCsv, &source()).unwrap();
    let first = &crate::common::entries(&back.value)[0].ways()[0];
    (first.nrpn_msb.is_some(), out.lost.into_iter().filter(|l| l.place == "/cc_params/entries/0").collect())
}

const NRPN_THEN_CC_SAME_RANGE: &str = r#""nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 127], "also": [{"cc": 74, "range": [0, 127]}]"#;
const NRPN_THEN_CC_OTHER_RANGE: &str = r#""nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383], "also": [{"cc": 74, "range": [0, 127]}]"#;
const CC_THEN_NRPN: &str = r#""cc": 74, "range": [0, 127], "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "range": [0, 16383]}]"#;

#[test]
fn an_nrpn_then_cc_control_of_equal_ranges_names_the_lost_order_and_comes_back_cc_first() {
    for default in [None, Some(64)] {
        let (nrpn_first, lost) = order_after_the_csv(NRPN_THEN_CC_SAME_RANGE, default);
        assert!(!nrpn_first, "{default:?}");
        assert_eq!(lost.len(), 1, "{default:?}: {lost:?}");
        assert!(lost[0].reason.contains("order"), "{}", lost[0].reason);
    }
}

#[test]
fn an_nrpn_then_cc_control_of_different_ranges_without_default_names_the_lost_order() {
    let (nrpn_first, lost) = order_after_the_csv(NRPN_THEN_CC_OTHER_RANGE, None);
    assert!(!nrpn_first);
    assert_eq!(lost.len(), 1, "{lost:?}");
    assert!(lost[0].reason.contains("order"), "{}", lost[0].reason);
}

#[test]
fn an_nrpn_then_cc_control_of_different_ranges_with_a_default_keeps_its_order_and_names_no_loss_of_order() {
    let (nrpn_first, lost) = order_after_the_csv(NRPN_THEN_CC_OTHER_RANGE, Some(8192));
    assert!(nrpn_first);
    assert!(lost.is_empty(), "{lost:?}");
}

#[test]
fn a_cc_then_nrpn_control_keeps_its_order_and_names_no_loss_of_order() {
    for default in [None, Some(64)] {
        let (nrpn_first, lost) = order_after_the_csv(CC_THEN_NRPN, default);
        assert!(!nrpn_first, "{default:?}");
        assert!(lost.is_empty(), "{default:?}: {lost:?}");
    }
}

#[test]
fn the_conformance_case_of_an_nrpn_then_cc_control_comes_back_in_order_through_the_csv_or_names_the_lost_order() {
    let devices = crate::common::credited_devices();
    let (_, d) = devices.iter().find(|(name, _)| name == "0.6/control-ways-nrpn-first").expect("the conformance case 0.6/control-ways-nrpn-first exists and names a source");
    let out = export(d, Format::MidiCsv).unwrap();
    let back = import(&out.value, Format::MidiCsv, &source()).unwrap();
    let before = crate::common::entries(&d.device);
    let after = crate::common::entries(&back.value);
    assert_eq!(before.len(), after.len());
    for (n, (b, a)) in before.iter().zip(&after).enumerate() {
        let order = |e: &osd::CcEntry| e.ways().iter().map(|w| w.cc.is_some()).collect::<Vec<_>>();
        let named = out.lost.iter().any(|l| l.place == format!("/cc_params/entries/{n}"));
        assert!(order(b) == order(a) || named, "entry {n}: the order is lost and not named: {:?}", out.lost);
    }
    assert!(before[0].nrpn_msb.is_some() && before[0].also.as_ref().is_some_and(|w| w[0].cc.is_some()), "the case is NRPN first, CC second");
}
