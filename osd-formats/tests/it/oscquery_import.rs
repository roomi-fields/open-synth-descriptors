//! An OSCQuery namespace imports into an OSD device descriptor `osd` reads strictly, carrying the
//! source the caller gives; a JSON that is not a namespace of the proposal is refused with its
//! JSON pointer.

use crate::common::electra::json_of;
use crate::common::midnam::lost_at;
use crate::common::oscquery::*;
use osd_formats::{import, FaultKind, Format};
use serde_json::{json, Value};

fn sample() -> Value {
    namespace(vec![
        (
            "filter",
            container(
                "/synth/filter",
                vec![
                    ("cutoff", method("/synth/filter/cutoff", "i", &[Some((0, 127))])),
                    ("mode", json!({"FULL_PATH": "/synth/filter/mode", "ACCESS": 2, "TYPE": "s", "EXTENDED_TYPE": ["mode"]})),
                ],
            ),
        ),
        (
            "mix",
            json!({"FULL_PATH": "/synth/mix", "ACCESS": 2, "TYPE": "ifsT", "EXTENDED_TYPE": ["level", "pan", "label", "mute"],
                "RANGE": [{"MIN": 0, "MAX": 10}, {"MIN": -1, "MAX": 1}, null, null]}),
        ),
        ("panic", json!({"FULL_PATH": "/synth/panic", "ACCESS": 2})),
    ])
}

fn refused(ns: &Value) -> osd_formats::Fault {
    import(&ns.to_string(), Format::OscQuery, &source()).expect_err("the namespace is refused")
}

fn command<'a>(d: &'a Value, osc: &str) -> &'a Value {
    d["commands"].as_array().unwrap().iter().find(|c| c["osc"] == osc).unwrap_or_else(|| panic!("no command {osc}: {d}"))
}

#[test]
fn the_import_reads_strictly_with_the_prefix_the_name_and_the_source_given() {
    let (d, _) = imported(&sample());
    assert_eq!(d["device"]["osc_prefix"], "/synth");
    assert_eq!(d["device"]["name"], "synth");
    assert_eq!(d["_sources"].as_array().unwrap().len(), 1);
    let src = source();
    assert_eq!(d["_sources"][0]["license"], src.license.clone().unwrap());
    assert_eq!(d["_sources"][0]["url"], src.url.clone().unwrap());
    assert_eq!(d["_sources"][0]["author"], src.author.clone().unwrap());
    assert_eq!(d["$schema"], "https://w3id.org/osd/0.6/device.schema.json");
}

#[test]
fn each_writable_method_becomes_a_command_at_its_address_below_the_prefix_with_its_arguments() {
    let (d, _) = imported(&sample());
    assert_eq!(d["commands"].as_array().unwrap().len(), 4);
    let cutoff = command(&d, "/filter/cutoff");
    assert_eq!(cutoff["args"][0]["type"], "int");
    assert_eq!(cutoff["args"][0]["range"], json!([0, 127]));
    let mix = command(&d, "/mix");
    let types: Vec<&str> = mix["args"].as_array().unwrap().iter().map(|a| a["type"].as_str().unwrap()).collect();
    assert_eq!(types, vec!["int", "float", "string", "bool"]);
    let names: Vec<&str> = mix["args"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["level", "pan", "label", "mute"]);
    assert_eq!(mix["args"][0]["range"], json!([0, 10]));
    assert_eq!(mix["args"][1]["range"], json!([-1, 1]));
    assert!(mix["args"][2].get("range").is_none() && mix["args"][3].get("range").is_none(), "{mix}");
    let panic = command(&d, "/panic");
    assert!(panic.get("args").is_none_or(|a| a.as_array().unwrap().is_empty()), "{panic}");
    assert!(panic.get("frame").is_none() && panic.get("forward").is_none(), "nothing is invented for what the command sends: {panic}");
}

#[test]
fn a_false_tag_is_a_boolean_argument_too() {
    let ns = namespace(vec![("flag", json!({"FULL_PATH": "/synth/flag", "ACCESS": 2, "TYPE": "F", "EXTENDED_TYPE": ["on"]}))]);
    let (d, _) = imported(&ns);
    assert_eq!(command(&d, "/flag")["args"][0]["type"], "bool");
}

