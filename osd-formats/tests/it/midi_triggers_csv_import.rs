//! The import of a pencilresearch/midi triggers CSV: the descriptor, its source, its refusals and
//! its losses.

use crate::common::{csv, entries, row, source, trigger_csv, trigger_header_line, trigger_row, triggers};
use osd_formats::{import, FaultKind, Format};
use serde_json::Value;

fn ok(text: &str) -> osd_formats::Translated<osd::Device> {
    import(text, Format::MidiTriggersCsv, &source()).unwrap_or_else(|f| panic!("refused: {f:?}"))
}

fn refused(text: &str) -> osd_formats::Fault {
    match import(text, Format::MidiTriggersCsv, &source()) {
        Ok(_) => panic!("the file is accepted"),
        Err(f) => f,
    }
}

fn kick() -> Vec<String> {
    trigger_row(&[
        ("manufacturer", "Lofty"),
        ("device", "Trundler"),
        ("section", "Drums"),
        ("sound_name", "Kick"),
        ("sound_description", "The big one"),
        ("midi_channel", "10"),
        ("note_number_min_value", "36"),
        ("note_number_max_value", "38"),
        ("note_number_default_value", "37"),
        ("comments", "Choke group 1"),
        ("velocity_usage", "Soft: damped; hard: open"),
    ])
}

fn plain(name: &str, min: &str, max: &str) -> Vec<String> {
    trigger_row(&[("sound_name", name), ("note_number_min_value", min), ("note_number_max_value", max)])
}

fn places(t: &osd_formats::Translated<osd::Device>) -> Vec<&str> {
    t.lost.iter().map(|l| l.place.as_str()).collect()
}

#[test]
fn a_row_with_a_note_range_becomes_a_trigger_with_every_column_it_carries() {
    let t = ok(&trigger_csv(&[kick()]));
    let list = triggers(&t.value);
    assert_eq!(list.len(), 1);
    let e = &list[0];
    assert_eq!(e.sound, "Kick");
    assert_eq!(e.section.as_deref(), Some("Drums"));
    assert_eq!(e.channel, Some(9));
    assert_eq!(e.note_range, [36, 38]);
    assert_eq!(e.note, Some(37));
    assert_eq!(e.comments.as_deref(), Some("Choke group 1"));
    assert_eq!(e.velocity_usage.as_deref(), Some("Soft: damped; hard: open"));
    assert_eq!(e.channel_mode, None);
    assert!(e.osc.starts_with('/'));
    assert_eq!(e.remarks.get("_description").and_then(Value::as_str), Some("The big one"));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
}

#[test]
fn the_manufacturer_and_device_name_the_descriptor_device_from_the_first_row_that_gives_them() {
    let first = plain("A", "1", "1");
    let mut second = kick();
    second[0] = "Lofty".into();
    let t = ok(&trigger_csv(&[first, second]));
    assert_eq!(t.value.device.vendor.as_deref(), Some("Lofty"));
    assert_eq!(t.value.device.name, "Trundler");
    assert!(t.value.device.osc_prefix.starts_with('/'));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
}

