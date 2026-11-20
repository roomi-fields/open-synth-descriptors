//! An OSD device exports to the device definition the Deluge reads (an XML `midiDevice` with
//! a `ccLabels` tag, one attribute per CC from 0 to 119), the credit in an XML comment, and
//! what the format has no place for named in the losses.

use crate::common::deluge::*;
use osd_formats::{credit_to_carry, export, import, FaultKind, Format};
use serde_json::json;

fn name_of(text: &str, cc: u32) -> Option<String> {
    labels_read(text).into_iter().find(|(n, _)| *n == cc).map(|(_, v)| v)
}

#[test]
fn the_form_the_firmware_definitions_have_is_the_form_the_export_writes() {
    let firmware = labels_read(FIRMWARE_DEFINITION);
    assert_eq!(firmware.len(), 5);
    let d = cc_device(json!([{"osc": "/a", "cc": 1}, {"osc": "/b", "cc": 7}]));
    let (text, _) = exported(&d);
    assert_well_formed(&text);
    let root = crate::common::xml::parse(&text).unwrap().root;
    assert_eq!(root.name, "midiDevice");
    assert_eq!(root.kids("ccLabels").len(), 1, "{text}");
    assert_eq!(labels_read(&text), [(1, "/a".to_string()), (7, "/b".to_string())]);
}

#[test]
fn a_cc_from_0_to_119_is_one_attribute_of_cclabels_named_by_the_role_path() {
    let d = cc_device(json!([
        {"osc": "/filter/cutoff", "cc": 74, "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}},
        {"osc": "/zero", "cc": 0},
        {"osc": "/last", "cc": 119},
    ]));
    let (text, _) = exported(&d);
    assert_eq!(name_of(&text, 74).as_deref(), Some("/Filter/Cutoff"), "{text}");
    assert_eq!(name_of(&text, 0).as_deref(), Some("/zero"));
    assert_eq!(name_of(&text, 119).as_deref(), Some("/last"));
    assert_eq!(labels_read(&text).len(), 3);
}

#[test]
fn the_name_is_the_role_path_else_a_section_that_starts_with_a_slash_else_the_osc_address() {
    let d = cc_device(json!([
        {"osc": "/o1", "cc": 1, "section": "/Sec/One", "role": {"parts": [{"part": "filter"}], "setting": "cutoff"}},
        {"osc": "/o2", "cc": 2, "section": "/Sec/Two"},
        {"osc": "/o3", "cc": 3},
        {"osc": "/o4", "cc": 4, "section": "Free section"},
    ]));
    let (text, lost) = exported(&d);
    assert_eq!(name_of(&text, 1).as_deref(), Some("/Filter/Cutoff"));
    assert_eq!(name_of(&text, 2).as_deref(), Some("/Sec/Two"));
    assert_eq!(name_of(&text, 3).as_deref(), Some("/o3"));
    assert_eq!(name_of(&text, 4).as_deref(), Some("/o4"), "a free section is never a name");
    assert!(places(&lost).contains(&"/cc_params/entries/3/section"), "{lost:?}");
    assert!(!places(&lost).contains(&"/cc_params/entries/1/section"), "{lost:?}");
}

#[test]
fn a_name_is_escaped_for_xml_and_the_document_stays_well_formed() {
    let tricky = "/a\"b&c<d>e";
    let d = cc_device(json!([{"osc": "/x", "cc": 5, "section": tricky}, {"osc": "/y", "cc": 6, "section": "/it's <b>"}]));
    let (text, _) = exported(&d);
    assert_well_formed(&text);
    assert_eq!(name_of(&text, 5).as_deref(), Some(tricky));
    assert_eq!(name_of(&text, 6).as_deref(), Some("/it's <b>"));
    assert!(text.contains("&amp;") && text.contains("&lt;") && text.contains("&quot;"), "{text}");
    assert!(!text.contains("a\"b"), "{text}");
}

#[test]
fn a_cc_comes_out_with_its_name_and_the_fields_the_format_does_not_carry_are_named_lost() {
    let d = cc_device(json!([{
        "osc": "/filter/cutoff", "cc": 74, "range": [10, 100], "default": 50, "priority": 2, "channel": 3,
        "values": {"low": 10, "high": 100}
    }]));
    let (text, lost) = exported(&d);
    assert_eq!(name_of(&text, 74).as_deref(), Some("/filter/cutoff"));
    let p = places(&lost);
    for field in ["range", "default", "priority", "channel", "values"] {
        let place = format!("/cc_params/entries/0/{field}");
        assert!(p.contains(&place.as_str()), "{place} is not named: {p:?}");
    }
    assert!(!p.contains(&"/cc_params/entries/0"), "the command is written: {p:?}");
}

#[test]
fn a_command_with_no_cc_or_a_cc_above_119_is_not_written_and_is_named_lost() {
    let d = device(json!({
        "cc_params": {"entries": [
            {"osc": "/a", "cc": 10},
            {"osc": "/n", "nrpn_msb": 3, "nrpn_lsb": 9},
            {"osc": "/r", "rpn_msb": 0, "rpn_lsb": 2},
            {"osc": "/h", "cc": 120},
            {"osc": "/z", "cc": 127},
        ]},
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
                   "entries": [{"osc": "/s", "id": 12, "range": [0, 63]}]},
    }));
    let (text, lost) = exported(&d);
    assert_eq!(labels_read(&text), [(10, "/a".to_string())], "{text}");
    let p = places(&lost);
    for place in ["/cc_params/entries/1", "/cc_params/entries/2", "/cc_params/entries/3", "/cc_params/entries/4", "/params/entries/0"] {
        assert!(p.contains(&place), "{place} is not named: {p:?}");
    }
    assert!(!p.contains(&"/cc_params/entries/0"), "{p:?}");
}

