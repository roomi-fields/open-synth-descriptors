//! Capabilities (0.5). A device declares `capabilities` (a closed list, today
//! `lua`); the reader derives the list from the fields present and refuses a
//! declared list that differs, as a bad value at `/capabilities`. The Lua
//! fields are the osc-bridge ones of 0.4 and four fields that enter 0.5 with
//! the meaning osc-bridge gives them: `identity.script`,
//! `identity.firmware.script`, `programs.recall_script`, `programs.dump.script`.
//! `identity.to` (`every_port`, `named_ports`) and `programs.after` (a list of
//! messages) hold no Lua. `osd::convert` writes the derived list.

mod common;

use osd::{Capability, FaultKind, Kind};
use serde_json::{json, Value};

const SCHEMA: &str = "https://w3id.org/osd/0.5/device.schema.json";

/// A 0.5 device with `part` merged at its top level.
fn device(part: Value) -> Value {
    let mut doc = json!({"$schema": SCHEMA, "device": {"name": "T", "osc_prefix": "/t"}});
    for (k, v) in part.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

/// The five new or newly-derived Lua fields, each as a device without `capabilities`.
fn lua_devices() -> Vec<(&'static str, Value)> {
    vec![
        ("identity.script", device(json!({"identity": {"reply": [240, 126, "any", 6, 2], "script": "return reply[5] == 2"}}))),
        (
            "identity.firmware.script",
            device(json!({"identity": {"firmware": {"decode": "lua", "at": [5], "script": "return string.format('%d.%02d', reply[1] >> 4, reply[1] & 15)"}}})),
        ),
        ("programs.recall_script", device(json!({"programs": {"recall_script": "return {{0xC0, ctx.program}}"}}))),
        ("programs.dump.script", device(json!({"programs": {"dump": {"kind": "request", "script": "return {240, 1, ctx.program, 247}"}}}))),
        (
            "cc_params transform",
            device(json!({"cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "transform": "x * 2"}]}})),
        ),
    ]
}

/// The same device declaring `list`.
fn declaring(doc: &Value, list: Value) -> Value {
    let mut out = doc.clone();
    out["capabilities"] = list;
    out
}

#[track_caller]
fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Device)
}

#[track_caller]
fn accepted(doc: &Value) {
    let d = read(doc).unwrap_or_else(|f| panic!("{f} in {doc}"));
    assert_eq!(&serde_json::from_str::<Value>(&osd::write(&d)).unwrap(), doc);
    assert_eq!(common::Schemas::load("0.5").check(Kind::Device, doc), None, "{doc}");
}

#[track_caller]
fn refused_at(doc: &Value, at: &str) {
    let f = read(doc).expect_err(&format!("the reader accepts {doc}"));
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::BadValue, at), "{f}");
}

