//! A base of 0.4 or 0.5 whose control gives both a CC and an NRPN keeps both messages through
//! `import_into`: the base is converted to 0.7 by the chain of `osd`, the control stays one entry
//! with a CC way then an NRPN way, its messages stay claimed, an NRPN given by one number goes to
//! the note `_incomplete` and claims nothing, and a mix 0.7 cannot say is refused.

use crate::common::{csv, device_from, row, source, trigger_csv, trigger_row};
use osd_formats::{import_into, Format, Translated};
use osd::{Kind, Version};
use serde_json::{json, Value};

const SCHEMA_0_5: &str = "https://w3id.org/osd/0.5/device.schema.json";
const VERSIONS: [Version; 2] = [Version::V0_4, Version::V0_5];

/// The control that mixes a CC and an NRPN.
fn mixed() -> Value {
    json!({"osc": "/a", "cc": 1, "nrpn_msb": 1, "nrpn_lsb": 2})
}

fn cc_way(number: u8) -> Value {
    json!({"ctrlType": "cc", "ctrlIndex": [number]})
}

fn nrpn_way(msb: u8, lsb: u8) -> Value {
    json!({"ctrlType": "nrpn", "ctrlIndex": [msb, lsb], "value_bits": 14})
}

/// The mixed control as 0.7 says it.
fn mixed_in_0_7() -> Value {
    json!({"osc": "/a", "ways": [cc_way(1), nrpn_way(1, 2)]})
}

/// A device of `version` with these controls; a 0.4 text states no `$schema`.
fn text_of_base(version: Version, entries: &[Value]) -> Value {
    let mut doc = json!({
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
        "cc_params": {"entries": entries},
    });
    if version == Version::V0_5 {
        doc["$schema"] = json!(SCHEMA_0_5);
    }
    doc
}

fn base(version: Version, entries: &[Value]) -> osd::Device {
    device_from(&text_of_base(version, entries).to_string())
}

fn written(d: &osd::Device) -> Value {
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(d.clone())))).unwrap()
}

/// A controller file of one row with these cells.
fn controllers(cells: &[(&str, &str)]) -> String {
    let mut all = vec![("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Resonance")];
    all.extend_from_slice(cells);
    csv(&[row(&all)])
}

/// A file of each CSV format that gives none of the messages of the mixed control.
fn files() -> [(String, Format); 2] {
    let drums = trigger_csv(&[trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])]);
    [(controllers(&[("cc_msb", "74")]), Format::MidiCsv), (drums, Format::MidiTriggersCsv)]
}

fn joined(base: osd::Device, text: &str, format: Format) -> Translated<osd::Device> {
    import_into(base, text, format, &source()).unwrap_or_else(|f| panic!("{format:?}: {f}"))
}

/// The controls of the result, once the reader of 0.7 has read its text.
fn controls_read_in_0_7(d: &osd::Device) -> Vec<Value> {
    let doc = written(d);
    osd::read_as(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("the reader refuses the result: {f}\n{doc}"));
    doc["cc_params"]["entries"].as_array().unwrap().clone()
}

#[test]
fn a_mixed_control_of_a_base_of_0_4_or_0_5_reaches_0_7_as_a_cc_way_then_an_nrpn_way_with_each_csv() {
    for version in VERSIONS {
        for (text, format) in files() {
            let out = joined(base(version, &[mixed()]), &text, format);
            let controls = controls_read_in_0_7(&out.value);
            assert_eq!(controls[0], mixed_in_0_7(), "{version:?} {format:?}");
            // The controller file adds its one control; the triggers file adds none.
            let expected = if format == Format::MidiCsv { 2 } else { 1 };
            assert_eq!(controls.len(), expected, "{version:?} {format:?}: {controls:?}");
            assert!(out.lost.is_empty(), "{version:?} {format:?}: {:?}", out.lost);
        }
    }
}

#[test]
fn each_mixed_control_of_a_base_keeps_its_own_two_ways_wherever_it_stands() {
    let entries = [
        json!({"osc": "/plain", "cc": 7}),
        mixed(),
        json!({"osc": "/zero", "cc": 0, "nrpn_msb": 0, "nrpn_lsb": 0}),
    ];
    for version in VERSIONS {
        for (text, format) in files() {
            let out = joined(base(version, &entries), &text, format);
            let controls = controls_read_in_0_7(&out.value);
            assert_eq!(
                controls[..3],
                [
                    json!({"osc": "/plain", "ways": [cc_way(7)]}),
                    mixed_in_0_7(),
                    json!({"osc": "/zero", "ways": [cc_way(0), nrpn_way(0, 0)]}),
                ],
                "{version:?} {format:?}"
            );
            assert!(out.lost.is_empty(), "{version:?} {format:?}: {:?}", out.lost);
        }
    }
}

/// Joins a controller row with `cells` to the mixed control: the losses are at `places`, the base's
/// control keeps its two ways, and the controls the row adds have the ways of `added`.
fn assert_row_on_the_mixed_control(cells: &[(&str, &str)], places: &[&str], added: &[Vec<Value>]) {
    for version in VERSIONS {
        let out = joined(base(version, &[mixed()]), &controllers(cells), Format::MidiCsv);
        let found: Vec<&str> = out.lost.iter().map(|l| l.place.as_str()).collect();
        assert_eq!(found, places, "{version:?} {cells:?}: {:?}", out.lost);
        let controls = controls_read_in_0_7(&out.value);
        assert_eq!(controls[0], mixed_in_0_7(), "{version:?} {cells:?}");
        let ways: Vec<Vec<Value>> = controls[1..].iter().map(|c| c["ways"].as_array().unwrap().clone()).collect();
        assert_eq!(ways, added, "{version:?} {cells:?}");
    }
}