#[test]
fn a_container_alone_is_not_a_command_and_a_namespace_with_no_method_gives_a_device_without_commands() {
    let ns = namespace(vec![("filter", container("/synth/filter", vec![]))]);
    let (d, _) = imported(&ns);
    assert!(d.get("commands").is_none_or(|c| c.as_array().unwrap().is_empty()), "{d}");
}

#[test]
fn a_read_only_method_is_lost_with_its_pointer_and_is_not_a_command() {
    let ns = namespace(vec![
        ("status", json!({"FULL_PATH": "/synth/status", "ACCESS": 1, "TYPE": "s", "EXTENDED_TYPE": ["text"]})),
        ("cutoff", method("/synth/cutoff", "i", &[Some((0, 127))])),
    ]);
    let (d, lost) = imported(&ns);
    assert_eq!(d["commands"].as_array().unwrap().len(), 1);
    assert_eq!(lost_at(&lost, "/CONTENTS/synth/CONTENTS/status").len(), 1, "{lost:?}");
}

#[test]
fn a_read_and_write_method_is_a_command_and_its_read_side_is_lost_at_its_access() {
    let ns = namespace(vec![("both", json!({"FULL_PATH": "/synth/both", "ACCESS": 3, "TYPE": "i"}))]);
    let (d, lost) = imported(&ns);
    assert_eq!(command(&d, "/both")["args"][0]["type"], "int");
    assert_eq!(lost_at(&lost, "/CONTENTS/synth/CONTENTS/both/ACCESS").len(), 1, "{lost:?}");
}

#[test]
fn each_attribute_the_descriptor_cannot_hold_has_a_loss_at_its_pointer() {
    let node = json!({
        "FULL_PATH": "/synth/x", "ACCESS": 2, "TYPE": "i", "RANGE": [{"MIN": 0, "MAX": 3, "VALS": ["a", "b"]}],
        "DESCRIPTION": "A knob", "TAGS": ["t"], "UNIT": "dB", "CRITICAL": true, "CLIPMODE": "both", "VALUE": [1], "X_EXTENSION": 5,
    });
    let (d, lost) = imported(&namespace(vec![("x", node)]));
    let base = "/CONTENTS/synth/CONTENTS/x";
    for attr in ["DESCRIPTION", "TAGS", "UNIT", "CRITICAL", "CLIPMODE", "VALUE", "X_EXTENSION", "RANGE/0/VALS"] {
        let found = lost_at(&lost, &format!("{base}/{attr}"));
        assert_eq!(found.len(), 1, "{attr}: {lost:?}");
        assert!(!found[0].reason.is_empty());
    }
    assert!(lost_at(&lost, &format!("{base}/RANGE")).is_empty() && lost_at(&lost, &format!("{base}/TYPE")).is_empty(), "{lost:?}");
    assert_eq!(command(&d, "/x")["args"][0]["range"], json!([0, 3]));
}

#[test]
fn the_description_of_the_device_node_is_lost_and_the_name_is_the_prefix_not_the_description() {
    let mut ns = sample();
    ns["CONTENTS"]["synth"]["DESCRIPTION"] = json!("Trundler by Lofty. Source: Dee");
    let (d, lost) = imported(&ns);
    assert_eq!(d["device"]["name"], "synth");
    assert_eq!(lost_at(&lost, "/CONTENTS/synth/DESCRIPTION").len(), 1, "{lost:?}");
    assert!(!d.to_string().contains("Lofty"), "{d}");
}

#[test]
fn a_source_without_a_credit_or_without_a_licence_is_refused() {
    let mut no_license = source();
    no_license.license = None;
    assert_eq!(import(&sample().to_string(), Format::OscQuery, &no_license).unwrap_err().kind, FaultKind::SourceIncomplete);
    let mut no_credit = source();
    (no_credit.url, no_credit.author, no_credit.attribution) = (None, None, None);
    assert_eq!(import(&sample().to_string(), Format::OscQuery, &no_credit).unwrap_err().kind, FaultKind::SourceIncomplete);
}