#[test]
fn a_device_or_manufacturer_that_changes_between_rows_is_a_loss_at_its_cell() {
    let mut other = kick();
    other[0] = "Other".into();
    other[1] = "Else".into();
    let t = ok(&trigger_csv(&[kick(), other]));
    assert_eq!(t.value.device.name, "Trundler");
    assert_eq!(places(&t), ["line 3, column manufacturer", "line 3, column device"]);
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn the_descriptor_is_the_latest_version_and_reads_strictly_with_the_source_as_given() {
    let src = source();
    let t = import(&trigger_csv(&[kick()]), Format::MidiTriggersCsv, &src).unwrap();
    assert_eq!(t.value.schema, Some(osd::Version::V0_6));
    assert_eq!(t.value.sources, Some(vec![src]));
    let text = osd::write(&osd::Descriptor::Device(Box::new(t.value.clone())));
    match osd::read(&text, osd::Kind::Device) {
        Ok(osd::Descriptor::Device(back)) => assert_eq!(*back, t.value),
        other => panic!("not read back strictly: {other:?}"),
    }
}

#[test]
fn an_empty_cell_gives_no_value_the_source_does_not_give() {
    let t = ok(&trigger_csv(&[plain("Bare", "10", "20")]));
    let e = &triggers(&t.value)[0];
    assert_eq!((e.channel, e.note, e.section.clone(), e.comments.clone(), e.velocity_usage.clone()), (None, None, None, None, None));
    assert_eq!(e.channel_mode, None);
    assert!(!e.remarks.contains_key("_description"));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
}

#[test]
fn the_channel_is_an_integer_from_1_to_16_shifted_to_0_to_15() {
    for (cell, channel) in [("1", 0u8), ("16", 15), ("10", 9)] {
        let mut r = plain("S", "1", "2");
        r[5] = cell.into();
        let t = ok(&trigger_csv(&[r]));
        assert_eq!(triggers(&t.value)[0].channel, Some(channel), "{cell}");
        assert!(t.lost.is_empty(), "{cell}: {:?}", t.lost);
    }
}

#[test]
fn a_channel_that_is_not_an_integer_from_1_to_16_is_a_loss_never_a_channel_mode() {
    for cell in ["omni", "any", "Omni", "0", "17", "-1", "1.5", "ch 3"] {
        let mut r = plain("S", "1", "2");
        r[5] = cell.into();
        let t = ok(&trigger_csv(&[kick(), r]));
        let list = triggers(&t.value);
        assert_eq!(list.len(), 2, "{cell}");
        assert_eq!(list[1].channel, None, "{cell}");
        assert_eq!(list[1].channel_mode, None, "{cell}");
        assert_eq!(places(&t), ["line 3, column midi_channel"], "{cell}");
        assert!(!t.lost[0].reason.is_empty());
    }
}

#[test]
fn a_range_of_equal_bounds_is_a_range_and_a_default_inside_it_is_the_note() {
    let mut r = plain("Snare", "38", "38");
    r[8] = "38".into();
    let t = ok(&trigger_csv(&[r]));
    let e = &triggers(&t.value)[0];
    assert_eq!((e.note_range, e.note), ([38, 38], Some(38)));
    assert!(t.lost.is_empty(), "{:?}", t.lost);
}

#[test]
fn a_default_outside_the_range_is_a_loss_at_its_column_and_gives_no_note() {
    for default in ["10", "50"] {
        let mut r = plain("Snare", "20", "30");
        r[8] = default.into();
        let t = ok(&trigger_csv(&[kick(), r]));
        assert_eq!(triggers(&t.value)[1].note, None, "{default}");
        assert_eq!(places(&t), ["line 3, column note_number_default_value"], "{default}");
        assert!(!t.lost[0].reason.is_empty());
    }
}

#[test]
fn a_row_without_both_bounds_is_no_trigger_and_is_a_loss_by_its_line_with_its_reason() {
    let only_min = trigger_row(&[("sound_name", "A"), ("note_number_min_value", "36")]);
    let only_max = trigger_row(&[("sound_name", "B"), ("note_number_max_value", "36")]);
    let only_default = trigger_row(&[("sound_name", "C"), ("note_number_default_value", "36")]);
    let no_note = trigger_row(&[("sound_name", "D"), ("comments", "just a remark")]);
    let t = ok(&trigger_csv(&[kick(), only_min, only_max, only_default, no_note]));
    let list = triggers(&t.value);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].sound, "Kick");
    assert_eq!(places(&t), ["line 3", "line 4", "line 5", "line 6"]);
    assert!(t.lost.iter().all(|l| !l.reason.is_empty()));
}

#[test]
fn a_file_without_any_trigger_gives_a_descriptor_without_entries() {
    let t = ok(&trigger_csv(&[trigger_row(&[("sound_name", "D")])]));
    assert!(triggers(&t.value).is_empty());
    assert_eq!(places(&t), ["line 2"]);
}

#[test]
fn each_trigger_address_is_unique_with_the_suffix_of_the_controller_import_in_the_order_of_the_rows() {
    let same = |lo: &str| trigger_row(&[("section", "Drums"), ("sound_name", "Kick"), ("note_number_min_value", lo), ("note_number_max_value", lo)]);
    let list = triggers(&ok(&trigger_csv(&[same("1"), same("2"), same("3")])).value);
    assert_eq!(list.len(), 3);
    assert!(list[1].osc.ends_with("-2") && list[2].osc.ends_with("-3"), "{:?}", list.iter().map(|e| &e.osc).collect::<Vec<_>>());
    assert_eq!(list[1].osc, format!("{}-2", list[0].osc));
    assert_eq!(list[2].osc, format!("{}-3", list[0].osc));
}

