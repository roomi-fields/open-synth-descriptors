//! The converter to 0.7 keeps a source's `imported_at`, which 0.7 does not define, as a note for
//! people: `_imported_at`, or the next free suffix when that key is taken (the rule of
//! `_provider`, spec/0.7.md 6.8.4). It holds for the sources of both lists and on every path that
//! converts to 0.7.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Source, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const D05: &str = "https://w3id.org/osd/0.5/device.schema.json";

/// The versions before 0.7, by their `$schema` (a 0.4 text states none).
const BEFORE_0_7: [Option<&str>; 3] = [None, Some(D05), Some(D06)];

fn device(schema: Option<&str>, extra: Value) -> Value {
    let mut doc = json!({"device": {"name": "Synth", "osc_prefix": "/synth"}});
    if let Some(s) = schema {
        doc["$schema"] = json!(s);
    }
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

/// The text carried to 0.7: read by `osd::read` in 0.7, valid against the 0.7 schema, and
/// converting to itself.
fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {doc}"));
    let read = osd::read(&out, Kind::Device).unwrap_or_else(|f| panic!("{f} in {out}"));
    assert_eq!(read.schema(), Some(Version::V0_7), "{out}");
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(common::Schemas::load("0.7").check(Kind::Device, &value), None, "{out}");
    assert_eq!(osd::convert_to(&out, Kind::Device, Version::V0_7).as_deref(), Ok(out.as_str()), "the converted text converts to itself");
    value
}

/// The sources of a text of `schema`, converted.
fn sources_of(schema: Option<&str>, list: Value) -> Value {
    converted(&device(schema, json!({"_sources": list})))["sources"].clone()
}

#[test]
fn the_date_of_a_source_of_0_4_0_5_or_0_6_becomes_the_note_imported_at_with_the_same_value() {
    for schema in BEFORE_0_7 {
        for date in ["2026-01-01", "2026-01-02T03:04:05Z", "last spring", ""] {
            let out = sources_of(schema, json!([{"type": "vendor-doc", "imported_at": date}]));
            assert_eq!(out, json!([{"type": "vendor-doc", "_imported_at": date}]), "{schema:?} {date:?}");
        }
    }
}

#[test]
fn a_note_imported_at_already_there_stays_and_the_date_takes_the_suffix_2_then_3() {
    for schema in BEFORE_0_7 {
        let out = sources_of(schema, json!([
            {"type": "vendor-doc", "imported_at": "2026-01-01", "_imported_at": "2025-12-01"},
            {"type": "vendor-doc", "imported_at": "2026-01-01", "_imported_at": "2025-12-01", "_imported_at_2": "2025-11-01"},
        ]));
        assert_eq!(
            out,
            json!([
                {"type": "vendor-doc", "_imported_at": "2025-12-01", "_imported_at_2": "2026-01-01"},
                {"type": "vendor-doc", "_imported_at": "2025-12-01", "_imported_at_2": "2025-11-01", "_imported_at_3": "2026-01-01"},
            ]),
            "{schema:?}"
        );
    }
}

#[test]
fn the_date_takes_the_first_free_key_whatever_the_taken_ones_hold() {
    let out = sources_of(Some(D06), json!([
        {"type": "vendor-doc", "imported_at": "d", "_imported_at_2": "later"},
        {"type": "vendor-doc", "imported_at": "d", "_imported_at": "a", "_imported_at_3": "c"},
        {"type": "vendor-doc", "imported_at": "d", "_imported_at": null, "_imported_at_2": 7, "_imported_at_3": {"x": 1}},
        {"type": "vendor-doc", "imported_at": "d", "_imported_at": "d"},
    ]));
    assert_eq!(
        out,
        json!([
            {"type": "vendor-doc", "_imported_at": "d", "_imported_at_2": "later"},
            {"type": "vendor-doc", "_imported_at": "a", "_imported_at_2": "d", "_imported_at_3": "c"},
            {"type": "vendor-doc", "_imported_at": null, "_imported_at_2": 7, "_imported_at_3": {"x": 1}, "_imported_at_4": "d"},
            {"type": "vendor-doc", "_imported_at": "d", "_imported_at_2": "d"},
        ])
    );
}