#[test]
fn a_row_that_gives_again_the_cc_of_a_mixed_control_loses_that_way_at_cc_msb() {
    assert_row_on_the_mixed_control(&[("cc_msb", "1")], &["line 2", "line 2, column cc_msb"], &[]);
    assert_row_on_the_mixed_control(
        &[("cc_msb", "1"), ("nrpn_msb", "3"), ("nrpn_lsb", "4")],
        &["line 2, column cc_msb"],
        &[vec![nrpn_way(3, 4)]],
    );
}

#[test]
fn a_row_that_gives_again_the_nrpn_of_a_mixed_control_loses_that_way_at_nrpn_msb() {
    let nrpn = [("nrpn_msb", "1"), ("nrpn_lsb", "2")];
    assert_row_on_the_mixed_control(&nrpn, &["line 2", "line 2, column nrpn_msb"], &[]);
    assert_row_on_the_mixed_control(
        &[("cc_msb", "74"), nrpn[0], nrpn[1]],
        &["line 2, column nrpn_msb"],
        &[vec![cc_way(74)]],
    );
}

#[test]
fn a_row_that_gives_again_both_messages_of_a_mixed_control_loses_each_at_its_cell() {
    assert_row_on_the_mixed_control(
        &[("cc_msb", "1"), ("nrpn_msb", "1"), ("nrpn_lsb", "2")],
        &["line 2", "line 2, column cc_msb", "line 2, column nrpn_msb"],
        &[],
    );
}

/// The mixes whose NRPN is given by one number.
fn mixes_with_half_an_nrpn() -> [Value; 3] {
    [
        json!({"osc": "/a", "cc": 1, "nrpn_msb": 1}),
        json!({"osc": "/a", "cc": 1, "nrpn_msb": 0}),
        json!({"osc": "/a", "cc": 1, "nrpn_lsb": 2}),
    ]
}

#[test]
fn a_mixed_control_with_half_an_nrpn_is_written_back_in_its_version_as_read() {
    for version in VERSIONS {
        for entry in mixes_with_half_an_nrpn() {
            let text = text_of_base(version, &[entry]);
            assert_eq!(written(&device_from(&text.to_string())), text, "{version:?}");
        }
    }
}

/// A mix whose NRPN is given by one number, as 0.7 says it: the CC way, and the number given.
fn in_0_7_with_half_an_nrpn(entry: &Value) -> Value {
    let mut note = entry.clone();
    let fields = note.as_object_mut().unwrap();
    fields.remove("osc");
    fields.remove("cc");
    json!({"osc": "/a", "ways": [cc_way(1)], "_incomplete": [note]})
}

#[test]
fn a_base_with_a_mixed_control_with_half_an_nrpn_keeps_its_cc_way_and_the_number_in_the_note_with_each_csv() {
    for version in VERSIONS {
        for entry in mixes_with_half_an_nrpn() {
            let plain = (json!({"osc": "/plain", "cc": 7}), json!({"osc": "/plain", "ways": [cc_way(7)]}));
            let half = (entry.clone(), in_0_7_with_half_an_nrpn(&entry));
            for bases in [vec![half.clone()], vec![plain, half]] {
                let (entries, want): (Vec<Value>, Vec<Value>) = bases.into_iter().unzip();
                for (text, format) in files() {
                    let out = joined(base(version, &entries), &text, format);
                    let controls = controls_read_in_0_7(&out.value);
                    assert_eq!(controls[..want.len()], want[..], "{version:?} {format:?} {entry}");
                    assert!(out.lost.is_empty(), "{version:?} {format:?} {entry}: {:?}", out.lost);
                }
            }
        }
    }
}

#[test]
fn a_row_that_gives_a_whole_nrpn_is_added_beside_a_base_control_with_half_of_it() {
    for version in VERSIONS {
        // The number 0 that the base does not give is not claimed by it.
        let entry = json!({"osc": "/a", "cc": 1, "nrpn_msb": 1});
        let cells = [("nrpn_msb", "1"), ("nrpn_lsb", "0")];
        let out = joined(base(version, std::slice::from_ref(&entry)), &controllers(&cells), Format::MidiCsv);
        assert!(out.lost.is_empty(), "{version:?}: {:?}", out.lost);
        let controls = controls_read_in_0_7(&out.value);
        assert_eq!(controls[0], in_0_7_with_half_an_nrpn(&entry), "{version:?}");
        let ways: Vec<&Value> = controls[1..].iter().map(|c| &c["ways"]).collect();
        assert_eq!(ways, [&json!([nrpn_way(1, 0)])], "{version:?}");
    }
}

#[test]
fn a_base_with_a_mixed_control_with_a_block_and_no_nrpn_is_not_convertible_at_that_control_with_each_csv() {
    let entry = json!({"osc": "/a", "cc": 1, "rpn_msb": 0, "rpn_lsb": 2, "nrpn_lsb_range": [0, 15]});
    for version in VERSIONS {
        let first = (vec![entry.clone()], "/cc_params/entries/0");
        let second = (vec![json!({"osc": "/plain", "cc": 7}), entry.clone()], "/cc_params/entries/1");
        for (entries, place) in [first, second] {
            for (text, format) in files() {
                let fault = match import_into(base(version, &entries), &text, format, &source()) {
                    Ok(t) => panic!("{version:?} {format:?}: a descriptor is returned:\n{}", written(&t.value)),
                    Err(f) => f,
                };
                assert_eq!(format!("{:?}", fault.kind), "NotConvertible", "{version:?} {format:?}: {fault}");
                assert_eq!(fault.place, place, "{version:?} {format:?}: {fault}");
            }
        }
    }
}