#[test]
fn the_address_is_built_from_section_and_name_as_the_controller_import_builds_it() {
    let cc = row(&[("section", "Drums & Co"), ("parameter_name", "Kick drum!"), ("cc_msb", "1")]);
    let cc_osc = entries(&import(&csv(&[cc]), Format::MidiCsv, &source()).unwrap().value)[0].osc.clone();
    let tr = trigger_row(&[("section", "Drums & Co"), ("sound_name", "Kick drum!"), ("note_number_min_value", "1"), ("note_number_max_value", "1")]);
    assert_eq!(triggers(&ok(&trigger_csv(&[tr])).value)[0].osc, cc_osc);
}

#[test]
fn an_import_without_license_is_refused_naming_the_license() {
    let mut src = source();
    src.license = None;
    let f = import(&trigger_csv(&[kick()]), Format::MidiTriggersCsv, &src).expect_err("refused");
    assert_eq!(f.kind, FaultKind::SourceIncomplete);
    assert!(f.place.contains("license"), "{:?}", f.place);
}

#[test]
fn an_import_without_url_author_or_attribution_is_refused_naming_them() {
    let mut src = source();
    (src.url, src.author, src.attribution) = (None, None, None);
    let f = import(&trigger_csv(&[kick()]), Format::MidiTriggersCsv, &src).expect_err("refused");
    assert_eq!(f.kind, FaultKind::SourceIncomplete);
    for field in ["url", "author", "attribution"] {
        assert!(f.place.contains(field), "{:?} does not name {field}", f.place);
    }
}

#[test]
fn a_file_whose_header_differs_from_the_template_is_refused_at_line_1() {
    let controller_header = csv(&[]);
    let swapped = "device,manufacturer,section,sound_name,sound_description,midi_channel,note_number_min_value,note_number_max_value,note_number_default_value,comments,velocity_usage\n";
    let extra = format!("{},extra\n", trigger_header_line());
    let short = "manufacturer,device,section,sound_name\n";
    for text in ["", controller_header.as_str(), swapped, extra.as_str(), short, "Lofty,Trundler,Drums\n"] {
        let f = refused(text);
        assert_eq!(f.kind, FaultKind::InvalidFile, "{text:?}");
        assert!(f.place.starts_with("line 1"), "{text:?}: {:?}", f.place);
    }
}

#[test]
fn a_row_whose_cell_count_differs_from_the_header_is_refused_at_its_line() {
    let text = format!("{}\nLofty,Trundler,Drums,Kick,,10,36,38,,,\nLofty,Trundler,only three\n", trigger_header_line());
    let f = refused(&text);
    assert_eq!(f.kind, FaultKind::InvalidFile);
    assert!(f.place.starts_with("line 3"), "{:?}", f.place);
}

#[test]
fn a_note_number_outside_0_to_127_or_unreadable_is_refused_at_its_line() {
    for column in [6, 7, 8] {
        for value in ["128", "-1", "x", "1.5", "1e2"] {
            let mut r = plain("S", "1", "2");
            r[column] = value.into();
            let f = refused(&trigger_csv(&[kick(), r]));
            assert_eq!(f.kind, FaultKind::InvalidFile, "{column}={value}");
            assert!(f.place.starts_with("line 3"), "{column}={value}: {:?}", f.place);
        }
    }
}

#[test]
fn a_minimum_above_its_maximum_is_refused_at_its_line() {
    let f = refused(&trigger_csv(&[kick(), plain("S", "100", "50")]));
    assert_eq!(f.kind, FaultKind::InvalidFile);
    assert!(f.place.starts_with("line 3"), "{:?}", f.place);
}

#[test]
fn the_losses_come_in_the_order_of_the_source_and_are_the_same_on_each_import() {
    let mut a = plain("A", "1", "5");
    a[8] = "9".into();
    let none = trigger_row(&[("sound_name", "Nothing")]);
    let mut b = plain("B", "1", "5");
    b[5] = "omni".into();
    let text = trigger_csv(&[a, none, b]);
    let first = ok(&text);
    assert_eq!(places(&first), ["line 2, column note_number_default_value", "line 3", "line 4, column midi_channel"]);
    assert_eq!(first.lost, ok(&text).lost);
}
