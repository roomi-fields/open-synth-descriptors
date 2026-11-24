//! An OSD device exports to an OSCQuery namespace valid by the proposal (checked by
//! common/oscquery.rs), with what a namespace cannot carry in the losses.

use crate::common::midnam::{device_with, lost_at};
use crate::common::oscquery::*;
use crate::common::{credited_devices, device_from, resolved};
use osd_formats::{export, FaultKind, Format};
use serde_json::{json, Value};

fn ns_of(extra: Value) -> (Value, Vec<osd_formats::Loss>) {
    exported(&device_with(extra))
}

#[test]
fn the_export_is_a_namespace_valid_by_the_proposal_with_the_device_under_its_prefix() {
    let (ns, _) = ns_of(json!({"cc_params": {"entries": [{"osc": "/filter/cutoff", "cc": 74, "range": [0, 127]}]}}));
    assert_eq!(ns["FULL_PATH"], "/");
    assert_eq!(ns["ACCESS"], 0);
    let dev = node_at(&ns, "/trundler").expect("the device node");
    assert_eq!(dev["FULL_PATH"], "/trundler");
    assert_eq!(dev["ACCESS"], 0);
    assert!(node_at(&ns, "/trundler/filter").is_some_and(|n| n["ACCESS"] == 0 && n.get("TYPE").is_none()), "{ns}");
}

#[test]
fn a_device_with_nothing_to_address_exports_a_valid_namespace() {
    let (ns, _) = ns_of(json!({}));
    assert_eq!(violations(&ns.to_string()), Vec::<String>::new());
    assert!(methods(&ns).is_empty());
}

#[test]
fn every_device_of_the_conformance_cases_and_the_examples_that_names_a_source_exports_a_valid_namespace() {
    let devices = credited_devices();
    assert!(devices.len() >= 4);
    for (name, d) in devices {
        let t = export(&d, Format::OscQuery).unwrap_or_else(|f| panic!("{name}: {f}"));
        assert_eq!(violations(&t.value), Vec::<String>::new(), "{name}\n{}", t.value);
    }
}

#[test]
fn a_device_without_a_source_is_not_exported() {
    let f = export(&device_with(json!({"_sources": null})), Format::OscQuery).unwrap_err();
    assert_eq!(f.kind, FaultKind::NoSources);
}

