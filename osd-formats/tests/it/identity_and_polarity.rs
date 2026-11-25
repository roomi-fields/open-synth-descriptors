//! The forms 0.7 changes, in the formats: the maker's SysEx id is `/identity/manufacturerId` in the
//! losses of every format that has no place for it, and midi.guide's orientation column reads and
//! writes `polarity` (`centered` is `bipolar`, `0-based` is `unipolar`), exactly.

use crate::common::{device_from, entries, parse, resolved, row, csv, source, HEADER};
use osd_formats::{export, import, Format};
use serde_json::{json, Value};

fn device_07(extra: Value) -> osd::Resolved {
    let mut doc = json!({
        "$schema": "https://w3id.org/osd/0.7/device.schema.json",
        "sources": [{"type": "community-re", "url": "https://example.org/d", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"},
    });
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    resolved(device_from(&doc.to_string()))
}

#[test]
fn every_format_without_a_place_for_the_makers_id_names_it_at_its_identity_pointer() {
    let d = device_07(json!({"identity": {"manufacturerId": [0, 32, 41]}}));
    for format in [Format::MidiCsv, Format::MidiTriggersCsv, Format::ElectraOne, Format::Midnam, Format::OscQuery] {
        let t = export(&d, format).unwrap_or_else(|f| panic!("{format:?}: {f:?}"));
        let at: Vec<_> = t.lost.iter().filter(|l| l.place == "/identity/manufacturerId").collect();
        assert_eq!(at.len(), 1, "{format:?}: {:?}", t.lost);
        assert!(!at[0].reason.is_empty(), "{format:?}");
        assert!(t.lost.iter().all(|l| l.place != "/device/manufacturer_id"), "{format:?}: {:?}", t.lost);
    }
}

#[test]
fn a_device_without_a_makers_id_has_no_loss_for_it() {
    let d = device_07(json!({"identity": {"reply": [240, 126, "any", 6, 2]}}));
    for format in [Format::MidiCsv, Format::MidiTriggersCsv, Format::ElectraOne, Format::Midnam, Format::OscQuery] {
        let t = export(&d, format).unwrap_or_else(|f| panic!("{format:?}: {f:?}"));
        assert!(t.lost.iter().all(|l| l.place != "/identity/manufacturerId"), "{format:?}: {:?}", t.lost);
    }
}

fn controls() -> Value {
    json!({"cc_params": {"entries": [
        {"osc": "/pan", "ways": [{"ctrlType": "cc", "ctrlIndex": [10]}], "polarity": "bipolar"},
        {"osc": "/cutoff", "ways": [{"ctrlType": "cc", "ctrlIndex": [74]}], "polarity": "unipolar"},
        {"osc": "/level", "ways": [{"ctrlType": "cc", "ctrlIndex": [7]}]},
    ]}})
}

#[test]
fn the_orientation_column_is_written_from_the_polarity() {
    let t = export(&device_07(controls()), Format::MidiCsv).unwrap();
    let column = HEADER.iter().position(|h| *h == "orientation").unwrap();
    let cells: Vec<String> = parse(&t.value)[1..].iter().map(|r| r[column].clone()).collect();
    assert_eq!(cells, ["centered", "0-based", ""]);
    assert!(t.lost.iter().all(|l| !l.place.ends_with("/polarity")), "{:?}", t.lost);
}

#[test]
fn the_orientation_column_is_read_into_the_polarity() {
    let rows = [
        row(&[("parameter_name", "Pan"), ("cc_msb", "10"), ("orientation", "centered")]),
        row(&[("parameter_name", "Cutoff"), ("cc_msb", "74"), ("orientation", "0-based")]),
        row(&[("parameter_name", "Level"), ("cc_msb", "7")]),
    ];
    let t = import(&csv(&rows), Format::MidiCsv, &source()).unwrap();
    let polarity: Vec<_> = entries(&t.value).iter().map(|e| e.polarity).collect();
    assert_eq!(polarity, [Some(osd::Polarity::Bipolar), Some(osd::Polarity::Unipolar), None]);
    let written: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(t.value)))).unwrap();
    let list = written["cc_params"]["entries"].as_array().unwrap();
    assert_eq!((list[0]["polarity"].as_str(), list[1]["polarity"].as_str()), (Some("bipolar"), Some("unipolar")));
    assert!(list.iter().all(|e| e.get("orientation").is_none()), "{written}");
}

#[test]
fn a_polarity_makes_the_round_trip_through_the_csv_exact() {
    let first = export(&device_07(controls()), Format::MidiCsv).unwrap();
    let back = import(&first.value, Format::MidiCsv, &source()).unwrap();
    let polarity: Vec<_> = entries(&back.value).iter().map(|e| e.polarity).collect();
    assert_eq!(polarity, [Some(osd::Polarity::Bipolar), Some(osd::Polarity::Unipolar), None]);
    let second = export(&resolved(back.value), Format::MidiCsv).unwrap();
    assert_eq!(parse(&second.value), parse(&first.value));
}

#[test]
fn the_formats_with_no_field_for_it_name_the_polarity_in_the_losses() {
    let d = device_07(controls());
    for format in [Format::ElectraOne, Format::Midnam, Format::OscQuery, Format::Midi2Controllers] {
        let t = export(&d, format).unwrap_or_else(|f| panic!("{format:?}: {f:?}"));
        for place in ["/cc_params/entries/0/polarity", "/cc_params/entries/1/polarity"] {
            assert_eq!(t.lost.iter().filter(|l| l.place == place).count(), 1, "{format:?} {place}: {:?}", t.lost);
        }
        assert!(t.lost.iter().all(|l| !l.place.ends_with("/orientation")), "{format:?}: {:?}", t.lost);
        assert!(t.lost.iter().all(|l| !l.place.starts_with("/cc_params/entries/2/polarity")), "{format:?}");
    }
}
