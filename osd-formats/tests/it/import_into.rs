//! `import_into`: the controller CSV and the triggers CSV of one device in one descriptor.

use crate::common::{csv, entries, resolved, row, source, trigger_csv, trigger_row, triggers};
use osd_formats::{export, import, import_into, FaultKind, Format};

fn controllers() -> String {
    csv(&[
        row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "74"), ("cc_min_value", "0"), ("cc_max_value", "127")]),
        row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Filter"), ("parameter_name", "Cutoff"), ("cc_msb", "75")]),
    ])
}

fn drums() -> String {
    trigger_csv(&[
        trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("midi_channel", "10"), ("note_number_min_value", "36"), ("note_number_max_value", "36")]),
        trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("note_number_min_value", "37"), ("note_number_max_value", "37")]),
    ])
}

fn first(text: &str, format: Format) -> osd::Device {
    import(text, format, &source()).unwrap().value
}

fn other_source() -> osd::Source {
    let mut s = source();
    s.url = Some("https://example.org/triggers".into());
    s
}

#[test]
fn a_triggers_file_joins_the_descriptor_of_the_controller_file_and_both_sources_are_kept() {
    let base = first(&controllers(), Format::MidiCsv);
    let t = import_into(base.clone(), &drums(), Format::MidiTriggersCsv, &other_source()).unwrap();
    assert_eq!(entries(&t.value), entries(&base));
    assert_eq!(triggers(&t.value).len(), 2);
    assert_eq!(t.value.sources, Some(vec![source(), other_source()]));
    assert_eq!(t.value.device, base.device);
    assert!(t.lost.is_empty(), "{:?}", t.lost);
}

#[test]
fn a_controller_file_joins_the_descriptor_of_the_triggers_file() {
    let base = first(&drums(), Format::MidiTriggersCsv);
    let t = import_into(base.clone(), &controllers(), Format::MidiCsv, &other_source()).unwrap();
    assert_eq!(triggers(&t.value), triggers(&base));
    assert_eq!(entries(&t.value).len(), 2);
    assert_eq!(t.value.sources, Some(vec![source(), other_source()]));
}

#[test]
fn the_two_files_in_either_order_give_the_same_controls_and_triggers() {
    let a = import_into(first(&controllers(), Format::MidiCsv), &drums(), Format::MidiTriggersCsv, &source()).unwrap().value;
    let b = import_into(first(&drums(), Format::MidiTriggersCsv), &controllers(), Format::MidiCsv, &source()).unwrap().value;
    assert_eq!(entries(&a), entries(&b));
    assert_eq!(triggers(&a), triggers(&b));
    assert_eq!(a.device, b.device);
}

#[test]
fn the_export_of_the_joined_descriptor_gives_back_each_of_the_two_csv() {
    let joined = import_into(first(&controllers(), Format::MidiCsv), &drums(), Format::MidiTriggersCsv, &source()).unwrap().value;
    let r = resolved(joined);
    let back_triggers = export(&r, Format::MidiTriggersCsv).unwrap().value;
    let back_controllers = export(&r, Format::MidiCsv).unwrap().value;
    assert_eq!(crate::common::parse(&back_triggers), crate::common::parse(&drums()));
    assert_eq!(crate::common::parse(&back_controllers), crate::common::parse(&controllers()));
}

#[test]
fn the_addresses_are_unique_over_the_whole_descriptor_with_the_suffixes_of_the_import() {
    // A trigger and a control of the same section and name must not share an address.
    let cc = csv(&[row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("parameter_name", "Kick"), ("cc_msb", "1")])]);
    let tr = trigger_csv(&[trigger_row(&[("manufacturer", "Lofty"), ("device", "Trundler"), ("section", "Drums"), ("sound_name", "Kick"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])]);
    let d = import_into(first(&cc, Format::MidiCsv), &tr, Format::MidiTriggersCsv, &source()).unwrap().value;
    let mut all: Vec<String> = entries(&d).iter().map(|e| e.osc.clone()).chain(triggers(&d).iter().map(|e| e.osc.clone())).collect();
    assert_eq!(all.len(), 2);
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 2, "addresses repeat");
}

#[test]
fn rows_of_a_second_file_come_after_the_entries_the_base_already_holds() {
    let a = first(&drums(), Format::MidiTriggersCsv);
    let t = import_into(a.clone(), &drums(), Format::MidiTriggersCsv, &source()).unwrap();
    let list = triggers(&t.value);
    assert_eq!(list.len(), 4);
    assert_eq!(&list[..2], &triggers(&a)[..]);
    let mut osc: Vec<&str> = list.iter().map(|e| e.osc.as_str()).collect();
    osc.sort();
    osc.dedup();
    assert_eq!(osc.len(), 4);
    assert_eq!(t.value.sources.as_ref().unwrap().len(), 2);
}

#[test]
fn the_identity_of_the_base_wins_and_a_different_vendor_or_name_is_a_loss_at_its_cell() {
    let base = first(&controllers(), Format::MidiCsv);
    let other = trigger_csv(&[trigger_row(&[("manufacturer", "Other"), ("device", "Else"), ("sound_name", "Kick"), ("note_number_min_value", "36"), ("note_number_max_value", "36")])]);
    let t = import_into(base, &other, Format::MidiTriggersCsv, &source()).unwrap();
    assert_eq!((t.value.device.vendor.as_deref(), t.value.device.name.as_str()), (Some("Lofty"), "Trundler"));
    let places: Vec<&str> = t.lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, ["line 2, column manufacturer", "line 2, column device"]);
}

#[test]
fn the_other_blocks_of_the_base_are_left_alone() {
    let text = r#"{"$schema": "https://w3id.org/osd/0.6/device.schema.json", "_sources": [{"type": "community-re", "url": "u", "license": "CC0-1.0"}],
        "device": {"name": "Trundler", "vendor": "Lofty", "osc_prefix": "/trundler"}, "sysex": {"header": [240], "footer": [247]}}"#;
    let base = crate::common::device_from(text);
    let t = import_into(base.clone(), &drums(), Format::MidiTriggersCsv, &source()).unwrap();
    assert_eq!(t.value.sysex, base.sysex);
    assert_eq!(t.value.device, base.device);
    assert_eq!(t.value.sources.as_ref().unwrap().len(), 2);
}

#[test]
fn a_format_other_than_the_two_csv_is_refused_as_an_invalid_file() {
    for format in [Format::ElectraOne, Format::Midnam, Format::OscQuery] {
        let base = first(&controllers(), Format::MidiCsv);
        let f = import_into(base, "{}", format, &source()).expect_err("refused");
        assert_eq!(f.kind, FaultKind::InvalidFile, "{format:?}");
    }
}

#[test]
fn the_source_and_the_file_are_judged_as_by_import() {
    let base = first(&controllers(), Format::MidiCsv);
    let mut no_license = source();
    no_license.license = None;
    let f = import_into(base.clone(), &drums(), Format::MidiTriggersCsv, &no_license).expect_err("refused");
    assert_eq!(f.kind, FaultKind::SourceIncomplete);
    let f = import_into(base, "manufacturer,device\n", Format::MidiTriggersCsv, &source()).expect_err("refused");
    assert_eq!(f.kind, FaultKind::InvalidFile);
    assert!(f.place.starts_with("line 1"), "{:?}", f.place);
}