#[test]
fn a_prefix_of_two_segments_is_two_nested_containers() {
    let (ns, _) = ns_of(json!({"device": {"name": "T", "osc_prefix": "/rack/trundler"}, "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}));
    assert_eq!(node_at(&ns, "/rack").unwrap()["ACCESS"], 0);
    assert_eq!(node_at(&ns, "/rack/trundler/a").unwrap()["FULL_PATH"], "/rack/trundler/a");
}

#[test]
fn a_control_is_a_writable_integer_method_at_its_address_with_its_range() {
    let (ns, _) = ns_of(json!({"cc_params": {"entries": [
        {"osc": "/filter/cutoff", "cc": 74, "range": [10, 100]},
        {"osc": "/filter/resonance", "cc": 71},
    ]}}));
    let cutoff = node_at(&ns, "/trundler/filter/cutoff").unwrap();
    assert_eq!(cutoff["ACCESS"], 2);
    assert_eq!(cutoff["TYPE"], "i");
    assert_eq!(cutoff["RANGE"], json!([{"MIN": 10, "MAX": 100}]));
    let resonance = node_at(&ns, "/trundler/filter/resonance").unwrap();
    assert_eq!(resonance["TYPE"], "i");
    assert_eq!(resonance["RANGE"], json!([{"MIN": 0, "MAX": 127}]), "the bounds MIDI fixes for a 7-bit controller");
}

#[test]
fn a_sysex_parameter_is_a_writable_integer_method_at_its_address_with_its_range() {
    let (ns, _) = ns_of(json!({"params": {"get_frame": [240, 1, 247], "set_frame": [240, 2, 247], "value_type": "u7",
        "entries": [{"osc": "/p/one", "pr": 1, "p": 2, "c": 3, "r": 4, "range": [0, 63]}]}}));
    let p = node_at(&ns, "/trundler/p/one").unwrap();
    assert_eq!((p["ACCESS"].clone(), p["TYPE"].clone(), p["RANGE"].clone()), (json!(2), json!("i"), json!([{"MIN": 0, "MAX": 63}])));
}

#[test]
fn a_command_is_a_method_with_the_types_and_the_names_of_its_arguments() {
    let (ns, _) = ns_of(json!({"commands": [
        {"osc": "/mix", "args": [
            {"name": "level", "type": "int", "range": [0, 10]},
            {"name": "pan", "type": "float", "range": [-1, 1]},
            {"name": "label", "type": "string"},
            {"name": "mute", "type": "bool"},
        ], "frame": [240, 247]},
        {"osc": "/panic", "frame": [240, 247]},
    ]}));
    let mix = node_at(&ns, "/trundler/mix").unwrap();
    assert_eq!(mix["ACCESS"], 2);
    assert_eq!(mix["TYPE"], "ifsT");
    assert_eq!(mix["EXTENDED_TYPE"], json!(["level", "pan", "label", "mute"]));
    assert_eq!(mix["RANGE"][0], json!({"MIN": 0, "MAX": 10}));
    assert_eq!(mix["RANGE"][1], json!({"MIN": -1, "MAX": 1}));
    assert!(mix["RANGE"][2].is_null() && mix["RANGE"][3].is_null(), "{mix}");
    let alarm = node_at(&ns, "/trundler/panic").unwrap();
    assert_eq!(alarm["ACCESS"], 2);
    assert!(alarm.get("TYPE").is_none_or(|t| *t == ""), "{alarm}");
}

#[test]
fn the_frame_of_a_command_is_lost_with_its_place() {
    let (_, lost) = ns_of(json!({"commands": [{"osc": "/panic", "frame": [240, 247]}, {"osc": "/other", "frame": [240, 1, 247]}]}));
    for place in ["/commands/0/frame", "/commands/1/frame"] {
        assert_eq!(lost_at(&lost, place).len(), 1, "{place}: {lost:?}");
    }
}

#[test]
fn the_integer_argument_types_are_all_the_integer_tag_and_an_enum_is_a_string() {
    let (ns, _) = ns_of(json!({"commands": [
        {"osc": "/n", "args": [{"name": "a", "type": "u7"}, {"name": "b", "type": "u14"}, {"name": "c", "type": "s32"}, {"name": "d", "type": "enum", "values": {"x": 0, "y": 1}}], "frame": [240, 247]},
    ]}));
    assert_eq!(node_at(&ns, "/trundler/n").unwrap()["TYPE"], "iiis");
}

#[test]
fn the_device_node_describes_the_device_and_credits_its_sources_with_their_licence() {
    let (ns, _) = ns_of(json!({}));
    let text = node_at(&ns, "/trundler").unwrap()["DESCRIPTION"].as_str().expect("a description").to_string();
    for word in ["Trundler", "Dee", "CC-BY-4.0", "https://example.org/d"] {
        assert!(text.contains(word), "{word} is not in {text:?}");
    }
}

#[test]
fn nothing_is_in_the_namespace_that_the_descriptor_does_not_hold() {
    let (ns, _) = ns_of(json!({"cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127]}]}}));
    let all = methods(&ns);
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].0, "/trundler/a");
    assert!(!ns.to_string().contains("VALUE") && !ns.to_string().contains("HOST_INFO"), "{ns}");
}

fn full_device() -> Value {
    json!({
        "identity": {"request": [240, 126, 127, 6, 1, 247]},
        "sysex": {"header": [240, 1], "footer": [247]},
        "midi_out": {"default_channel": 3},
        "programs": {"program_change": true, "banks": [{"name": "Factory", "size": 3, "first_pc": 10, "first_number": 1}]},
        "triggers": {"entries": [{"osc": "/kick", "sound": "Kick", "note_range": [36, 36], "note": 36}]},
        "replies": [{"match_frame": [240, 1, 247], "emit_osc": "/r"}],
        "cc_params": {"entries": [{"osc": "/a", "cc": 1, "range": [0, 127], "section": "Filter", "expects": "switch", "orientation": "centered"}]},
    })
}

#[test]
fn each_element_the_format_cannot_carry_has_a_loss_with_its_place() {
    let (_, lost) = ns_of(full_device());
    for place in [
        "/identity",
        "/sysex",
        "/midi_out",
        "/programs",
        "/triggers",
        "/replies",
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
fn what_the_namespace_carries_is_not_in_the_losses() {
    let (_, lost) = ns_of(full_device());
    for place in ["/cc_params/entries/0", "/cc_params/entries/0/osc", "/cc_params/entries/0/range", "/cc_params/entries/0/cc", "/device/osc_prefix", "/_sources"] {
        assert!(lost_at(&lost, place).is_empty(), "{place} is carried: {lost:?}");
    }
}

#[test]
fn the_example_devices_export_and_every_loss_has_a_pointer_and_a_reason() {
    for (name, d) in credited_devices() {
        let t = export(&d, Format::OscQuery).unwrap();
        assert!(t.lost.iter().all(|l| l.place.starts_with('/') && !l.reason.is_empty()), "{name}: {:?}", t.lost);
    }
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/0.5/synth.device.json")).unwrap();
    let t = export(&resolved(device_from(&text)), Format::OscQuery).unwrap();
    let ns: Value = serde_json::from_str(&t.value).unwrap();
    assert!(node_at(&ns, "/test_synth/filter/cutoff").is_some(), "{ns}");
    assert!(t.lost.iter().any(|l| l.place == "/triggers"), "{:?}", t.lost);
}

fn capture_command(osc: Value, args: Value) -> Value {
    json!({"osc": osc, "args": args, "frame": [240, 247]})
}

#[test]
fn a_command_whose_segment_is_a_capture_is_written_with_its_parents_and_the_capture_kept_in_the_segment() {
    let (ns, lost) = ns_of(json!({"commands": [
        capture_command(json!(["/track/", {"capture": "n"}, "/volume"]), json!([{"name": "value", "type": "float", "range": [0, 1]}])),
        capture_command(json!(["/scene/", {"capture": "n"}, "/fire"]), json!([])),
    ]}));
    let volume = node_at(&ns, "/trundler/track/{n}/volume").unwrap_or_else(|| panic!("{ns}"));
    assert_eq!(volume["FULL_PATH"], "/trundler/track/{n}/volume");
    assert_eq!(volume["ACCESS"], 2);
    assert_eq!(volume["TYPE"], "f");
    assert_eq!(volume["EXTENDED_TYPE"], json!(["value"]));
    for parent in ["/trundler/track", "/trundler/track/{n}", "/trundler/scene", "/trundler/scene/{n}"] {
        assert!(node_at(&ns, parent).is_some_and(|n| n["ACCESS"] == 0), "{parent}: {ns}");
    }
    assert!(node_at(&ns, "/trundler/scene/{n}/fire").is_some_and(|n| n["ACCESS"] == 2), "{ns}");
    assert!(lost.iter().all(|l| !l.reason.contains("has a capture")), "{lost:?}");
}

#[test]
fn a_command_with_several_captures_or_a_capture_inside_a_text_segment_keeps_each_capture_as_it_is() {
    let (ns, lost) = ns_of(json!({"commands": [
        capture_command(json!(["/track/", {"capture": "n"}, "/send/", {"capture": "m"}]), json!([{"name": "level", "type": "int", "range": [0, 10]}])),
        capture_command(json!(["/bank", {"capture": "k"}, "/go"]), json!([])),
    ]}));
    let send = node_at(&ns, "/trundler/track/{n}/send/{m}").unwrap_or_else(|| panic!("{ns}"));
    assert_eq!(send["TYPE"], "i");
    assert_eq!(send["RANGE"], json!([{"MIN": 0, "MAX": 10}]));
    assert!(node_at(&ns, "/trundler/track/{n}/send").is_some_and(|n| n["ACCESS"] == 0), "{ns}");
    assert!(node_at(&ns, "/trundler/bank{k}/go").is_some_and(|n| n["ACCESS"] == 2), "{ns}");
    assert!(lost.iter().all(|l| !l.reason.contains("has a capture")), "{lost:?}");
}

#[test]
fn a_command_with_a_capture_that_the_namespace_cannot_hold_is_named_in_the_losses_with_its_place() {
    let (ns, lost) = ns_of(json!({"commands": [
        capture_command(json!(["/track/", {"capture": "n"}]), json!([])),
        capture_command(json!(["/track/", {"capture": "n"}]), json!([])),
        capture_command(json!(["/track/", {"capture": "n"}, "/volume"]), json!([])),
    ]}));
    assert_eq!(methods(&ns).len(), 1, "{ns}");
    assert_eq!(lost_at(&lost, "/commands/1").len(), 1, "{lost:?}");
    assert_eq!(lost_at(&lost, "/commands/2").len(), 1, "{lost:?}");
}