#[test]
fn the_other_keys_of_a_dated_source_come_out_as_without_the_date() {
    let plain = json!([
        {"type": "electra-preset", "preset_name": "P", "revision": "3", "url": "https://x.org/p/abc", "preset_id": "abc", "license": "CC-BY-4.0", "x-other": [1]},
        {"type": "knobkraft", "provider": "Knob Kraft", "notes": "n"},
        {"type": "midnam", "content_sha256": "00", "attribution": "A", "_note": "kept"},
        {"type": "community-re", "id": "s1", "firmware": "1.2", "author": "Dee", "commit": "abc", "notes": "n"},
    ]);
    for schema in BEFORE_0_7 {
        let mut dated = plain.clone();
        for (n, s) in dated.as_array_mut().unwrap().iter_mut().enumerate() {
            s["imported_at"] = json!(format!("2026-01-0{}", n + 1));
        }
        let mut want = sources_of(schema, plain.clone());
        for (n, s) in want.as_array_mut().unwrap().iter_mut().enumerate() {
            s["_imported_at"] = json!(format!("2026-01-0{}", n + 1));
        }
        assert_eq!(sources_of(schema, dated), want, "{schema:?}");
    }
}

#[test]
fn a_source_without_a_date_gains_no_key() {
    for schema in BEFORE_0_7 {
        let list = json!([
            {"type": "vendor-doc"},
            {"type": "community-re", "_imported_at": "2025-12-01", "notes": "n"},
            {"type": "vendor-doc", "_imported_at_2": "x"},
        ]);
        assert_eq!(sources_of(schema, list.clone()), list, "{schema:?}");
    }
}

#[test]
fn each_source_of_both_lists_keeps_its_own_date() {
    let doc = device(Some(D06), json!({
        "midi_of": "acme/b",
        "_sources": [{"type": "vendor-doc", "imported_at": "a"}, {"type": "test-fixture"}, {"type": "vendor-doc", "imported_at": "b"}],
        "_midi_of_sources": [{"type": "vendor-doc", "imported_at": "c", "_imported_at": "old"}, {"type": "midnam", "imported_at": "d"}],
    }));
    let out = converted(&doc);
    assert_eq!(out["sources"], json!([{"type": "vendor-doc", "_imported_at": "a"}, {"type": "test-fixture"}, {"type": "vendor-doc", "_imported_at": "b"}]));
    assert_eq!(
        out["midi_of_sources"],
        json!([
            {"type": "vendor-doc", "_imported_at": "old", "_imported_at_2": "c"},
            {"type": "contributed", "notes": "type in 0.6: midnam", "_imported_at": "d"},
        ])
    );
}

#[test]
fn a_converted_source_holds_the_date_as_a_note_and_none_in_its_field() {
    let out = osd::convert_to(&device(Some(D06), json!({"_sources": [{"type": "vendor-doc", "imported_at": "2026-01-01"}]})).to_string(), Kind::Device, Version::V0_7).unwrap();
    let Descriptor::Device(d) = osd::read(&out, Kind::Device).unwrap() else { panic!("a device") };
    let source: &Source = &d.sources.as_ref().unwrap()[0];
    assert_eq!(source.imported_at, None);
    assert_eq!(source.extra.get("_imported_at"), Some(&json!("2026-01-01")));
}

#[test]
fn a_0_7_text_that_holds_the_date_beside_the_note_is_still_refused_at_the_date() {
    let doc = device(Some(D07), json!({"sources": [{"type": "vendor-doc", "_imported_at": "a", "imported_at": "b"}]}));
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err("refused");
    assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/sources/0/imported_at"));
}

// ---- the resolver ----

