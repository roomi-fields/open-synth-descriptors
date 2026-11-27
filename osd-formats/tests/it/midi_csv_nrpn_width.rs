//! The pencilresearch/midi CSV export of an NRPN way of 7 bits: the CSV has no column for the
//! width of a value and the import reads every NRPN on 14 bits, so the export names the width, and
//! a default the import reads on another scale (osd-formats/docs/CADRE.md R8; osd-formats/docs/CADRE.md R16).

use crate::common::{device_from, entries, parse, resolved, source, HEADER};
use osd_formats::{export, import, Format, Loss};
use serde_json::{json, Value};

/// A device of 0.7 of one control of these fields.
fn device(controls: &[Value]) -> osd::Device {
    let entries: Vec<Value> = controls
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let mut c = c.clone();
            c["osc"] = json!(format!("/c{n}"));
            c
        })
        .collect();
    device_from(
        &json!({
            "$schema": "https://w3id.org/osd/0.7/device.schema.json",
            "sources": [{"type": "community-re", "url": "https://github.com/pencilresearch/midi", "license": "CC-BY-SA-4.0"}],
            "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
            "cc_params": {"entries": entries},
        })
        .to_string(),
    )
}

fn with_range(mut way: Value, range: Option<[i64; 2]>) -> Value {
    if let Some(r) = range {
        way["range"] = json!(r);
    }
    way
}

fn cc(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "cc", "ctrlIndex": [74]}), range)
}

fn cc14(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "cc", "ctrlIndex": [5, 37], "value_bits": 14}), range)
}

fn nrpn7(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 7}), range)
}

fn nrpn14(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": 14}), range)
}

/// The rows of the export and its losses.
fn exported(controls: &[Value]) -> (Vec<Vec<String>>, Vec<Loss>) {
    let out = export(&resolved(device(controls)), Format::MidiCsv).unwrap();
    (parse(&out.value), out.lost)
}

fn cell(record: &[String], column: &str) -> String {
    record[HEADER.iter().position(|h| *h == column).unwrap()].clone()
}

/// The losses of a width, in the order of their places.
fn widths(lost: &[Loss]) -> Vec<&Loss> {
    let mut out: Vec<&Loss> = lost.iter().filter(|l| l.place.ends_with("/value_bits")).collect();
    out.sort_by(|a, b| a.place.cmp(&b.place));
    out
}

fn places<'a>(lost: &[&'a Loss]) -> Vec<&'a str> {
    lost.iter().map(|l| l.place.as_str()).collect()
}

/// The reason says what the CSV lacks and how the import reads the NRPN.
fn assert_says_the_width_is_lost(reason: &str, case: &str) {
    let said = reason.to_lowercase();
    for word in ["column", "width", "14 bits", "msb", "lsb"] {
        assert!(said.contains(word), "{case}: {word:?} is not said: {reason}");
    }
}

/// The controls whose row writes no bounds for their 7-bit NRPN way, with the index of that way.
fn without_bounds() -> Vec<(Value, usize)> {
    vec![
        (json!({"ways": [nrpn7(None)]}), 0),
        (json!({"ways": [cc(None), nrpn7(None)]}), 1),
        (json!({"ways": [nrpn7(None), cc(None)]}), 0),
        (json!({"ways": [cc14(None), nrpn7(None)]}), 1),
        (json!({"ways": [nrpn7(None), cc14(None)]}), 0),
        (json!({"ways": [cc(None), nrpn7(None)], "default": 64}), 1),
        (json!({"ways": [nrpn7(None), cc(None)], "default": 64}), 0),
        (json!({"ways": [nrpn7(None)], "default": 64}), 0),
    ]
}

