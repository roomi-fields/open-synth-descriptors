//! An OSD device exports to a MIDNAM file valid by the DTD of the MIDI Association (checked by
//! the verifier of common/midnam.rs), with what MIDNAM cannot carry in the losses.

use crate::common::midnam::*;
use crate::common::xml::Node;
use crate::common::{case_devices, credited_devices, device_from, resolved};
use osd_formats::{export, FaultKind, Format};
use serde_json::json;

fn controls_of(root: &Node) -> Vec<&Node> {
    root.all("Control")
}

#[test]
fn the_export_is_a_document_valid_by_the_dtd_with_the_public_document_type() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 74, "range": [0, 127]}]}}));
    let (text, _) = exported(&d);
    assert_eq!(violations(&text), Vec::<String>::new(), "{text}");
    assert!(text.contains(PUBLIC_ID), "{text}");
    assert!(text.contains("http://www.midi.org/dtds/MIDINameDocument10.dtd"), "{text}");
    assert!(text.trim_start().starts_with("<?xml"), "{text}");
}

#[test]
fn a_device_with_no_control_no_trigger_and_no_program_exports_a_valid_document() {
    let (text, _) = exported(&device_with(json!({})));
    assert_eq!(violations(&text), Vec::<String>::new(), "{text}");
}

#[test]
fn every_device_of_the_conformance_cases_and_the_examples_that_names_a_source_exports_a_valid_document() {
    let devices = credited_devices();
    assert!(devices.len() >= 4, "{} devices with a source in the cases", devices.len());
    for (name, d) in devices {
        let t = export(&d, Format::Midnam).unwrap_or_else(|f| panic!("{name}: {f}"));
        assert_eq!(violations(&t.value), Vec::<String>::new(), "{name}\n{}", t.value);
    }
}

#[test]
fn a_device_without_a_source_is_not_exported() {
    let d = device_with(json!({"_sources": null}));
    let f = export(&d, Format::Midnam).unwrap_err();
    assert_eq!(f.kind, FaultKind::NoSources);
    let mut empty = device_with(json!({}));
    empty.device.sources = Some(vec![]);
    assert_eq!(export(&empty, Format::Midnam).unwrap_err().kind, FaultKind::NoSources);
}

#[test]
fn the_manufacturer_and_the_model_are_the_vendor_and_the_name_of_the_device() {
    let root = parsed(&exported(&device_with(json!({}))).0);
    let master = root.kid("MasterDeviceNames").unwrap();
    assert_eq!(master.kid("Manufacturer").unwrap().text, "Lofty");
    assert_eq!(master.kids("Model").len(), 1);
    assert_eq!(master.kid("Model").unwrap().text, "Trundler");
}

#[test]
fn the_author_carries_the_credit_and_the_licence_of_every_source() {
    let d = device_with(json!({"_sources": [
        {"type": "community-re", "url": "https://example.org/one", "author": "Dee", "license": "CC-BY-4.0"},
        {"type": "vendor-doc", "attribution": "The Maker", "license": "CC0-1.0"},
    ]}));
    let root = parsed(&exported(&d).0);
    let author = &root.kid("Author").unwrap().text;
    for word in ["Dee", "CC-BY-4.0", "https://example.org/one", "The Maker", "CC0-1.0"] {
        assert!(author.contains(word), "{word} is not in the author: {author:?}");
    }
}

#[test]
fn a_device_without_a_vendor_has_an_empty_manufacturer_and_the_loss_says_so_nothing_is_invented() {
    let d = device_with(json!({"device": {"name": "Trundler", "osc_prefix": "/trundler"}}));
    let (text, lost) = exported(&d);
    let root = parsed(&text);
    assert_eq!(root.all("Manufacturer")[0].text, "");
    assert_eq!(lost_at(&lost, "/device/vendor").len(), 1, "{lost:?}");
}

