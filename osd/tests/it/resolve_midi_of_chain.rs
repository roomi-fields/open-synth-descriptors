//! A chain of `midi_of` files of several versions: what each link may hold is judged on the text
//! it writes, in its own version (23.2, 23.9), whatever version the file that follows it asks for;
//! the credit of every link reaches `midi_of_sources` (23.13).

use osd::{Capability, Descriptor, FaultKind, Resolved};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const S7: &str = "https://w3id.org/osd/0.7/device.schema.json";
const S6: &str = "https://w3id.org/osd/0.6/device.schema.json";
const S5: &str = "https://w3id.org/osd/0.5/device.schema.json";

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

/// A device of the version `schema` names; an empty `schema` is a 0.4 one, which writes no `$schema`.
fn device(schema: &str, extra: Value) -> Value {
    let mut doc = json!({"device": {"name": "Synth", "osc_prefix": "/synth"}});
    if !schema.is_empty() {
        doc["$schema"] = json!(schema);
    }
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

/// A source as every version before 0.7 writes it, and as 0.6 may be converted.
fn old(name: &str) -> Value {
    json!({"type": "vendor-doc", "name": name})
}

/// The source `old(name)` as the converter writes it in 0.7.
fn new(name: &str) -> Value {
    json!({"type": "vendor-doc", "title": name})
}

fn with_id(id: &str, name: &str) -> Value {
    json!({"type": "vendor-doc", "id": id, "name": name})
}

/// The last link of a chain, of a version before 0.7: one control and the credit `extra` gives.
fn leaf(schema: &str, extra: Value) -> Value {
    let mut doc = device(schema, json!({"cc_params": {"entries": [{"osc": "/x", "cc": 1}]}}));
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn link(schema: &str, target: &str, extra: Value) -> Value {
    let mut doc = device(schema, json!({"midi_of": target}));
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn resolved(id: &str, files: &BTreeMap<String, String>) -> Resolved {
    osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"))
}

fn doc_of(r: &Resolved) -> Value {
    serde_json::from_str(&osd::write(&Descriptor::Device(Box::new(r.device.clone())))).unwrap()
}

fn doc(id: &str, files: &BTreeMap<String, String>) -> Value {
    doc_of(&resolved(id, files))
}

fn assert_no_underscore_credit(doc: &Value) {
    assert!(doc.get("_sources").is_none() && doc.get("_midi_of_sources").is_none(), "{doc}");
}

fn refused(id: &str, files: &BTreeMap<String, String>) -> (FaultKind, Option<String>, String) {
    let f = osd::resolve(id, files).expect_err("should be refused");
    (f.kind, f.file, f.pointer)
}

fn at(kind: FaultKind, file: &str, pointer: &str) -> (FaultKind, Option<String>, String) {
    (kind, Some(file.to_string()), pointer.to_string())
}

// ---- a link of a version before 0.7 under a file that asks for a newer one ----

#[test]
fn a_0_6_middle_link_with_underscore_sources_resolves_under_a_0_7_file() {
    let files = catalogue(vec![
        ("acme/a", link(S7, "acme/b", json!({}))),
        ("acme/b", link(S6, "acme/c", json!({"_sources": [{"type": "vendor-doc"}]}))),
        ("acme/c", leaf(S6, json!({}))),
    ]);
    let r = resolved("acme/a", &files);
    assert_eq!(r.files, ["acme/a", "acme/b", "acme/c"]);
    let doc = doc_of(&r);
    assert_eq!(doc["$schema"], S7);
    assert_eq!(doc["midi_of_sources"], json!([{"type": "vendor-doc"}]));
    assert!(doc.get("sources").is_none(), "a writes no sources: {doc}");
    assert_eq!(doc["cc_params"]["entries"][0]["osc"], "/x");
    assert_no_underscore_credit(&doc);
}

#[test]
fn the_credit_of_each_link_under_a_0_7_file_is_converted_and_listed_from_the_nearest_target() {
    let b = json!([{"type": "vendor-doc", "name": "B", "revision": "3", "imported_at": "2026-01-02"}]);
    let files = catalogue(vec![
        ("acme/a", link(S7, "acme/b", json!({"sources": [new("A")]}))),
        ("acme/b", link(S6, "acme/c", json!({"_sources": b}))),
        ("acme/c", leaf(S6, json!({"_sources": [old("C"), old("B2")]}))),
    ]);
    let doc = doc("acme/a", &files);
    assert_eq!(doc["sources"], json!([new("A")]));
    assert_eq!(
        doc["midi_of_sources"],
        json!([{"type": "vendor-doc", "title": "B", "version": "3", "_imported_at": "2026-01-02"}, new("C"), new("B2")])
    );
    assert_no_underscore_credit(&doc);
}

#[test]
fn every_middle_link_of_a_longer_chain_of_older_files_is_judged_on_its_own_text() {
    let files = catalogue(vec![
        ("acme/a", link(S7, "acme/b", json!({}))),
        ("acme/b", link(S6, "acme/c", json!({"_sources": [old("B")]}))),
        ("acme/c", link(S6, "acme/d", json!({"_sources": [old("C")]}))),
        ("acme/d", link(S5, "acme/e", json!({"_sources": [old("D")]}))),
        ("acme/e", leaf("", json!({"_sources": [old("E")]}))),
    ]);
    let doc = doc("acme/a", &files);
    assert_eq!(doc["midi_of_sources"], json!([new("B"), new("C"), new("D"), new("E")]));
    assert_no_underscore_credit(&doc);
}

#[test]
fn a_0_5_or_0_4_middle_link_with_underscore_sources_resolves_under_a_0_7_file() {
    for older in [S5, ""] {
        let files = catalogue(vec![
            ("acme/a", link(S7, "acme/b", json!({"sources": [new("A")]}))),
            ("acme/b", link(older, "acme/c", json!({"_sources": [old("B")]}))),
            ("acme/c", leaf(older, json!({"_sources": [old("C")]}))),
        ]);
        let doc = doc("acme/a", &files);
        assert_eq!(doc["sources"], json!([new("A")]), "{older}");
        assert_eq!(doc["midi_of_sources"], json!([new("B"), new("C")]), "{older}");
        assert_no_underscore_credit(&doc);
    }
}

#[test]
fn a_0_6_middle_link_under_a_0_7_file_resolves_over_a_0_7_target() {
    let files = catalogue(vec![
        ("acme/a", link(S7, "acme/b", json!({}))),
        ("acme/b", link(S6, "acme/c", json!({"_sources": [old("B")]}))),
        ("acme/c", device(S7, json!({"sources": [new("C")]}))),
    ]);
    assert_eq!(doc("acme/a", &files)["midi_of_sources"], json!([new("B"), new("C")]));
}

#[test]
fn a_0_6_middle_link_with_underscore_midi_of_sources_over_a_target_without_credit_keeps_that_credit() {
    for c in [leaf(S6, json!({})), device(S7, json!({}))] {
        let files = catalogue(vec![
            ("acme/a", link(S7, "acme/b", json!({}))),
            ("acme/b", link(S6, "acme/c", json!({"_midi_of_sources": [old("X")]}))),
            ("acme/c", c),
        ]);
        let doc = doc("acme/a", &files);
        assert_eq!(doc["midi_of_sources"], json!([new("X")]));
        assert_no_underscore_credit(&doc);
    }
}

// ---- the credit an older file writes under `_midi_of_sources` ----

/// b, of `schema`, credits B as its source and X as taken from another; its target c credits Y.
fn credited(schema: &str, a: Value) -> BTreeMap<String, String> {
    catalogue(vec![
        ("acme/a", a),
        ("acme/b", link(schema, "acme/c", json!({"_sources": [old("B")], "_midi_of_sources": [old("X")]}))),
        ("acme/c", leaf(schema, json!({"_sources": [old("Y")]}))),
    ])
}

#[test]
fn the_underscore_midi_of_sources_of_a_middle_link_follow_the_credit_of_its_target_in_0_7() {
    let files = credited(S6, link(S7, "acme/b", json!({})));
    let doc = doc("acme/a", &files);
    assert_eq!(doc["midi_of_sources"], json!([new("B"), new("Y"), new("X")]));
    assert_no_underscore_credit(&doc);
}

#[test]
fn the_underscore_midi_of_sources_of_a_file_resolved_in_its_own_version_follow_the_credit_of_its_target() {
    for schema in [S6, S5, ""] {
        let files = credited(schema, link(S7, "acme/b", json!({})));
        let doc = doc("acme/b", &files);
        assert_eq!(doc["_sources"], json!([old("B")]), "{schema}");
        assert_eq!(doc["_midi_of_sources"], json!([old("Y"), old("X")]), "{schema}");
        assert!(doc.get("midi_of_sources").is_none(), "{doc}");
    }
}

#[test]
fn the_underscore_midi_of_sources_of_an_older_file_follow_those_of_its_target() {
    let files = catalogue(vec![
        ("acme/b", link(S6, "acme/c", json!({"_midi_of_sources": [old("X"), old("Y")]}))),
        ("acme/c", leaf(S6, json!({"_sources": [old("Y")], "_midi_of_sources": [old("Z")]}))),
    ]);
    assert_eq!(doc("acme/b", &files)["_midi_of_sources"], json!([old("Y"), old("Z"), old("X"), old("Y")]));
}

#[test]
fn the_underscore_midi_of_sources_of_an_older_file_are_kept_when_it_is_joined_to_a_0_7_target() {
    let files = catalogue(vec![
        ("acme/a", link(S6, "acme/b", json!({"_midi_of_sources": [old("X")]}))),
        ("acme/b", device(S7, json!({"sources": [new("Y")]}))),
    ]);
    let doc = doc("acme/a", &files);
    assert_eq!(doc["midi_of_sources"], json!([new("Y"), new("X")]));
    assert_no_underscore_credit(&doc);
}

// ---- one id in the credit of an older file and of its target ----

fn same_id(a: &str, b_extra: Value, c_extra: Value) -> BTreeMap<String, String> {
    catalogue(vec![
        ("acme/a", link(a, "acme/b", json!({}))),
        ("acme/b", link(S6, "acme/c", b_extra)),
        ("acme/c", leaf(S6, c_extra)),
    ])
}

#[test]
fn an_id_an_older_link_writes_under_underscore_sources_and_its_target_credits_is_refused_in_0_7_at_that_key() {
    let files = same_id(S7, json!({"_sources": [with_id("manual", "B")]}), json!({"_sources": [with_id("manual", "C")]}));
    assert_eq!(refused("acme/a", &files), at(FaultKind::BadValue, "acme/b", "/_sources/0"));
}

#[test]
fn the_refused_id_is_pointed_at_its_own_place_in_the_list_the_older_link_writes() {
    for key in ["_sources", "_midi_of_sources"] {
        let files = same_id(
            S7,
            json!({ key: [with_id("other", "B1"), with_id("manual", "B2")] }),
            json!({"_sources": [with_id("manual", "C")]}),
        );
        assert_eq!(refused("acme/a", &files), at(FaultKind::BadValue, "acme/b", &format!("/{key}/1")), "{key}");
    }
}

#[test]
fn an_id_the_target_credits_as_taken_from_another_is_refused_too() {
    let files = same_id(S7, json!({"_sources": [with_id("manual", "B")]}), json!({"_midi_of_sources": [with_id("manual", "Z")]}));
    assert_eq!(refused("acme/a", &files), at(FaultKind::BadValue, "acme/b", "/_sources/0"));
    // Credited by a target further down the chain.
    let files = catalogue(vec![
        ("acme/a", link(S7, "acme/b", json!({}))),
        ("acme/b", link(S6, "acme/c", json!({"_sources": [with_id("manual", "B")]}))),
        ("acme/c", link(S6, "acme/d", json!({"_sources": [with_id("other", "C")]}))),
        ("acme/d", leaf(S6, json!({"_sources": [with_id("manual", "D")]}))),
    ]);
    assert_eq!(refused("acme/a", &files), at(FaultKind::BadValue, "acme/b", "/_sources/0"));
}

#[test]
fn an_id_of_an_older_file_that_a_0_7_target_credits_is_refused_at_the_key_it_writes() {
    for key in ["_sources", "_midi_of_sources"] {
        let files = catalogue(vec![
            ("acme/a", link(S6, "acme/b", json!({ key: [with_id("manual", "A")] }))),
            ("acme/b", device(S7, json!({"sources": [{"type": "vendor-doc", "id": "manual"}]}))),
        ]);
        assert_eq!(refused("acme/a", &files), at(FaultKind::BadValue, "acme/a", &format!("/{key}/0")), "{key}");
    }
}

#[test]
fn the_same_id_in_two_0_6_files_resolves_in_0_6_where_an_id_is_never_checked() {
    let files = same_id(S6, json!({"_sources": [with_id("manual", "B")]}), json!({"_sources": [with_id("manual", "C")]}));
    for id in ["acme/a", "acme/b"] {
        let doc = doc(id, &files);
        assert_eq!(doc["$schema"], S6);
        assert!(doc["_midi_of_sources"].as_array().unwrap().contains(&with_id("manual", "C")), "{id}: {doc}");
    }
}

#[test]
fn distinct_ids_down_a_chain_of_older_files_resolve_in_0_7_and_are_kept() {
    let files = same_id(S7, json!({"_sources": [with_id("b", "B")]}), json!({"_sources": [with_id("c", "C")]}));
    assert_eq!(
        doc("acme/a", &files)["midi_of_sources"],
        json!([{"type": "vendor-doc", "id": "b", "title": "B"}, {"type": "vendor-doc", "id": "c", "title": "C"}])
    );
}

// ---- the capabilities the conversion writes ----

#[test]
fn a_0_4_middle_link_that_adds_a_script_keeps_its_route_under_a_newer_file() {
    for newer in [S5, S6, S7] {
        let files = catalogue(vec![
            ("acme/a", link(newer, "acme/b", json!({}))),
            ("acme/b", link("", "acme/c", json!({"custom_commands": [{"osc": "/run", "script": "return {}"}]}))),
            ("acme/c", leaf("", json!({}))),
        ]);
        let r = resolved("acme/a", &files);
        assert_eq!(doc_of(&r)["custom_commands"][0]["osc"], "/run", "{newer}");
        assert_eq!((r.capabilities, r.inherited), (vec![Capability::Lua], vec![Capability::Lua]), "{newer}");
    }
}

// ---- what a middle link may still not hold ----

#[test]
fn a_0_5_middle_link_that_declares_capabilities_is_refused_in_its_file() {
    for extra in [json!({"capabilities": []}), json!({"capabilities": ["lua"], "custom_commands": [{"osc": "/run", "script": "return {}"}]})] {
        for newer in [S6, S7] {
            let files = catalogue(vec![
                ("acme/a", link(newer, "acme/b", json!({}))),
                ("acme/b", link(S5, "acme/c", extra.clone())),
                ("acme/c", leaf(S5, json!({}))),
            ]);
            assert_eq!(refused("acme/a", &files), at(FaultKind::UnknownField, "acme/b", "/capabilities"), "{newer} {extra}");
        }
    }
}

#[test]
fn a_0_6_middle_link_that_holds_a_key_of_its_target_is_refused_in_its_file() {
    // The second key holds only a field the conversion to 0.7 removes: the text still writes it.
    for transport in [json!({}), json!({"rate_limit_hz": 10})] {
        let files = catalogue(vec![
            ("acme/a", link(S7, "acme/b", json!({}))),
            ("acme/b", link(S6, "acme/c", json!({"transport": transport}))),
            ("acme/c", leaf(S6, json!({}))),
        ]);
        assert_eq!(refused("acme/a", &files), at(FaultKind::UnknownField, "acme/b", "/transport"), "{transport}");
    }
}

#[test]
fn a_0_7_middle_link_that_writes_midi_of_sources_is_refused_in_its_file() {
    let files = catalogue(vec![
        ("acme/a", link(S7, "acme/b", json!({}))),
        ("acme/b", link(S7, "acme/c", json!({"midi_of_sources": [new("X")]}))),
        ("acme/c", leaf(S6, json!({}))),
    ]);
    assert_eq!(refused("acme/a", &files), at(FaultKind::UnknownField, "acme/b", "/midi_of_sources"));
}