fn convert(doc: &Value) -> Value {
    serde_json::from_str(&osd::convert(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f}"))).unwrap()
}

// ---- criteria 1, 6, 10: the declared list equals the derived one ----

#[test]
fn criterion_1_and_6_each_lua_field_with_lua_declared_is_accepted() {
    for (_, doc) in lua_devices() {
        accepted(&declaring(&doc, json!(["lua"])));
    }
}

#[test]
fn criterion_6_each_lua_field_without_the_declaration_is_a_bad_value_at_the_list() {
    for (field, doc) in lua_devices() {
        if field == "identity.firmware.script" {
            continue; // criterion 5 states it with its own pointer
        }
        refused_at(&doc, "/capabilities");
        refused_at(&declaring(&doc, json!([])), "/capabilities");
    }
}

#[test]
fn criterion_10_lua_declared_without_a_lua_field_is_refused() {
    refused_at(&declaring(&device(json!({})), json!(["lua"])), "/capabilities");
    refused_at(&declaring(&device(json!({"programs": {"before": [[240, 247]]}})), json!(["lua"])), "/capabilities");
}

#[test]
fn criterion_10_an_empty_list_with_no_lua_field_and_no_list_are_accepted() {
    accepted(&declaring(&device(json!({})), json!([])));
    accepted(&device(json!({})));
}

#[test]
fn criterion_10_a_list_that_repeats_a_capability_differs_from_the_derived_one() {
    let (_, doc) = &lua_devices()[2];
    refused_at(&declaring(doc, json!(["lua", "lua"])), "/capabilities");
}

#[test]
fn criterion_1_the_reader_derives_the_list_of_a_resolved_device() {
    struct One(String);
    impl osd::Files for One {
        fn get(&self, id: &str) -> Option<String> {
            (id == "acme/t").then(|| self.0.clone())
        }
    }
    for (field, doc) in lua_devices() {
        let r = osd::resolve("acme/t", &One(declaring(&doc, json!(["lua"])).to_string())).unwrap_or_else(|f| panic!("{field}: {f}"));
        assert_eq!(r.capabilities, vec![Capability::Lua], "{field}");
    }
    let r = osd::resolve("acme/t", &One(device(json!({})).to_string())).unwrap();
    assert!(r.capabilities.is_empty());
}

// ---- criterion 9: the list is closed ----

#[test]
fn criterion_9_a_capability_outside_the_list_is_a_bad_value_at_the_value() {
    for word in ["wasm", "Lua", "lua ", "x-acme-wasm", ""] {
        let doc = declaring(&device(json!({})), json!([word]));
        refused_at(&doc, "/capabilities/0");
        let errors = common::Schemas::load("0.5").errors(Kind::Device, &doc);
        assert!(errors.iter().any(|(p, _)| p == "/capabilities/0"), "{word:?}: {errors:?}");
    }
    let (_, lua) = &lua_devices()[2];
    refused_at(&declaring(lua, json!(["lua", "wasm"])), "/capabilities/1");
}

// ---- criterion 5: decode lua requires the capability ----

#[test]
fn criterion_5_decode_lua_without_the_lua_capability_is_refused_by_the_schema_and_the_reader() {
    let (_, doc) = &lua_devices()[1];
    refused_at(doc, "/capabilities");
    refused_at(&declaring(doc, json!([])), "/capabilities");
    let errors = common::Schemas::load("0.5").errors(Kind::Device, doc);
    assert!(!errors.is_empty(), "the schema accepts decode lua without the capability");
    let errors = common::Schemas::load("0.5").errors(Kind::Device, &declaring(doc, json!([])));
    assert!(!errors.is_empty(), "the schema accepts decode lua with an empty list");
}

#[test]
fn criterion_5_decode_lua_with_the_capability_is_accepted() {
    let (_, doc) = &lua_devices()[1];
    accepted(&declaring(doc, json!(["lua"])));
}

// ---- criterion 2: the six fields of D3 ----

#[test]
fn criterion_2_identity_to_takes_every_port_and_named_ports_and_derives_no_capability() {
    accepted(&device(json!({"identity": {"to": "every_port"}})));
    accepted(&device(json!({"identity": {"request": [240, 125, 70, 76, 1, 247], "to": "named_ports", "ports": ["FM-1"]}})));
    accepted(&declaring(&device(json!({"identity": {"to": "named_ports", "ports": ["a", "b"]}})), json!([])));
}

#[test]
fn criterion_2_and_6_identity_to_derives_no_capability_so_lua_declared_with_it_alone_is_refused() {
    let doc = declaring(&device(json!({"identity": {"to": "named_ports", "ports": ["FM-1"]}})), json!(["lua"]));
    refused_at(&doc, "/capabilities");
}

#[test]
fn criterion_2_identity_to_outside_its_two_words_is_a_bad_value_at_the_value() {
    for word in ["all", "Every_port", "named_port", "", "x-acme-ports"] {
        let doc = device(json!({"identity": {"to": word}}));
        refused_at(&doc, "/identity/to");
        let errors = common::Schemas::load("0.5").errors(Kind::Device, &doc);
        assert!(errors.iter().any(|(p, _)| p.starts_with("/identity/to")), "{word:?}: {errors:?}");
    }
}

#[test]
fn criterion_2_a_script_is_a_string_at_each_new_field() {
    for (pointer, doc) in [
        ("/identity/script", device(json!({"identity": {"script": 3}}))),
        ("/programs/recall_script", device(json!({"programs": {"recall_script": ["x"]}}))),
        ("/programs/dump/script", device(json!({"programs": {"dump": {"kind": "request", "script": 3}}}))),
    ] {
        let f = read(&doc).expect_err(pointer);
        assert_eq!(f.pointer, pointer, "{f}");
        let errors = common::Schemas::load("0.5").errors(Kind::Device, &doc);
        assert!(errors.iter().any(|(p, _)| p.starts_with(pointer)), "{pointer}: {errors:?}");
    }
}

#[test]
fn criterion_2_dump_kind_none_request_edit_buffer_and_manual_are_accepted() {
    for kind in ["request", "edit_buffer", "manual", "none"] {
        accepted(&device(json!({"programs": {"dump": {"kind": kind}}})));
    }
    refused_at(&device(json!({"programs": {"dump": {"kind": "script"}}})), "/programs/dump/kind");
}

#[test]
fn criterion_7_programs_after_is_a_list_of_messages_like_before_and_derives_no_capability() {
    accepted(&device(json!({"programs": {"program_change": false, "before": [[240, 1, 247]], "after": [[240, 2, 247], [176, 7, 100]]}})));
    accepted(&declaring(&device(json!({"programs": {"after": [[240, 2, 247]]}})), json!([])));
    refused_at(&declaring(&device(json!({"programs": {"after": [[240, 2, 247]]}})), json!(["lua"])), "/capabilities");
}

#[test]
fn criterion_7_programs_after_is_not_a_script() {
    for after in [json!("return {}"), json!([240, 2, 247]), json!([[300]]), json!([["x"]])] {
        let doc = device(json!({"programs": {"after": after}}));
        let f = read(&doc).expect_err(&format!("accepted {doc}"));
        assert!(f.pointer.starts_with("/programs/after"), "{f}");
        let errors = common::Schemas::load("0.5").errors(Kind::Device, &doc);
        assert!(errors.iter().any(|(p, _)| p.starts_with("/programs/after")), "{errors:?}");
    }
}

// ---- criteria 3 and 8: the converter writes the derived list ----

#[test]
fn criterion_3_and_8_a_0_4_driver_with_a_lua_field_converts_with_capabilities_lua() {
    let cc = json!({"device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "transform": "x * 2"}]}});
    let reverse = json!({"device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "transform_reverse": "x // 2"}]}});
    for (doc, name) in [(cc, "transform"), (reverse, "transform_reverse")] {
        let out = convert(&doc);
        assert_eq!(out["capabilities"], json!(["lua"]), "{name}: {out}");
        accepted(&out);
    }
}

#[test]
fn criterion_8_a_0_4_driver_without_a_lua_field_converts_without_capabilities() {
    for doc in [
        json!({"device": {"name": "T", "osc_prefix": "/t"}}),
        json!({"device": {"name": "T", "osc_prefix": "/t"}, "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1}]}}),
    ] {
        let out = convert(&doc);
        assert!(out.get("capabilities").is_none(), "{out}");
    }
}

#[test]
fn criterion_8_the_converted_list_equals_the_list_the_reader_derives_from_the_converted_text() {
    let doc = json!({"device": {"name": "T", "osc_prefix": "/t"},
        "cc_params": {"channel": 0, "entries": [{"osc": "/a", "cc": 1, "transform": "x"}]}});
    let out = convert(&doc);
    let text = out.to_string();
    let resolved = osd::resolve("acme/t", &Single(text)).unwrap();
    assert_eq!(resolved.capabilities, vec![Capability::Lua]);
    assert_eq!(out["capabilities"], json!(["lua"]));
}

struct Single(String);
impl osd::Files for Single {
    fn get(&self, id: &str) -> Option<String> {
        (id == "acme/t").then(|| self.0.clone())
    }
}

// ---- criterion 4: the conformance cases exist ----

#[test]
fn criterion_4_a_conformance_case_exists_per_lua_field_per_new_field_and_for_a_differing_list() {
    let read = common::repo().join("conformance/0.5/read");
    let convert = common::repo().join("conformance/0.5/convert");
    let text = |dir: &std::path::Path| -> String {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .filter_map(|p| std::fs::read_to_string(p.join("descriptor.json")).ok())
            .map(|t| serde_json::from_str::<Value>(&t).unwrap().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let reads = text(&read);
    for needle in [
        "\"recall_script\"", "\"dump\":{", "\"to\":\"named_ports\"", "\"to\":\"every_port\"", "\"after\":", "\"capabilities\":[\"lua\"]",
        "\"script\":\"return reply", "\"decode\":\"lua\"", "\"script\":\"return {240",
    ] {
        assert!(reads.contains(needle), "no 0.5 reading case holds {needle}");
    }
    let faults = std::fs::read_dir(&read)
        .unwrap()
        .map(|e| e.unwrap().path().join("case.json"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter(|c| c.contains("\"/capabilities\"") && c.contains("BadValue"))
        .count();
    assert!(faults >= 1, "no reading case refuses a declared list that differs");
    assert!(text(&convert).contains("\"transform\""), "no conversion case holds a Lua field");
    let converted = std::fs::read_dir(&convert)
        .unwrap()
        .filter_map(|e| std::fs::read_to_string(e.unwrap().path().join("case.json")).ok())
        .filter(|c| c.contains("\"capabilities\""))
        .count();
    assert!(converted >= 1, "no conversion case writes capabilities");
}