#[test]
fn each_kind_of_control_is_written_with_its_type_and_number() {
    let entries = json!([
        {"osc": "/a", "cc": 74, "range": [0, 127]},
        {"osc": "/b", "cc": 5, "cc_lsb": 37, "range": [0, 16383]},
        {"osc": "/c", "nrpn_msb": 2, "nrpn_lsb": 44, "value_bits": 14, "range": [0, 16383]},
        {"osc": "/d", "rpn_msb": 1, "rpn_lsb": 2, "value_bits": 14, "range": [0, 16383]},
    ]);
    let root = parsed(&exported(&device_with(json!({"cc_params": {"entries": entries}}))).0);
    let got: Vec<(String, String)> =
        controls_of(&root).iter().map(|c| (c.attr("Type").unwrap_or("7bit").to_string(), c.attr("Number").unwrap().to_string())).collect();
    let want: Vec<(String, String)> = [("7bit", "74"), ("14bit", "5"), ("NRPN", "300"), ("RPN", "130")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    assert_eq!(got, want);
}

#[test]
fn the_values_of_a_control_are_its_range_and_its_default() {
    let entries = json!([{"osc": "/a", "cc": 1, "range": [10, 100], "default": 64}, {"osc": "/b", "cc": 2, "range": [0, 127]}]);
    let root = parsed(&exported(&device_with(json!({"cc_params": {"entries": entries}}))).0);
    let values = |n: usize| controls_of(&root)[n].kid("Values").unwrap().clone();
    assert_eq!((values(0).attr("Min"), values(0).attr("Max"), values(0).attr("Default")), (Some("10"), Some("100"), Some("64")));
    assert_eq!((values(1).attr("Min"), values(1).attr("Max"), values(1).attr("Default")), (Some("0"), Some("127"), None));
}

#[test]
fn a_control_without_a_range_has_the_span_of_its_type_and_nothing_else_is_invented() {
    let entries = json!([
        {"osc": "/a", "cc": 1},
        {"osc": "/b", "cc": 2, "cc_lsb": 34},
        {"osc": "/c", "nrpn_msb": 1, "nrpn_lsb": 1, "value_bits": 14},
        {"osc": "/d", "rpn_msb": 0, "rpn_lsb": 0, "value_bits": 14},
    ]);
    let root = parsed(&exported(&device_with(json!({"cc_params": {"entries": entries}}))).0);
    let spans: Vec<(String, String, bool)> = controls_of(&root)
        .iter()
        .map(|c| {
            let v = c.kid("Values").unwrap();
            (v.attr("Min").unwrap().to_string(), v.attr("Max").unwrap().to_string(), v.attr("Default").is_some())
        })
        .collect();
    let want = |max: &str| ("0".to_string(), max.to_string(), false);
    assert_eq!(spans, vec![want("127"), want("16383"), want("16383"), want("16383")]);
}

#[test]
fn the_name_of_a_control_is_its_remark_name_or_else_the_words_of_its_address() {
    let doc = json!({
        "cc_params": {"entries": [
            {"osc": "/filter/cutoff", "cc": 74, "range": [0, 127]},
            {"osc": "/amp_env/attack", "cc": 73, "range": [0, 127], "_name": "Attack time"},
        ]}
    });
    let root = parsed(&exported(&device_with(doc)).0);
    let names: Vec<&str> = controls_of(&root).iter().map(|c| c.attr("Name").unwrap()).collect();
    assert_eq!(names, vec!["Filter Cutoff", "Attack time"]);
}

#[test]
fn the_text_of_a_name_with_xml_signs_is_escaped_and_read_back_as_it_was() {
    let doc = json!({"device": {"name": "A & B <mk2>", "vendor": "Lofty \"Co\"", "osc_prefix": "/ab"},
        "cc_params": {"entries": [{"osc": "/x", "cc": 1, "range": [0, 127], "_name": "Cut <off> & \"res\""}]}});
    let root = parsed(&exported(&device_with(doc)).0);
    assert_eq!(root.kid("MasterDeviceNames").unwrap().kid("Model").unwrap().text, "A & B <mk2>");
    assert_eq!(root.kid("MasterDeviceNames").unwrap().kid("Manufacturer").unwrap().text, "Lofty \"Co\"");
    assert_eq!(controls_of(&root)[0].attr("Name"), Some("Cut <off> & \"res\""));
}

#[test]
fn the_channel_name_set_assignments_give_each_of_the_16_channels_a_name_set_the_document_declares() {
    let root = parsed(&exported(&device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}))).0);
    let sets: Vec<&str> = root.all("ChannelNameSet").iter().map(|s| s.attr("Name").unwrap()).collect();
    assert!(!sets.is_empty());
    let assigns = root.all("ChannelNameSetAssign");
    let channels: Vec<u32> = assigns.iter().map(|a| a.attr("Channel").unwrap().parse().unwrap()).collect();
    assert_eq!(channels, (1..=16).collect::<Vec<u32>>());
    for a in assigns {
        assert!(sets.contains(&a.attr("NameSet").unwrap()), "{:?} is no set of {sets:?}", a.attr("NameSet"));
    }
    for set in root.all("ChannelNameSet") {
        let uses = set.kid("UsesControlNameList").expect("a set uses the control names").attr("Name").unwrap();
        assert!(root.all("ControlNameList").iter().any(|l| l.attr("Name") == Some(uses)), "{uses}");
    }
}