#[test]
fn two_commands_with_the_same_cc_write_one_attribute_the_first_in_the_order_of_the_descriptor() {
    let d = cc_device(json!([
        {"osc": "/first", "cc": 20, "channel": 1},
        {"osc": "/other", "cc": 21},
        {"osc": "/second", "cc": 20, "channel": 2},
        {"osc": "/third", "cc": 20, "channel": 3},
    ]));
    let (text, lost) = exported(&d);
    assert_eq!(labels_read(&text), [(20, "/first".to_string()), (21, "/other".to_string())], "{text}");
    let p = places(&lost);
    assert!(p.contains(&"/cc_params/entries/2"), "{p:?}");
    assert!(p.contains(&"/cc_params/entries/3"), "{p:?}");
    assert!(!p.contains(&"/cc_params/entries/0"), "{p:?}");
}

#[test]
fn the_cc_come_out_in_ascending_order_whatever_the_order_of_the_commands() {
    let d = cc_device(json!([
        {"osc": "/c", "cc": 90}, {"osc": "/a", "cc": 3}, {"osc": "/d", "cc": 119}, {"osc": "/b", "cc": 74}, {"osc": "/z", "cc": 0},
    ]));
    let (text, _) = exported(&d);
    let numbers: Vec<u32> = labels_read(&text).into_iter().map(|(n, _)| n).collect();
    assert_eq!(numbers, [0, 3, 74, 90, 119]);
}

#[test]
fn the_document_holds_no_tag_nor_attribute_but_midi_device_cc_labels_and_its_numbers() {
    let d = cc_device(json!([{"osc": "/a", "cc": 1, "range": [0, 10], "values": {"x": 1}}, {"osc": "/b", "cc": 2}]));
    let (text, _) = exported(&d);
    let root = crate::common::xml::parse(&text).unwrap().root;
    assert_eq!(root.name, "midiDevice");
    assert!(root.attrs.is_empty(), "{:?}", root.attrs);
    assert!(root.text.is_empty());
    assert_eq!(root.children.len(), 1);
    let labels = &root.children[0];
    assert_eq!(labels.name, "ccLabels");
    assert!(labels.children.is_empty());
    for (k, _) in &labels.attrs {
        let n: u32 = k.parse().unwrap_or_else(|_| panic!("the attribute {k} is not a number"));
        assert!(n < 120);
    }
    assert_eq!(labels.attrs.len(), 2);
}

#[test]
fn the_document_carries_the_credit_and_licence_of_each_source_in_a_comment_and_names_the_sources_lost() {
    let d = device(json!({
        "_sources": [
            {"type": "community-re", "url": "https://example.org/one", "author": "First Maker", "license": "CC-BY-4.0"},
            {"type": "vendor-doc", "url": "https://example.org/two", "attribution": "Second Co", "license": "MIT"},
        ],
        "cc_params": {"entries": [{"osc": "/a", "cc": 1}]},
    }));
    let t = export(&d, Format::Deluge).unwrap();
    assert!(places(&t.lost).contains(&"/_sources"), "{:?}", t.lost);
    let lines = credit_to_carry(&d, &t.lost);
    assert_eq!(lines.len(), 2);
    assert_well_formed(&t.value);
    let comment = comments(&t.value).join("\n");
    for line in &lines {
        assert!(comment.contains(line.as_str()), "{line:?} is not in the comment {comment:?}");
    }
    assert!(comment.contains("CC-BY-4.0") && comment.contains("MIT"));
}

#[test]
fn a_credit_with_double_hyphens_is_broken_so_that_the_comment_stays_well_formed() {
    let d = device(json!({
        "_sources": [{"type": "community-re", "url": "https://example.org/a--b", "author": "Foo--Bar", "license": "CC-BY-4.0"}],
        "cc_params": {"entries": [{"osc": "/a", "cc": 1}]},
    }));
    let (text, _) = exported(&d);
    assert_well_formed(&text);
    let comment = comments(&text).join("\n");
    assert!(comment.contains("Foo- -Bar"), "{comment:?}");
    assert!(comment.contains("https://example.org/a- -b"), "{comment:?}");
    assert_eq!(labels_read(&text), [(1, "/a".to_string())]);
}

#[test]
fn a_device_without_sources_is_refused() {
    let d = device(json!({"_sources": null, "cc_params": {"entries": [{"osc": "/a", "cc": 1}]}}));
    let f = export(&d, Format::Deluge).unwrap_err();
    assert_eq!(f.kind, FaultKind::NoSources);
}

#[test]
fn the_device_blocks_the_format_has_no_place_for_are_named_lost() {
    let (_, lost) = exported(&cc_device(json!([{"osc": "/a", "cc": 1}])));
    assert!(places(&lost).contains(&"/device/name"), "{lost:?}");
}

#[test]
fn the_deluge_is_exported_for_every_credited_device_of_the_cases_and_examples() {
    for (name, d) in crate::common::credited_devices() {
        let t = export(&d, Format::Deluge).unwrap_or_else(|f| panic!("{name}: {f}"));
        assert_well_formed(&t.value);
        for (n, _) in labels_read(&t.value) {
            assert!(n < 120, "{name}");
        }
    }
}

#[test]
fn the_deluge_cannot_be_imported() {
    let f = import(FIRMWARE_DEFINITION, Format::Deluge, &crate::common::source()).unwrap_err();
    assert_eq!(f.kind, FaultKind::InvalidFile);
    let rule = f.rule.to_lowercase();
    assert!(rule.contains("deluge") && rule.contains("import"), "{}", f.rule);
}
