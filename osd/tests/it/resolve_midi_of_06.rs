//! What a file with `midi_of` may add and declare, by the version of its own text:
//! from 0.6 params entries, scripted commands and `capabilities`; 0.5 keeps its limit.

use crate::common;

use osd::{Capability, FaultKind, Kind};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const S6: &str = "https://w3id.org/osd/0.6/device.schema.json";
const S5: &str = "https://w3id.org/osd/0.5/device.schema.json";

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn frames() -> Value {
    json!({"get_frame": [240, 247], "set_frame": [240, 247], "value_type": "u7"})
}

fn with_entries(entries: Value) -> Value {
    let mut p = frames();
    p["entries"] = entries;
    p
}

fn device(schema: &str, extra: Value) -> Value {
    let mut doc = json!({"$schema": schema, "device": {"name": "Synth", "osc_prefix": "/synth"}});
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn script(osc: &str) -> Value {
    json!({"osc": osc, "script": "return {}"})
}

type Refused = (FaultKind, Option<String>, String);

fn refused(files: &BTreeMap<String, String>) -> Refused {
    let f = osd::resolve("acme/a", files).expect_err("should be refused");
    (f.kind, f.file, f.pointer)
}

fn at(kind: FaultKind, file: &str, pointer: &str) -> Refused {
    (kind, Some(file.to_string()), pointer.to_string())
}

fn ok(files: &BTreeMap<String, String>) -> osd::Resolved {
    osd::resolve("acme/a", files).unwrap_or_else(|f| panic!("{f}"))
}

fn entries_of(r: &osd::Resolved) -> Vec<String> {
    let doc: Value = serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device.clone())))).unwrap();
    doc["params"]["entries"].as_array().unwrap().iter().map(|e| e["osc"].as_str().unwrap().to_string()).collect()
}

fn doc_of(r: &osd::Resolved) -> Value {
    serde_json::from_str(&osd::write(&osd::Descriptor::Device(Box::new(r.device.clone())))).unwrap()
}

#[test]
fn a_0_6_midi_of_file_adds_params_entries_after_its_targets_and_takes_the_frames_from_it() {
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": {"entries": [{"osc": "/p3"}, {"osc": "/p4"}]}}))),
        ("acme/b", device(S6, json!({"params": with_entries(json!([{"osc": "/p1"}, {"osc": "/p2"}]))}))),
    ]);
    let r = ok(&files);
    assert_eq!(entries_of(&r), ["/p1", "/p2", "/p3", "/p4"]);
    let p = &doc_of(&r)["params"];
    assert_eq!((&p["get_frame"], &p["set_frame"], &p["value_type"]), (&json!([240, 247]), &json!([240, 247]), &json!("u7")));
}

#[test]
fn a_0_6_midi_of_params_entry_at_a_targets_address_wins_and_comes_last() {
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": {"entries": [{"osc": "/p1", "p": 9}]}}))),
        ("acme/b", device(S6, json!({"params": with_entries(json!([{"osc": "/p1"}, {"osc": "/p2"}]))}))),
    ]);
    assert_eq!(entries_of(&ok(&files)), ["/p2", "/p1"]);
}

#[test]
fn params_entries_added_down_a_chain_of_0_6_files_come_in_chain_order() {
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": {"entries": [{"osc": "/pa"}]}}))),
        ("acme/b", device(S6, json!({"midi_of": "acme/c", "params": {"entries": [{"osc": "/pb"}]}}))),
        ("acme/c", device(S6, json!({"params": with_entries(json!([{"osc": "/pc"}]))}))),
    ]);
    assert_eq!(entries_of(&ok(&files)), ["/pc", "/pb", "/pa"]);
}