#[test]
fn a_text_that_is_not_json_and_a_json_that_is_not_a_node_are_refused() {
    let f = import("{not json", Format::OscQuery, &source()).unwrap_err();
    assert_eq!(f.kind, FaultKind::InvalidFile);
    for (what, doc) in [("an array", "[1, 2]"), ("a number", "3"), ("a string", "\"/\"")] {
        let f = import(doc, Format::OscQuery, &source()).unwrap_err();
        assert_eq!(f.kind, FaultKind::InvalidFile, "{what}");
        assert_eq!(f.place, "", "{what}: the root is the pointer \"\"");
    }
}

#[test]
fn a_namespace_that_breaks_the_proposal_is_refused_at_the_pointer_of_the_value() {
    // (what, the pointer of the value altered, its new value; null removes it, the place refused)
    let cases: Vec<(&str, &str, Value)> = vec![
        ("no root FULL_PATH", "/FULL_PATH", Value::Null),
        ("a root FULL_PATH that is not /", "/FULL_PATH", json!("/x")),
        ("a child FULL_PATH that is not its path", "/CONTENTS/synth/FULL_PATH", json!("/other")),
        ("an ACCESS above 3", "/CONTENTS/synth/CONTENTS/mix/ACCESS", json!(4)),
        ("an ACCESS that is not an integer", "/CONTENTS/synth/CONTENTS/mix/ACCESS", json!("rw")),
        ("a TYPE with a letter that is no tag", "/CONTENTS/synth/CONTENTS/mix/TYPE", json!("iq")),
        ("a TYPE that is not a string", "/CONTENTS/synth/CONTENTS/mix/TYPE", json!(5)),
        ("a RANGE of another length than TYPE", "/CONTENTS/synth/CONTENTS/mix/RANGE", json!([null])),
        ("a RANGE with MIN above MAX", "/CONTENTS/synth/CONTENTS/mix/RANGE/0", json!({"MIN": 5, "MAX": 1})),
        ("an EXTENDED_TYPE of another length than TYPE", "/CONTENTS/synth/CONTENTS/mix/EXTENDED_TYPE", json!(["a"])),
        ("CONTENTS that is not an object", "/CONTENTS/synth/CONTENTS", json!([])),
        ("a child that is not an object", "/CONTENTS/synth/CONTENTS/mix", json!(7)),
    ];
    for (what, pointer, value) in cases {
        let mut ns = sample();
        if value.is_null() {
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            let holder = if parent.is_empty() { &mut ns } else { ns.pointer_mut(parent).unwrap() };
            holder.as_object_mut().unwrap().remove(key);
        } else {
            *ns.pointer_mut(pointer).unwrap() = value;
        }
        let f = refused(&ns);
        assert_eq!(f.kind, FaultKind::InvalidFile, "{what}: {f}");
        assert_eq!(f.place, pointer, "{what}: {f}");
        assert!(!f.rule.is_empty(), "{what}");
    }
}

#[test]
fn a_namespace_with_two_top_level_containers_or_a_method_at_the_root_has_no_one_prefix_and_is_refused() {
    let mut two = sample();
    two["CONTENTS"]["other"] = container("/other", vec![("a", method("/other/a", "i", &[]))]);
    let f = refused(&two);
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/CONTENTS"), "{f}");
    let at_root = container("/", vec![("a", method("/a", "i", &[]))]);
    let f = refused(&at_root);
    assert_eq!((f.kind, f.place.as_str()), (FaultKind::InvalidFile, "/CONTENTS"), "{f}");
    let empty = json!({"FULL_PATH": "/", "ACCESS": 0});
    assert_eq!(refused(&empty).kind, FaultKind::InvalidFile);
}

#[test]
fn what_the_namespace_does_not_say_is_not_invented() {
    let (d, _) = imported(&sample());
    for key in ["identity", "sysex", "cc_params", "params", "midi_out", "midi_in", "programs", "triggers", "replies"] {
        assert!(d.get(key).is_none(), "{key} is invented: {d}");
    }
    for key in ["vendor", "revision", "manufacturer_id", "kind", "transport"] {
        assert!(d["device"].get(key).is_none(), "{key} is invented: {}", d["device"]);
    }
}

#[test]
fn the_import_is_read_strictly() {
    let t = import(&sample().to_string(), Format::OscQuery, &source()).unwrap();
    let _ = json_of(&t.value);
}
