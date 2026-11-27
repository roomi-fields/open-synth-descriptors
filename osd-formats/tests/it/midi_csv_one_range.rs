//! The pencilresearch/midi CSV export of a control with a CC way and an NRPN way of which one
//! alone has a range: the import gives the one range of a row to both ways, so the row holds the
//! scale of each, and each way comes back on its own (osd-formats/docs/CADRE.md R8,
//! osd-formats/docs/CADRE.md R16).

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
            c["title"] = json!(format!("c{n}"));
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

fn cc(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "cc", "ctrlIndex": [74]}), range)
}

fn cc14(range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "cc", "ctrlIndex": [5, 37], "value_bits": 14}), range)
}

fn nrpn(bits: i64, range: Option<[i64; 2]>) -> Value {
    with_range(json!({"ctrlType": "nrpn", "ctrlIndex": [1, 2], "value_bits": bits}), range)
}

fn with_range(mut way: Value, range: Option<[i64; 2]>) -> Value {
    if let Some(r) = range {
        way["range"] = json!(r);
    }
    way
}

/// A CC way and an NRPN way, one alone with a range, with the scales the row is to hold: the CC's,
/// then the NRPN's.
fn one_range_of_another_scale() -> Vec<(Value, Value, [i64; 2], [i64; 2])> {
    vec![
        (cc(Some([0, 100])), nrpn(14, None), [0, 100], [0, 16383]),
        (cc(Some([0, 100])), nrpn(7, None), [0, 100], [0, 127]),
        (cc14(Some([10, 200])), nrpn(14, None), [10, 200], [0, 16383]),
        (cc(None), nrpn(14, Some([0, 100])), [0, 127], [0, 100]),
        (cc(None), nrpn(14, Some([0, 16383])), [0, 127], [0, 16383]),
        (cc14(None), nrpn(14, Some([10, 200])), [0, 16383], [10, 200]),
        (cc(None), nrpn(7, Some([-5, 5])), [0, 127], [-5, 5]),
    ]
}

/// The cells of the bounds of a row: the CC's minimum and maximum, then the NRPN's.
fn bounds_of(record: &[String]) -> [String; 4] {
    ["cc_min_value", "cc_max_value", "nrpn_min_value", "nrpn_max_value"].map(|c| cell(record, c))
}

fn cell(record: &[String], column: &str) -> String {
    record[HEADER.iter().position(|h| *h == column).unwrap()].clone()
}

/// The device exported, its rows, the losses of its controls but those of a range the import
/// adds (export_names_every_loss.rs and osd_field_tables.rs hold them), and the device imported back.
fn round_trip(d: &osd::Device) -> (Vec<Vec<String>>, Vec<Loss>, osd::Device) {
    let out = export(&resolved(d.clone()), Format::MidiCsv).unwrap();
    let back = import(&out.value, Format::MidiCsv, &source()).unwrap();
    assert!(back.lost.is_empty(), "{:?}", back.lost);
    let lost = out.lost.into_iter().filter(|l| l.place.starts_with("/cc_params") && !l.place.ends_with("/range")).collect();
    (parse(&out.value), lost, back.value)
}

/// The places of the losses of the width of the ways: an NRPN of 7 bits, which the CSV does not
/// say and the import reads on 14.
fn width_places(first: &Value, second: &Value) -> Vec<String> {
    [first, second]
        .iter()
        .enumerate()
        .filter(|(_, w)| w["ctrlType"] == "nrpn" && w["value_bits"] == 7)
        .map(|(m, _)| format!("/cc_params/entries/0/ways/{m}/value_bits"))
        .collect()
}

/// The places of the losses, sorted.
fn sorted_places(lost: &[Loss]) -> Vec<String> {
    let mut places: Vec<String> = lost.iter().map(|l| l.place.clone()).collect();
    places.sort();
    places
}

/// Each way of `after` has the scale it had in `before`, carried as its range.
fn assert_each_way_keeps_its_scale(before: &osd::CcEntry, after: &osd::CcEntry, case: &str) {
    assert_eq!(after.ways().len(), 2, "{case}");
    for way in after.ways() {
        let was = before.ways().into_iter().find(|w| w.ctrl_type == way.ctrl_type).unwrap();
        assert_eq!(way.ctrl_index, was.ctrl_index, "{case}");
        assert_eq!(way.range, Some(was.scale()), "{case}: {way:?}");
    }
}

#[test]
fn a_range_on_one_way_alone_is_written_with_the_scale_the_message_of_the_other_implies() {
    for (c, n, cc_scale, nrpn_scale) in one_range_of_another_scale() {
        for ways in [json!([c, n]), json!([n, c])] {
            let (records, _, _) = round_trip(&device(&[json!({"ways": ways})]));
            let expected = [cc_scale[0], cc_scale[1], nrpn_scale[0], nrpn_scale[1]].map(|v| v.to_string());
            assert_eq!(bounds_of(&records[1]), expected, "{ways}");
        }
    }
}

#[test]
fn each_way_comes_back_on_its_own_scale_when_one_alone_has_a_range() {
    for (c, n, _, _) in one_range_of_another_scale() {
        for ways in [json!([c, n]), json!([n, c])] {
            let d = device(&[json!({"ways": ways})]);
            let (_, _, back) = round_trip(&d);
            assert_each_way_keeps_its_scale(&entries(&d)[0], &entries(&back)[0], &ways.to_string());
        }
    }
}