#[test]
fn a_0_6_midi_of_file_gives_none_of_the_three_frame_keys_of_a_target_that_has_params() {
    for (key, value) in [("get_frame", json!([240, 247])), ("set_frame", json!([240, 247])), ("value_type", json!("u14"))] {
        let mut params = json!({"entries": [{"osc": "/p2"}]});
        params[key] = value;
        let files = catalogue(vec![
            ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": params}))),
            ("acme/b", device(S6, json!({"params": with_entries(json!([{"osc": "/p1"}]))}))),
        ]);
        assert_eq!(refused(&files), at(FaultKind::UnknownField, "acme/a", &format!("/params/{key}")), "{key}");
    }
}

#[test]
fn a_0_6_midi_of_params_table_taken_as_it_is_needs_each_of_the_three_frame_keys() {
    for missing in ["get_frame", "set_frame", "value_type"] {
        let mut params = with_entries(json!([{"osc": "/p2"}]));
        params.as_object_mut().unwrap().remove(missing);
        let files = catalogue(vec![
            ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": params}))),
            ("acme/b", device(S6, json!({}))),
        ]);
        assert_eq!(refused(&files), at(FaultKind::Missing, "acme/a", &format!("/params/{missing}")), "{missing}");
    }
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": with_entries(json!([{"osc": "/p2"}]))}))),
        ("acme/b", device(S6, json!({}))),
    ]);
    assert_eq!(entries_of(&ok(&files)), ["/p2"]);
}

#[test]
fn the_reader_reads_a_0_6_midi_of_file_with_params_entries_only_and_a_file_without_midi_of_still_needs_the_frames() {
    let alone = |s: &str, extra: Value| osd::read(&device(s, extra).to_string(), Kind::Device);
    assert!(alone(S6, json!({"midi_of": "acme/b", "params": {"entries": []}})).is_ok());
    assert!(alone(S6, json!({"midi_of": "acme/b", "params": with_entries(json!([]))})).is_ok());
    for missing in ["get_frame", "set_frame", "value_type"] {
        let mut params = with_entries(json!([]));
        params.as_object_mut().unwrap().remove(missing);
        let f = alone(S6, json!({"params": params})).expect_err("no midi_of");
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Missing, format!("/params/{missing}").as_str()), "{missing}");
    }
    // 0.5 is unchanged: its params needs the frames even with midi_of.
    let f = alone(S5, json!({"midi_of": "acme/b", "params": {"entries": []}})).expect_err("0.5");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::Missing, "/params/get_frame"));
}

#[test]
fn the_0_6_schema_asks_a_midi_of_file_for_params_entries_only_and_the_0_5_schema_asks_for_all_four() {
    let (s6, s5) = (common::Schemas::load("0.6"), common::Schemas::load("0.5"));
    let sharing = device(S6, json!({"midi_of": "acme/b", "params": {"entries": [{"osc": "/p"}]}}));
    assert!(s6.errors(Kind::Device, &sharing).is_empty(), "{:?}", s6.errors(Kind::Device, &sharing));
    let alone = device(S6, json!({"params": {"entries": [{"osc": "/p"}]}}));
    assert!(!s6.errors(Kind::Device, &alone).is_empty(), "a file without midi_of needs the frames");
    for missing in ["get_frame", "set_frame", "value_type"] {
        let mut params = with_entries(json!([]));
        params.as_object_mut().unwrap().remove(missing);
        let doc = device(S6, json!({"params": params}));
        assert!(!s6.errors(Kind::Device, &doc).is_empty(), "without midi_of, no {missing}");
    }
    let old = device(S5, json!({"midi_of": "acme/b", "params": {"entries": [{"osc": "/p"}]}}));
    assert!(!s5.errors(Kind::Device, &old).is_empty(), "0.5 still asks the frames");
}

