//! A CSV imported then exported gives back the same rows.

use crate::common::{csv, parse, resolved, row, source};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

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
             "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383]}]}]}}"#;
    let device = crate::common::device_from(text);
    let csv_text = export(&resolved(device.clone()), Format::MidiCsv).unwrap().value;
    let back = import(&csv_text, Format::MidiCsv, &source()).unwrap();
    let (before, after) = (crate::common::entries(&device), crate::common::entries(&back.value));
    assert_eq!(after.len(), 1);
    let (a, b) = (before[0].ways(), after[0].ways());
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((&x.ctrl_type, &x.ctrl_index, x.range), (&y.ctrl_type, &y.ctrl_index, y.range));
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
    let different = r#""nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383], "also": [{"cc": 74, "range": [0, 127]}]"#;
    assert_eq!(default_after_the_csv(different, 8192), Some(8192));
    // Same range: the CC comes first after the CSV, the default is the same.
    let same = r#""nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 127], "also": [{"cc": 74, "range": [0, 127]}]"#;
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
    (first.ctrl_type == "nrpn", out.lost.into_iter().filter(|l| l.place == "/cc_params/entries/0").collect())
}

const NRPN_THEN_CC_SAME_RANGE: &str = r#""nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 127], "also": [{"cc": 74, "range": [0, 127]}]"#;
const NRPN_THEN_CC_OTHER_RANGE: &str = r#""nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383], "also": [{"cc": 74, "range": [0, 127]}]"#;
const CC_THEN_NRPN: &str = r#""cc": 74, "range": [0, 127], "also": [{"nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "range": [0, 16383]}]"#;

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

/// A 14-bit CC and an NRPN without a range have one scale: only the column of the default says
/// which way is the first.
const NRPN_THEN_14_BIT_CC_NO_RANGE: &str = r#""nrpn_msb": 1, "nrpn_lsb": 2, "value_bits": 14, "also": [{"cc": 5, "cc_lsb": 37}]"#;

#[test]
fn a_row_with_a_14_bit_cc_an_nrpn_no_range_and_an_nrpn_default_above_127_comes_back_the_same() {
    for default in ["128", "8192", "16383"] {
        let r = base(&[("parameter_name", "Cutoff"), ("cc_msb", "5"), ("cc_lsb", "37"), ("nrpn_msb", "1"), ("nrpn_lsb", "2"), ("nrpn_default_value", default)]);
        assert_eq!(parse(&through(&csv(std::slice::from_ref(&r)))), parse(&csv(&[r])), "{default}");
    }
}

#[test]
fn an_nrpn_then_14_bit_cc_control_without_a_range_with_a_default_above_127_keeps_its_order_and_names_no_loss_of_order() {
    for default in [128, 8192] {
        let (nrpn_first, lost) = order_after_the_csv(NRPN_THEN_14_BIT_CC_NO_RANGE, Some(default));
        assert!(nrpn_first, "{default}");
        assert!(lost.is_empty(), "{default}: {lost:?}");
        assert_eq!(default_after_the_csv(NRPN_THEN_14_BIT_CC_NO_RANGE, default), Some(default));
    }
}

#[test]
fn an_nrpn_then_14_bit_cc_control_without_a_range_with_a_default_up_to_127_names_the_lost_order() {
    for default in [64, 127] {
        let (nrpn_first, lost) = order_after_the_csv(NRPN_THEN_14_BIT_CC_NO_RANGE, Some(default));
        assert!(!nrpn_first, "{default}");
        assert_eq!(lost.len(), 1, "{default}: {lost:?}");
        assert!(lost[0].reason.contains("order"), "{}", lost[0].reason);
        assert_eq!(default_after_the_csv(NRPN_THEN_14_BIT_CC_NO_RANGE, default), Some(default));
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
        let order = |e: &osd::CcEntry| e.ways().iter().map(|w| w.ctrl_type == "cc").collect::<Vec<_>>();
        let named = out.lost.iter().any(|l| l.place == format!("/cc_params/entries/{n}"));
        assert!(order(b) == order(a) || named, "entry {n}: the order is lost and not named: {:?}", out.lost);
    }
    let kinds: Vec<String> = before[0].ways().iter().map(|w| w.ctrl_type.clone()).collect();
    assert_eq!(&kinds[..2], ["nrpn", "cc"], "the case is NRPN first, CC second");
}

/// A device of 0.7 of one control per `(ways, default)`, none of whose ways has a `range`.
fn device_of_controls(controls: &[(Value, i64)]) -> osd::Device {
    let entries: Vec<Value> = controls.iter().enumerate().map(|(n, (ways, default))| json!({"osc": format!("/c{n}"), "ways": ways, "default": default})).collect();
    crate::common::device_from(
        &json!({
            "$schema": "https://w3id.org/osd/0.7/device.schema.json",
            "sources": [{"type": "community-re", "url": "https://github.com/pencilresearch/midi", "license": "CC-BY-SA-4.0"}],
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
            "cc_params": {"entries": entries},
        })
        .to_string(),
    )
}

/// An NRPN and a 7-bit CC, `wide` saying whether the CC writes its width, neither with a `range`.
fn nrpn_and_cc(number: i64, wide: bool) -> (Value, Value) {
    let nrpn = json!({"ctrlType": "nrpn", "ctrlIndex": [1, number], "value_bits": 14});
    let mut cc = json!({"ctrlType": "cc", "ctrlIndex": [number + 20]});
    if wide {
        cc["value_bits"] = json!(7);
    }
    (nrpn, cc)
}

