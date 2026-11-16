//! An integer is read by its value, as JSON Schema 2020-12 defines it: a
//! number with a zero fraction (`1.0`, `1e2`) is that integer, and it is
//! written back without a fraction.

use osd::{FaultKind, Kind};
use serde_json::{json, Value};

/// A device whose integers are written with a zero fraction, in each kind
/// of place an integer sits: a bounded field, a free one, a byte of a
/// frame, a pair, the values of a dictionary.
const WITH_FRACTIONS: &str = r#"{
  "device": {"name": "Test", "osc_prefix": "/test"},
  "midi_out": {"default_channel": 1.0, "note_offset": 1e2},
  "cc_params": {"entries": [{"osc": "/a", "cc": 7.0, "range": [0.0, 1.27e2]}]},
  "params": {"get_frame": [240.0, "{value}", 2.47E2], "set_frame": [], "value_type": "u7", "entries": [{"osc": "/b", "p": 1.0, "values": {"off": 0.0, "on": 1e0}}]}
}"#;

/// The same device, its integers written as integers.
const WITHOUT_FRACTIONS: &str = r#"{
  "device": {"name": "Test", "osc_prefix": "/test"},
  "midi_out": {"default_channel": 1, "note_offset": 100},
  "cc_params": {"entries": [{"osc": "/a", "cc": 7, "range": [0, 127]}]},
  "params": {"get_frame": [240, "{value}", 247], "set_frame": [], "value_type": "u7", "entries": [{"osc": "/b", "p": 1, "values": {"off": 0, "on": 1}}]}
}"#;

fn device(text: &str) -> Box<osd::Device> {
    match osd::read(text, Kind::Device) {
        Ok(osd::Descriptor::Device(d)) => d,
        other => panic!("not a device: {other:?}"),
    }
}

#[test]
fn a_number_with_a_zero_fraction_reads_as_that_integer() {
    assert_eq!(device(WITH_FRACTIONS), device(WITHOUT_FRACTIONS));
}

#[test]
fn an_integer_read_with_a_zero_fraction_is_written_without_one() {
    let written: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(device(WITH_FRACTIONS)))).unwrap();
    let want: Value = serde_json::from_str(WITHOUT_FRACTIONS).unwrap();
    assert_eq!(written, want);
    assert_eq!(written["midi_out"]["default_channel"], json!(1));
    assert!(written["midi_out"]["default_channel"].is_i64(), "{}", written["midi_out"]["default_channel"]);
}

#[test]
fn a_number_with_a_fraction_where_an_integer_is_expected_is_refused() {
    let cases = [
        (r#"{"default_channel": 1.5}"#, "/midi_out/default_channel"),
        (r#"{"note_offset": 1e-1}"#, "/midi_out/note_offset"),
        (r#"{"note_offset": 100.000001}"#, "/midi_out/note_offset"),
    ];
    for (midi_out, pointer) in cases {
        let text = format!(r#"{{"device": {{"name": "Test", "osc_prefix": "/test"}}, "midi_out": {midi_out}}}"#);
        let f = osd::read(&text, Kind::Device).expect_err("refused");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, pointer), "{f}");
    }
}

#[test]
fn a_whole_number_beyond_its_bounds_stays_refused() {
    let text = r#"{"device": {"name": "Test", "osc_prefix": "/test"}, "midi_out": {"default_channel": 1.6e1}}"#;
    let f = osd::read(text, Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/midi_out/default_channel"), "{f}");
    let text = r#"{"device": {"name": "Test", "osc_prefix": "/test"}, "midi_out": {"note_offset": 1e300}}"#;
    let f = osd::read(text, Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, "/midi_out/note_offset"), "{f}");
}