#[test]
fn a_0_6_midi_of_file_declares_capabilities_and_the_merge_holds_the_capabilities_of_both_files() {
    // The target brings lua, the file declares none: lua is inherited.
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "capabilities": []}))),
        ("acme/b", device(S6, json!({"capabilities": ["lua"], "custom_commands": [script("/c1")]}))),
    ]);
    let r = ok(&files);
    assert_eq!((r.capabilities.clone(), r.inherited.clone()), (vec![Capability::Lua], vec![Capability::Lua]));
    assert_eq!(r.device.capabilities, Some(vec![]));
    // The file brings lua, the target does not: lua is the file's own, not inherited.
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "capabilities": ["lua"], "custom_commands": [script("/c1")]}))),
        ("acme/b", device(S6, json!({}))),
    ]);
    let r = ok(&files);
    assert_eq!((r.capabilities.clone(), r.inherited.clone()), (vec![Capability::Lua], vec![]));
    assert_eq!(r.device.capabilities, Some(vec![Capability::Lua]));
    // Neither needs lua: both declare none.
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "capabilities": []}))),
        ("acme/b", device(S6, json!({"capabilities": []}))),
    ]);
    let r = ok(&files);
    assert_eq!((r.capabilities, r.inherited), (vec![], vec![]));
}

#[test]
fn capabilities_brought_by_a_target_further_down_a_chain_are_inherited_by_each_0_6_file_above() {
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "capabilities": []}))),
        ("acme/b", device(S6, json!({"midi_of": "acme/c", "capabilities": []}))),
        ("acme/c", device(S6, json!({"capabilities": ["lua"], "custom_commands": [script("/c1")]}))),
    ]);
    let r = ok(&files);
    assert_eq!((r.capabilities, r.inherited), (vec![Capability::Lua], vec![Capability::Lua]));
}

#[test]
fn a_0_6_midi_of_file_that_adds_a_script_declares_lua_itself_even_over_a_target_that_has_lua() {
    let target = device(S6, json!({"capabilities": ["lua"], "custom_commands": [script("/c1")]}));
    for declared in [None, Some(json!([]))] {
        let mut a = device(S6, json!({"midi_of": "acme/b", "custom_commands": [script("/c2")]}));
        if let Some(d) = declared.clone() {
            a["capabilities"] = d;
        }
        let files = catalogue(vec![("acme/a", a), ("acme/b", target.clone())]);
        assert_eq!(refused(&files), at(FaultKind::BadValue, "acme/a", "/capabilities"), "{declared:?}");
    }
    let a = device(S6, json!({"midi_of": "acme/b", "capabilities": ["lua"], "custom_commands": [script("/c2")]}));
    let files = catalogue(vec![("acme/a", a), ("acme/b", target)]);
    let r = ok(&files);
    let doc = doc_of(&r);
    let osc: Vec<&str> = doc["custom_commands"].as_array().unwrap().iter().map(|c| c["osc"].as_str().unwrap()).collect();
    assert_eq!(osc, ["/c1", "/c2"]);
    assert!(r.inherited.is_empty());
}

#[test]
fn a_0_6_midi_of_params_entry_with_a_transform_needs_lua_declared_by_the_file() {
    let target = device(S6, json!({"params": with_entries(json!([{"osc": "/p1"}]))}));
    let entry = json!({"osc": "/p2", "transform": "return value"});
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "params": {"entries": [entry.clone()]}}))),
        ("acme/b", target.clone()),
    ]);
    assert_eq!(refused(&files), at(FaultKind::BadValue, "acme/a", "/capabilities"));
    let files = catalogue(vec![
        ("acme/a", device(S6, json!({"midi_of": "acme/b", "capabilities": ["lua"], "params": {"entries": [entry]}}))),
        ("acme/b", target),
    ]);
    assert_eq!(ok(&files).capabilities, vec![Capability::Lua]);
}

#[test]
fn a_0_6_midi_of_file_that_adds_no_script_and_declares_lua_is_refused_whatever_its_target_brings() {
    for target in [device(S6, json!({})), device(S6, json!({"capabilities": ["lua"], "custom_commands": [script("/c1")]}))] {
        let files = catalogue(vec![
            ("acme/a", device(S6, json!({"midi_of": "acme/b", "capabilities": ["lua"]}))),
            ("acme/b", target),
        ]);
        assert_eq!(refused(&files), at(FaultKind::BadValue, "acme/a", "/capabilities"));
    }
}