/// The controls of the cases: each order of the two ways, and defaults up to 127.
fn nrpn_beside_a_7_bit_cc(nrpn_first: bool) -> Vec<(Value, i64)> {
    [(0, false), (64, true), (100, false), (127, true)]
        .into_iter()
        .enumerate()
        .map(|(n, (default, wide))| {
            let (nrpn, cc) = nrpn_and_cc(n as i64, wide);
            (if nrpn_first { json!([nrpn, cc]) } else { json!([cc, nrpn]) }, default)
        })
        .collect()
}

#[test]
fn a_default_up_to_127_on_the_scale_of_an_nrpn_beside_a_7_bit_cc_is_written_with_the_range_of_each_way() {
    for nrpn_first in [true, false] {
        let controls = nrpn_beside_a_7_bit_cc(nrpn_first);
        let out = export(&resolved(device_of_controls(&controls)), Format::MidiCsv).unwrap();
        let records = parse(&out.value);
        assert_eq!(records.len(), controls.len() + 1);
        let cell = |r: usize, column: &str| records[r][crate::common::HEADER.iter().position(|h| *h == column).unwrap()].clone();
        for (n, (_, default)) in controls.iter().enumerate() {
            let r = n + 1;
            assert_eq!((cell(r, "cc_min_value"), cell(r, "cc_max_value")), ("0".into(), "127".into()), "{nrpn_first} {default}");
            assert_eq!((cell(r, "nrpn_min_value"), cell(r, "nrpn_max_value")), ("0".into(), "16383".into()), "{nrpn_first} {default}");
            assert_eq!((cell(r, "cc_default_value"), cell(r, "nrpn_default_value")), (String::new(), default.to_string()), "{nrpn_first}");
        }
    }
}

#[test]
fn an_nrpn_beside_a_7_bit_cc_without_a_range_comes_back_nrpn_first_with_its_scales_as_ranges_and_its_default() {
    for nrpn_first in [true, false] {
        let controls = nrpn_beside_a_7_bit_cc(nrpn_first);
        let device = device_of_controls(&controls);
        let out = export(&resolved(device.clone()), Format::MidiCsv).unwrap();
        let back = import(&out.value, Format::MidiCsv, &source()).unwrap();
        assert!(back.lost.is_empty(), "{:?}", back.lost);
        let (before, after) = (crate::common::entries(&device), crate::common::entries(&back.value));
        assert_eq!(after.len(), before.len());
        for (b, a) in before.iter().zip(&after) {
            let ways = a.ways();
            let kinds: Vec<&str> = ways.iter().map(|w| w.ctrl_type.as_str()).collect();
            assert_eq!(kinds, ["nrpn", "cc"], "{nrpn_first}: {}", b.osc);
            for way in &ways {
                let was = b.ways().into_iter().find(|w| w.ctrl_type == way.ctrl_type).unwrap();
                assert_eq!(way.ctrl_index, was.ctrl_index);
                assert_eq!(way.scale(), was.scale(), "{nrpn_first}: {way:?}");
                assert_eq!(way.range, Some(was.scale()), "{nrpn_first}: {way:?}");
            }
            assert_eq!(a.default, b.default, "{nrpn_first}");
            assert_eq!(a.reference_scale(), [0, 16383], "{nrpn_first}");
        }
        // The ranges the import carries change nothing in the rows.
        let again = export(&resolved(back.value), Format::MidiCsv).unwrap();
        assert_eq!(parse(&again.value), parse(&out.value), "{nrpn_first}");
    }
}

#[test]
fn an_nrpn_then_7_bit_cc_control_without_a_range_with_a_default_up_to_127_loses_no_order_or_default() {
    let controls = nrpn_beside_a_7_bit_cc(true);
    let out = export(&resolved(device_of_controls(&controls)), Format::MidiCsv).unwrap();
    // The losses at a control as a whole (the order of its ways) and at its default.
    let of_controls: Vec<&osd_formats::Loss> =
        out.lost.iter().filter(|l| l.place.starts_with("/cc_params") && (l.place.matches('/').count() == 3 || l.place.ends_with("/default"))).collect();
    assert!(of_controls.is_empty(), "{of_controls:?}");
}

#[test]
fn a_cc_then_nrpn_control_without_a_range_with_a_default_up_to_127_on_the_nrpn_names_that_the_import_puts_the_nrpn_first() {
    let controls = nrpn_beside_a_7_bit_cc(false);
    let out = export(&resolved(device_of_controls(&controls)), Format::MidiCsv).unwrap();
    let back = import(&out.value, Format::MidiCsv, &source()).unwrap();
    // The losses at a control as a whole.
    let of_controls: Vec<&osd_formats::Loss> = out.lost.iter().filter(|l| l.place.starts_with("/cc_params") && l.place.matches('/').count() == 3).collect();
    let places: Vec<&str> = of_controls.iter().map(|l| l.place.as_str()).collect();
    let expected: Vec<String> = (0..controls.len()).map(|n| format!("/cc_params/entries/{n}")).collect();
    assert_eq!(places, expected);
    for (loss, entry) in of_controls.iter().zip(crate::common::entries(&back.value)) {
        assert!(loss.reason.contains("order") && loss.reason.contains("NRPN first"), "{}", loss.reason);
        assert_eq!(entry.ways()[0].ctrl_type, "nrpn", "the loss says what the import does");
    }
}