#[test]
fn the_notes_of_the_triggers_are_a_note_name_list_the_channel_sets_use() {
    let triggers = json!({"entries": [
        {"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "note": 36},
        {"osc": "/snare", "sound": "Snare", "note_range": [38, 38], "note": 38},
    ]});
    let root = parsed(&exported(&device_with(json!({"triggers": triggers}))).0);
    let notes: Vec<(&str, &str)> = root.all("Note").iter().map(|n| (n.attr("Number").unwrap(), n.attr("Name").unwrap())).collect();
    assert_eq!(notes, vec![("36", "Kick"), ("38", "Snare")]);
    for set in root.all("ChannelNameSet") {
        let uses = set.kid("UsesNoteNameList").expect("a set uses the note names").attr("Name").unwrap();
        assert!(root.all("NoteNameList").iter().any(|l| l.attr("Name") == Some(uses)), "{uses}");
    }
}

#[test]
fn the_banks_of_the_programs_are_patch_banks_with_their_bank_select_and_program_changes() {
    let programs = json!({"program_change": true, "banks": [
        {"name": "Factory", "msb": 3, "lsb": 4, "size": 3, "first_pc": 10, "first_number": 1},
        {"name": "User", "size": 2, "first_pc": 0, "first_number": 0},
    ]});
    let root = parsed(&exported(&device_with(json!({"programs": programs}))).0);
    let banks = root.all("PatchBank");
    assert_eq!(banks.iter().map(|b| b.attr("Name").unwrap()).collect::<Vec<_>>(), vec!["Factory", "User"]);
    let selects: Vec<(&str, &str)> = banks[0].all("ControlChange").iter().map(|c| (c.attr("Control").unwrap(), c.attr("Value").unwrap())).collect();
    assert_eq!(selects, vec![("0", "3"), ("32", "4")]);
    assert!(banks[1].all("ControlChange").is_empty(), "a bank without select writes none");
    let patches: Vec<(&str, &str)> = banks[0].all("Patch").iter().map(|p| (p.attr("Number").unwrap(), p.attr("ProgramChange").unwrap())).collect();
    assert_eq!(patches, vec![("1", "10"), ("2", "11"), ("3", "12")]);
    let names: Vec<&str> = banks[0].all("Patch").iter().map(|p| p.attr("Name").unwrap()).collect();
    assert_eq!(names, vec!["Factory 1", "Factory 2", "Factory 3"], "a patch the descriptor does not name is named by its bank and number");
    assert_eq!(banks[1].all("Patch").len(), 2);
}

#[test]
fn a_device_with_no_bank_still_has_the_patch_bank_the_dtd_requires_and_it_holds_no_patch() {
    let root = parsed(&exported(&device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}))).0);
    for set in root.all("ChannelNameSet") {
        assert!(!set.kids("PatchBank").is_empty());
    }
    assert!(root.all("Patch").is_empty());
}

fn full_device() -> osd::Resolved {
    device_with(json!({
        "identity": {"request": [240, 126, 127, 6, 1, 247]},
        "sysex": {"header": [240, 1], "footer": [247]},
        "midi_out": {"default_channel": 3},
        "commands": [{"osc": "/dump", "frame": [240, 1, 247]}],
        "params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
            "entries": [{"osc": "/p/one", "pr": 1, "p": 2, "c": 3, "r": 4, "range": [0, 127]}]},
        "cc_params": {"entries": [
            {"osc": "/a", "cc": 1, "range": [0, 127], "section": "Filter", "expects": "switch", "orientation": "centered"},
        ]},
    }))
}

#[test]
fn each_element_the_format_cannot_carry_has_a_loss_with_its_place() {
    let (_, lost) = exported(&full_device());
    for place in [
        "/commands/0",
        "/params/entries/0",
        "/identity",
        "/sysex",
        "/midi_out",
        "/cc_params/entries/0/section",
        "/cc_params/entries/0/expects",
        "/cc_params/entries/0/orientation",
    ] {
        let found = lost_at(&lost, place);
        assert_eq!(found.len(), 1, "{place}: {lost:?}");
        assert!(!found[0].reason.is_empty(), "{place}");
    }
}

#[test]
fn a_device_of_controls_only_loses_its_osc_prefix_and_nothing_else() {
    let d = device_with(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}));
    let (_, lost) = exported(&d);
    let places: Vec<&str> = lost.iter().map(|l| l.place.as_str()).collect();
    assert_eq!(places, vec!["/device/osc_prefix"], "{lost:?}");
}

#[test]
fn what_is_carried_is_not_in_the_losses() {
    let (_, lost) = exported(&full_device());
    for place in ["/cc_params/entries/0", "/cc_params/entries/0/cc", "/cc_params/entries/0/range", "/device/name", "/device/vendor", "/_sources"] {
        assert!(lost_at(&lost, place).is_empty(), "{place} is carried: {lost:?}");
    }
}

#[test]
fn the_example_device_with_its_commands_and_triggers_exports_and_loses_its_commands() {
    let cases = case_devices();
    let (_, text) = cases.iter().find(|(n, _)| n.ends_with("test_synth.json")).unwrap();
    let d = resolved(device_from(text));
    let (out, lost) = exported(&d);
    assert!(violations(&out).is_empty());
    assert!(!lost.is_empty());
    assert!(lost.iter().all(|l| l.place.starts_with('/') && !l.reason.is_empty()), "{lost:?}");
}