#[test]
fn a_0_6_midi_of_file_over_a_0_4_or_0_5_target_is_judged_by_the_rules_of_0_6() {
    let own = |extra: Value| {
        let mut base = json!({"midi_of": "acme/b", "capabilities": [], "params": {"entries": [{"osc": "/p2"}]}});
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        device(S6, base)
    };
    let t05 = device(S5, json!({"params": with_entries(json!([{"osc": "/p1"}]))}));
    let t04 = json!({"device": {"name": "Synth", "osc_prefix": "/synth"}, "params": with_entries(json!([{"osc": "/p1"}]))});
    for target in [t05, t04] {
        let files = catalogue(vec![("acme/a", own(json!({}))), ("acme/b", target)]);
        let r = ok(&files);
        assert_eq!(entries_of(&r), ["/p1", "/p2"]);
        assert_eq!(doc_of(&r)["$schema"], S6);
    }
    // A key of the target's is refused as in 0.6, over a 0.5 target too.
    let files = catalogue(vec![
        ("acme/a", own(json!({"params": with_entries(json!([]))}))),
        ("acme/b", device(S5, json!({"params": with_entries(json!([{"osc": "/p1"}]))}))),
    ]);
    assert_eq!(refused(&files), at(FaultKind::UnknownField, "acme/a", "/params/get_frame"));
}

#[test]
fn a_0_5_midi_of_file_over_a_0_6_target_keeps_the_limit_of_0_5() {
    let target = device(S6, json!({"capabilities": ["lua"], "custom_commands": [script("/c1")], "params": with_entries(json!([{"osc": "/p1"}]))}));
    for (extra, pointer) in [
        (json!({"capabilities": []}), "/capabilities"),
        (json!({"capabilities": ["lua"], "custom_commands": [script("/c2")]}), "/capabilities"),
        (json!({"params": with_entries(json!([{"osc": "/p2"}]))}), "/params/get_frame"),
    ] {
        let mut a = device(S5, json!({"midi_of": "acme/b"}));
        for (k, v) in extra.as_object().unwrap() {
            a[k] = v.clone();
        }
        let files = catalogue(vec![("acme/a", a), ("acme/b", target.clone())]);
        assert_eq!(refused(&files), at(FaultKind::UnknownField, "acme/a", pointer), "{extra}");
    }
}

#[test]
fn a_0_5_midi_of_file_over_a_0_5_target_keeps_the_limit_of_0_5() {
    for (extra, pointer) in [(json!({"capabilities": []}), "/capabilities"), (json!({"params": with_entries(json!([{"osc": "/p2"}]))}), "/params/get_frame")] {
        let mut a = device(S5, json!({"midi_of": "acme/b"}));
        for (k, v) in extra.as_object().unwrap() {
            a[k] = v.clone();
        }
        let files = catalogue(vec![("acme/a", a), ("acme/b", device(S5, json!({"params": with_entries(json!([{"osc": "/p1"}]))})))]);
        assert_eq!(refused(&files), at(FaultKind::UnknownField, "acme/a", pointer), "{extra}");
    }
}

#[test]
fn a_0_6_midi_of_file_holds_only_what_it_may_add() {
    let target = device(S6, json!({"params": with_entries(json!([{"osc": "/p1"}]))}));
    for (key, value) in [("sysex", json!({"header": [240]})), ("identity_x", json!(1)), ("loads_sounds_of", json!(["acme"]))] {
        let mut a = device(S6, json!({"midi_of": "acme/b"}));
        a[key] = value;
        let files = catalogue(vec![("acme/a", a), ("acme/b", target.clone())]);
        let got = refused(&files);
        assert_eq!((got.0, got.2.as_str()), (FaultKind::UnknownField, format!("/{key}").as_str()), "{key}");
    }
}