fn resolved(files: Vec<(&str, Value)>) -> Value {
    let files: BTreeMap<String, String> = files.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let r = osd::resolve("acme/a", &files).unwrap_or_else(|f| panic!("{f}"));
    let text = osd::write(&Descriptor::Device(Box::new(r.device)));
    osd::read_as(&text, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {text}"));
    serde_json::from_str(&text).unwrap()
}

/// A target with one control and one source, in the form of its version.
fn target(schema: Option<&str>, source: Value) -> Value {
    if schema == Some(D07) {
        device(schema, json!({"cc_params": {"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]}, "sources": [source]}))
    } else {
        device(schema, json!({"cc_params": {"entries": [{"osc": "/x", "cc": 1}]}, "_sources": [source]}))
    }
}

#[test]
fn a_resolution_that_joins_a_target_before_0_7_to_a_0_7_file_keeps_the_dates_of_the_target() {
    for schema in BEFORE_0_7 {
        let doc = resolved(vec![
            ("acme/a", device(Some(D07), json!({"midi_of": "acme/b", "sources": [{"type": "test-fixture"}]}))),
            ("acme/b", target(schema, json!({"type": "vendor-doc", "imported_at": "2026-01-01", "_imported_at": "2025-12-01"}))),
        ]);
        assert_eq!(doc["sources"], json!([{"type": "test-fixture"}]), "{schema:?}");
        assert_eq!(
            doc["midi_of_sources"],
            json!([{"type": "vendor-doc", "_imported_at": "2025-12-01", "_imported_at_2": "2026-01-01"}]),
            "{schema:?}"
        );
    }
}

#[test]
fn a_resolution_that_joins_a_file_before_0_7_to_a_0_7_target_keeps_the_dates_of_the_file() {
    let doc = resolved(vec![
        ("acme/a", device(Some(D06), json!({"midi_of": "acme/b", "_sources": [{"type": "vendor-doc", "imported_at": "2026-02-02"}]}))),
        ("acme/b", target(Some(D07), json!({"type": "test-fixture"}))),
    ]);
    assert_eq!(doc["sources"], json!([{"type": "vendor-doc", "_imported_at": "2026-02-02"}]));
    assert_eq!(doc["midi_of_sources"], json!([{"type": "test-fixture"}]));
}

#[test]
fn a_resolution_through_a_chain_keeps_the_date_of_the_last_target_before_0_7() {
    let doc = resolved(vec![
        ("acme/a", device(Some(D07), json!({"midi_of": "acme/b"}))),
        ("acme/b", device(Some(D06), json!({"midi_of": "acme/c"}))),
        ("acme/c", target(Some(D06), json!({"type": "vendor-doc", "imported_at": "e"}))),
    ]);
    assert_eq!(doc["midi_of_sources"], json!([{"type": "vendor-doc", "_imported_at": "e"}]), "{doc}");
}

// ---- the texts ----

fn text_of(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

const CASE: &str = "0.7/convert/source-imported-at-becomes-a-note";

#[test]
fn the_conversion_case_shows_a_date_kept_as_a_note_and_a_date_under_the_third_key() {
    let dir = common::repo().join("conformance").join(CASE);
    let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("case.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(dir.join("descriptor.json")).unwrap();
    let out = osd::convert_to(&text, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f}"));
    osd::read(&out, Kind::Device).unwrap_or_else(|f| panic!("{f} in {out}"));
    assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), case["result"]);
}

#[test]
fn the_text_of_0_7_says_the_date_of_a_source_is_kept_as_a_note_and_names_its_case() {
    let spec = text_of("spec/0.7.md");
    let carried = spec.lines().find(|l| l.starts_with("The converter from 0.6 carries a source")).expect("the paragraph on the conversion of a source");
    assert!(carried.contains("`_imported_at`") && carried.contains("`_imported_at_2`"), "{carried}");
    assert!(!spec.contains("`imported_at` is dropped"), "the text still says the date is dropped");
    assert!(!spec.contains("drops each of the seven"), "the text still says the converter drops the seven fields");
    let section: Vec<&str> = spec.lines().skip_while(|l| !l.starts_with("### 5.3.2 ")).skip(1).take_while(|l| !l.starts_with('#')).collect();
    assert!(section.iter().any(|l| l.contains("`_imported_at`")), "5.3.2 does not say `_imported_at`");
    assert!(section.iter().any(|l| l.starts_with("Cases:") && l.contains(&format!("`{CASE}`"))), "the Cases line of 5.3.2 does not name {CASE}");
}

#[test]
fn the_proposal_of_the_removed_fields_says_the_date_is_kept_as_a_note_and_names_its_case() {
    let proposal = text_of("spec/proposals/0008-what-0-7-removes-and-what-changes-form.md");
    assert!(proposal.contains("`_imported_at`"));
    assert!(proposal.contains("source-imported-at-becomes-a-note"));
    for old in ["`imported_at`, a key the format no longer defines, is dropped", "Only the converter 0.6 to 0.7 drops it"] {
        assert!(!proposal.contains(old), "the proposal still says: {old}");
    }
}

#[test]
fn the_interface_line_of_version_says_the_date_is_kept_as_a_note() {
    let interface = text_of("osd/docs/INTERFACE.md");
    let line = interface.lines().find(|l| l.starts_with("| `Version` |")).expect("the line of Version");
    assert!(line.contains("`_imported_at`"), "{line}");
    assert!(!line.contains("`imported_at` dropped"), "{line}");
}