#[test]
fn a_range_on_one_way_alone_without_a_default_loses_no_range_and_names_the_order_only_when_it_changes() {
    for (c, n, _, _) in one_range_of_another_scale() {
        // Without a default the import puts the CC first.
        let (_, lost, back) = round_trip(&device(&[json!({"ways": [c, n]})]));
        assert_eq!(sorted_places(&lost), width_places(&c, &n), "{c} {n}: {lost:?}");
        assert_eq!(entries(&back)[0].ways()[0].ctrl_type, "cc");
        let (_, lost, _) = round_trip(&device(&[json!({"ways": [n, c]})]));
        let mut expected = vec!["/cc_params/entries/0".to_string()];
        expected.extend(width_places(&n, &c));
        assert_eq!(sorted_places(&lost), expected, "{n} {c}: {lost:?}");
        let order = lost.iter().find(|l| l.place == "/cc_params/entries/0").unwrap();
        assert!(order.reason.contains("order") && order.reason.contains("CC first"), "{}", order.reason);
    }
}

#[test]
fn a_range_equal_to_the_scale_the_other_message_implies_is_written_once_and_both_ways_keep_it() {
    let cases = [
        (cc(Some([0, 127])), nrpn(7, None)),
        (cc(None), nrpn(7, Some([0, 127]))),
        (cc14(Some([0, 16383])), nrpn(14, None)),
        (cc14(None), nrpn(14, Some([0, 16383]))),
    ];
    for (c, n) in cases {
        for ways in [json!([c, n]), json!([n, c])] {
            let d = device(&[json!({"ways": ways})]);
            let (records, _, back) = round_trip(&d);
            let bounds = bounds_of(&records[1]);
            let pairs = bounds.chunks(2).filter(|p| p.iter().any(|v| !v.is_empty())).count();
            assert_eq!(pairs, 1, "{ways}: {bounds:?}");
            assert_each_way_keeps_its_scale(&entries(&d)[0], &entries(&back)[0], &ways.to_string());
        }
    }
}

/// The two ways of a control, the `scale` it writes, the kind of the way that gives the reference,
/// and a default on that scale.
type DefaultCase = (Value, Value, Option<[i64; 2]>, &'static str, i64);

/// Controls of one range whose default is on the scale of the way that gives the reference.
fn defaults_beside_one_range() -> Vec<DefaultCase> {
    vec![
        // The widest way gives the reference.
        (cc(Some([0, 100])), nrpn(14, None), None, "nrpn", 0),
        (cc(Some([0, 100])), nrpn(14, None), None, "nrpn", 50),
        (cc(Some([0, 100])), nrpn(14, None), None, "nrpn", 8192),
        (cc(Some([0, 100])), nrpn(7, None), None, "nrpn", 120),
        (cc(None), nrpn(14, Some([0, 100])), None, "cc", 120),
        (cc(None), nrpn(14, Some([0, 100])), None, "cc", 50),
        (cc(None), nrpn(14, Some([10, 200])), None, "nrpn", 150),
        (cc14(None), nrpn(14, Some([10, 200])), None, "cc", 9000),
        (cc14(Some([10, 200])), nrpn(14, None), None, "nrpn", 100),
        // The `scale` of the control names the narrower way.
        (cc(Some([0, 100])), nrpn(14, None), Some([0, 100]), "cc", 50),
        (cc(None), nrpn(14, Some([0, 100])), Some([0, 100]), "nrpn", 50),
    ]
}

fn control(first: &Value, second: &Value, scale: Option<[i64; 2]>, default: i64) -> Value {
    let mut c = json!({"ways": [first, second], "default": default});
    if let Some(s) = scale {
        c["scale"] = json!(s);
    }
    c
}

#[test]
fn a_default_beside_one_range_goes_to_the_column_of_the_reference_way_and_comes_back_on_its_scale() {
    for (c, n, scale, reference, default) in defaults_beside_one_range() {
        for (first, second) in [(&c, &n), (&n, &c)] {
            let d = device(&[control(first, second, scale, default)]);
            let case = format!("{first} {second} {scale:?} {default}");
            let (records, _, back) = round_trip(&d);
            let (written, empty) = if reference == "cc" { ("cc_default_value", "nrpn_default_value") } else { ("nrpn_default_value", "cc_default_value") };
            assert_eq!((cell(&records[1], written), cell(&records[1], empty)), (default.to_string(), String::new()), "{case}");
            let (before, after) = (&entries(&d)[0], &entries(&back)[0]);
            assert_eq!(after.default, Some(default), "{case}");
            assert_eq!(after.reference_scale(), before.reference_scale(), "{case}");
            assert_eq!(after.ways()[0].ctrl_type, reference, "{case}");
            assert_each_way_keeps_its_scale(before, after, &case);
        }
    }
}

#[test]
fn a_default_beside_one_range_names_the_lost_order_exactly_when_the_import_changes_it() {
    for (c, n, scale, reference, default) in defaults_beside_one_range() {
        for (first, second) in [(&c, &n), (&n, &c)] {
            let case = format!("{first} {second} {scale:?} {default}");
            let (_, lost, back) = round_trip(&device(&[control(first, second, scale, default)]));
            let after = entries(&back)[0].ways()[0].ctrl_type.clone();
            let widths = width_places(first, second);
            if first["ctrlType"] == after.as_str() {
                assert_eq!(sorted_places(&lost), widths, "{case}: {lost:?}");
            } else {
                let mut expected = vec!["/cc_params/entries/0".to_string()];
                expected.extend(widths);
                assert_eq!(sorted_places(&lost), expected, "{case}: {lost:?}");
                let said = if reference == "cc" { "CC first" } else { "NRPN first" };
                let order = lost.iter().find(|l| l.place == "/cc_params/entries/0").unwrap();
                assert!(order.reason.contains("order") && order.reason.contains(said), "{case}: {}", order.reason);
            }
        }
    }
}