/// The controls whose row writes bounds for their 7-bit NRPN way, its own or the shared range, with
/// the index of that way.
fn with_bounds() -> Vec<(Value, usize)> {
    let mut out = vec![
        (json!({"ways": [nrpn7(Some([0, 100]))]}), 0),
        (json!({"ways": [nrpn7(Some([0, 127]))]}), 0),
        // The CSV writes the scale the NRPN implies beside the range of the CC, so that a default
        // on that scale keeps it.
        (json!({"ways": [cc14(None), nrpn7(None)], "scale": [0, 127], "default": 50}), 1),
        (json!({"ways": [nrpn7(None), cc14(None)], "scale": [0, 127], "default": 50}), 0),
    ];
    let pairs = [
        (cc(Some([0, 100])), nrpn7(None)),
        (cc(Some([0, 127])), nrpn7(None)),
        (cc(None), nrpn7(Some([-5, 5]))),
        (cc(None), nrpn7(Some([0, 127]))),
        (cc(Some([0, 100])), nrpn7(Some([0, 100]))),
        (cc(Some([0, 100])), nrpn7(Some([10, 20]))),
    ];
    for (c, n) in pairs {
        out.push((json!({"ways": [c.clone(), n.clone()]}), 1));
        out.push((json!({"ways": [n, c]}), 0));
    }
    out
}

#[test]
fn a_7_bit_nrpn_way_written_without_bounds_names_its_width_and_the_scale_it_becomes() {
    for (control, way) in without_bounds() {
        let (records, lost) = exported(std::slice::from_ref(&control));
        let case = control.to_string();
        assert_eq!((cell(&records[1], "nrpn_msb"), cell(&records[1], "nrpn_lsb")), ("1".into(), "2".into()), "{case}");
        let named = widths(&lost);
        assert_eq!(places(&named), [format!("/cc_params/entries/0/ways/{way}/value_bits")], "{case}: {lost:?}");
        assert_says_the_width_is_lost(&named[0].reason, &case);
        assert!(named[0].reason.contains("16383"), "{case}: the new scale is not said: {}", named[0].reason);
    }
}

#[test]
fn a_7_bit_nrpn_way_written_with_bounds_names_its_width_and_no_new_scale() {
    for (control, way) in with_bounds() {
        let (_, lost) = exported(std::slice::from_ref(&control));
        let case = control.to_string();
        let named = widths(&lost);
        assert_eq!(places(&named), [format!("/cc_params/entries/0/ways/{way}/value_bits")], "{case}: {lost:?}");
        assert_says_the_width_is_lost(&named[0].reason, &case);
        assert!(!named[0].reason.contains("16383"), "{case}: the scale does not change: {}", named[0].reason);
    }
}

#[test]
fn each_control_with_a_7_bit_nrpn_names_its_width_at_its_own_entry() {
    let (_, lost) = exported(&[
        json!({"ways": [{"ctrlType": "nrpn", "ctrlIndex": [0, 1], "value_bits": 7}]}),
        json!({"ways": [{"ctrlType": "nrpn", "ctrlIndex": [0, 2], "value_bits": 14}]}),
        json!({"ways": [{"ctrlType": "cc", "ctrlIndex": [20]}, {"ctrlType": "nrpn", "ctrlIndex": [0, 3], "value_bits": 7}]}),
    ]);
    assert_eq!(places(&widths(&lost)), ["/cc_params/entries/0/ways/0/value_bits", "/cc_params/entries/2/ways/1/value_bits"], "{lost:?}");
}

#[test]
fn only_the_nrpn_way_the_row_writes_names_its_width_and_a_way_not_written_is_named_whole() {
    let second = json!({"ctrlType": "nrpn", "ctrlIndex": [3, 4], "value_bits": 7});
    let (_, lost) = exported(&[json!({"ways": [nrpn7(None), second]})]);
    let mut of_entry: Vec<&str> = lost.iter().map(|l| l.place.as_str()).filter(|p| p.starts_with("/cc_params")).collect();
    of_entry.sort();
    assert_eq!(of_entry, ["/cc_params/entries/0/ways/0/value_bits", "/cc_params/entries/0/ways/1"], "{lost:?}");

    let rpn = json!({"ctrlType": "rpn", "ctrlIndex": [0, 2], "value_bits": 7});
    let (_, lost) = exported(&[json!({"ways": [rpn, nrpn7(None)]})]);
    assert_eq!(places(&widths(&lost)), ["/cc_params/entries/0/ways/1/value_bits"], "{lost:?}");
}

#[test]
fn a_14_bit_nrpn_way_a_cc_way_and_a_7_bit_nrpn_way_not_written_name_no_width() {
    let second = json!({"ctrlType": "nrpn", "ctrlIndex": [3, 4], "value_bits": 7});
    let controls = [
        json!({"ways": [nrpn14(None)]}),
        json!({"ways": [nrpn14(Some([0, 127]))]}),
        json!({"ways": [cc(None), nrpn14(None)]}),
        json!({"ways": [nrpn14(None), cc14(None)]}),
        json!({"ways": [cc(None)]}),
        json!({"ways": [{"ctrlType": "cc", "ctrlIndex": [74], "value_bits": 7}]}),
        json!({"ways": [cc14(None)]}),
        json!({"ways": [nrpn14(None), second]}),
    ];
    for control in controls {
        let (_, lost) = exported(std::slice::from_ref(&control));
        assert!(widths(&lost).is_empty(), "{control}: {lost:?}");
    }
}

#[test]
fn a_7_bit_nrpn_without_a_range_writes_no_bounds_and_keeps_its_default_cell() {
    let bounds = ["cc_min_value", "cc_max_value", "nrpn_min_value", "nrpn_max_value"];
    for control in [json!({"ways": [nrpn7(None)], "default": 64}), json!({"ways": [cc14(None), nrpn7(None)]})] {
        let (records, _) = exported(std::slice::from_ref(&control));
        assert_eq!(bounds.map(|c| cell(&records[1], c)), [""; 4].map(String::from), "{control}");
        let default = control.get("default").map_or(String::new(), |d| d.to_string());
        assert_eq!((cell(&records[1], "cc_default_value"), cell(&records[1], "nrpn_default_value")), (String::new(), default), "{control}");
    }
}

/// The losses at the default of the first control.
fn of_default(lost: &[Loss]) -> Vec<&Loss> {
    lost.iter().filter(|l| l.place == "/cc_params/entries/0/default").collect()
}

#[test]
fn a_default_of_a_7_bit_nrpn_without_a_range_and_no_cc_is_written_and_named_for_the_scale_it_is_read_on() {
    let pressure = json!({"ctrlType": "chPress"});
    let mut controls = Vec::new();
    for default in [0, 64, 127] {
        controls.push(json!({"ways": [nrpn7(None)], "default": default}));
        controls.push(json!({"ways": [nrpn7(None)], "scale": [0, 127], "default": default}));
        controls.push(json!({"ways": [nrpn7(None), pressure], "default": default}));
    }
    for control in controls {
        let (records, lost) = exported(std::slice::from_ref(&control));
        let default = control["default"].to_string();
        assert_eq!((cell(&records[1], "cc_default_value"), cell(&records[1], "nrpn_default_value")), (String::new(), default), "{control}");
        let named = of_default(&lost);
        assert_eq!(named.len(), 1, "{control}: {lost:?}");
        assert!(named[0].reason.contains("16383") && named[0].reason.contains("127"), "{control}: both scales are said: {}", named[0].reason);
    }
}

#[test]
fn the_default_the_export_names_is_read_back_unchanged_on_the_scale_of_14_bits() {
    let out = export(&resolved(device(&[json!({"ways": [nrpn7(None)], "default": 64})])), Format::MidiCsv).unwrap();
    assert_eq!(of_default(&out.lost).len(), 1, "{:?}", out.lost);
    let back = import(&out.value, Format::MidiCsv, &source()).unwrap();
    let entry = &entries(&back.value)[0];
    assert_eq!((entry.default, entry.reference_scale()), (Some(64), [0, 16383]));
}

#[test]
fn a_default_that_comes_back_on_its_scale_is_not_named() {
    let controls = [
        json!({"ways": [nrpn14(None)], "default": 64}),
        json!({"ways": [nrpn14(None)], "default": 9000}),
        json!({"ways": [nrpn7(Some([0, 127]))], "default": 64}),
        json!({"ways": [nrpn7(Some([0, 100]))], "default": 64}),
        // The default of a 7-bit CC without a range, beside the NRPN, goes to the CC column.
        json!({"ways": [cc(None), nrpn7(None)], "default": 64}),
        json!({"ways": [nrpn7(None), cc(None)], "default": 64}),
        // A row with a CC writes the scales as ranges.
        json!({"ways": [cc(Some([0, 100])), nrpn7(None)], "scale": [0, 127], "default": 120}),
        json!({"ways": [cc14(None), nrpn7(None)], "scale": [0, 127], "default": 50}),
    ];
    for control in controls {
        let (_, lost) = exported(std::slice::from_ref(&control));
        assert!(of_default(&lost).is_empty(), "{control}: {lost:?}");
    }
}
