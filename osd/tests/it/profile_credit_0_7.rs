//! The credit of a maker profile in 0.7: a profile names its sources under `sources`, a list of
//! the sources of a device, judged by the same rules; `_sources` is refused; the converter renames
//! the `_sources` of an older profile, carries each source as a device's and keeps under the note
//! `_sources_2` what 0.7 cannot hold.

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version, WarningKind};
use serde_json::{json, Value};

const P07: &str = "https://w3id.org/osd/0.7/profile.schema.json";
const P06: &str = "https://w3id.org/osd/0.6/profile.schema.json";
const P05: &str = "https://w3id.org/osd/0.5/profile.schema.json";

/// A profile of the version of `schema` (none: 0.4) with the keys of `extra`.
fn profile(schema: Option<&str>, extra: Value) -> Value {
    let mut doc = json!({});
    if let Some(s) = schema {
        doc["$schema"] = json!(s);
    }
    for (k, v) in extra.as_object().unwrap() {
        doc[k] = v.clone();
    }
    doc
}

fn with_sources(list: Value) -> Value {
    profile(Some(P07), json!({"sources": list}))
}

fn refused(doc: &Value) -> (FaultKind, String) {
    let f = osd::read(&doc.to_string(), Kind::Profile).expect_err(&format!("refused: {doc}"));
    (f.kind, f.pointer)
}

fn read_ok(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Profile).unwrap_or_else(|f| panic!("{f} in {doc}"))
}

fn written(d: &Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn schema_errors(doc: &Value) -> Vec<String> {
    common::Schemas::load("0.7").errors(Kind::Profile, doc).into_iter().map(|(at, _)| at).collect()
}

/// The profile carried to 0.7: read back in 0.7, valid against the 0.7 schema, converting to itself.
fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Profile, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {doc}"));
    osd::read_as(&out, Kind::Profile, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {out}"));
    assert_eq!(osd::convert_to(&out, Kind::Profile, Version::V0_7).as_deref(), Ok(out.as_str()), "the result converts to itself");
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(schema_errors(&v), Vec::<String>::new(), "{v}");
    assert!(v.get("_sources").is_none(), "no _sources is written: {v}");
    v
}

fn not_converted(doc: &Value) -> (FaultKind, String) {
    let f = osd::convert_to(&doc.to_string(), Kind::Profile, Version::V0_7).expect_err(&format!("not converted: {doc}"));
    (f.kind, f.pointer)
}

fn at(kind: FaultKind, pointer: &str) -> (FaultKind, String) {
    (kind, pointer.to_string())
}

// ---- reading `sources` ----

#[test]
fn a_0_7_profile_reads_its_sources_writes_them_back_and_validates() {
    let doc = with_sources(json!([
        {"id": "manual", "type": "vendor-doc", "title": "Manual", "version": "2", "sha256": "ab", "license": "CC0-1.0", "provider": "knobkraft", "x-other": [1], "notes": "n"},
        {"type": "x-acme-scraped", "_imported_at": "2026-01-01"}
    ]));
    assert_eq!(written(&read_ok(&doc)), doc);
    assert_eq!(schema_errors(&doc), Vec::<String>::new());
}

#[test]
fn a_0_7_profile_keeps_its_sources_beside_its_blocks_and_notes() {
    let doc = profile(Some(P07), json!({"identity": {"reply_text": "acme"}, "sources": [{"type": "community-re"}], "programs": {"program_change": true}, "_midi_of_sources": "a note"}));
    assert_eq!(written(&read_ok(&doc)), doc);
    assert_eq!(schema_errors(&doc), Vec::<String>::new());
}

#[test]
fn a_0_7_profile_reads_an_empty_list_of_sources_and_writes_it_back() {
    let doc = with_sources(json!([]));
    assert_eq!(written(&read_ok(&doc)), doc);
    assert_eq!(schema_errors(&doc), Vec::<String>::new());
}

#[test]
fn a_0_7_profile_converts_to_itself_with_its_sources() {
    let doc = with_sources(json!([{"type": "vendor-doc"}]));
    assert_eq!(converted(&doc), doc);
}

// ---- `_sources` in 0.7 ----

#[test]
fn a_0_7_profile_refuses_underscore_sources_as_a_bad_value_whatever_its_value() {
    for value in [json!([{"type": "vendor-doc"}]), json!("a text"), json!(null), json!([]), json!({"x": 1}), json!([{"no_type": true}])] {
        let doc = profile(Some(P07), json!({"_sources": value}));
        assert_eq!(refused(&doc), at(FaultKind::BadValue, "/_sources"), "{doc}");
    }
}

#[test]
fn a_0_7_profile_refuses_underscore_sources_beside_sources() {
    let list = json!([{"type": "vendor-doc"}]);
    let doc = profile(Some(P07), json!({"sources": list.clone(), "_sources": list}));
    assert_eq!(refused(&doc), at(FaultKind::BadValue, "/_sources"));
}

#[test]
fn the_0_7_profile_schema_refuses_underscore_sources_at_its_key() {
    for value in [json!([{"type": "vendor-doc"}]), json!("a text"), json!(null)] {
        let doc = profile(Some(P07), json!({"_sources": value}));
        assert_eq!(schema_errors(&doc), ["/_sources"], "{doc}");
    }
}

// ---- each source judged as a device's ----

/// The reader and the schema both refuse the profile, the reader as `kind` at `pointer`.
fn refused_by_both(list: Value, kind: FaultKind, pointer: &str) {
    let doc = with_sources(list);
    assert_eq!(refused(&doc), at(kind, pointer), "{doc}");
    assert!(!schema_errors(&doc).is_empty(), "the schema accepts {doc}");
}

#[test]
fn a_source_of_a_profile_without_a_type_is_missing_it() {
    refused_by_both(json!([{"type": "vendor-doc"}, {"title": "Manual"}]), FaultKind::Missing, "/sources/1/type");
}

#[test]
fn a_source_of_a_profile_takes_each_of_the_ten_terms_or_an_extension() {
    let terms = [
        "hardware-verified",
        "hardware-observed",
        "software-verified",
        "vendor-doc",
        "vendor-osc-api",
        "vendor-self-described",
        "third-party-osc",
        "community-re",
        "contributed",
        "test-fixture",
        "x-acme-scraped",
    ];
    for term in terms {
        let doc = with_sources(json!([{"type": term}]));
        assert_eq!(written(&read_ok(&doc)), doc);
        assert_eq!(schema_errors(&doc), Vec::<String>::new(), "{doc}");
    }
}

#[test]
fn a_source_of_a_profile_refuses_the_product_types_and_a_word_outside_the_terms() {
    for word in ["electra-preset", "pencilresearch", "knobkraft", "midnam", "vendor", "Vendor-Doc", ""] {
        refused_by_both(json!([{"type": word}]), FaultKind::BadValue, "/sources/0/type");
    }
}

#[test]
fn a_source_of_a_profile_takes_a_licence_of_the_closed_list_alone() {
    for license in ["CC-BY-4.0", "public-domain", "NOASSERTION", "NONE"] {
        read_ok(&with_sources(json!([{"type": "vendor-doc", "license": license}])));
    }
    for license in ["free", "none", "Public-Domain"] {
        refused_by_both(json!([{"type": "vendor-doc", "license": license}]), FaultKind::BadValue, "/sources/0/license");
    }
}

#[test]
fn a_malformed_provider_of_a_profile_source_is_a_bad_value() {
    for provider in [json!("Knob Kraft"), json!(""), json!("a--b"), json!("a".repeat(65)), json!(3), json!(null)] {
        refused_by_both(json!([{"type": "vendor-doc", "provider": provider}]), FaultKind::BadValue, "/sources/0/provider");
    }
}

#[test]
fn the_id_of_a_source_of_a_profile_is_a_text_that_is_not_empty() {
    for id in [json!(""), json!(3)] {
        refused_by_both(json!([{"type": "vendor-doc", "id": id}]), FaultKind::BadValue, "/sources/0/id");
    }
}

#[test]
fn the_id_of_a_source_of_a_profile_is_written_once() {
    let doc = with_sources(json!([{"type": "vendor-doc", "id": "m"}, {"type": "community-re", "id": "n"}, {"type": "contributed", "id": "m"}]));
    assert_eq!(refused(&doc), at(FaultKind::BadValue, "/sources/2"));
}

#[test]
fn the_sources_of_a_profile_are_a_list_of_objects() {
    refused_by_both(json!("a text"), FaultKind::BadValue, "/sources");
    refused_by_both(json!({"type": "vendor-doc"}), FaultKind::BadValue, "/sources");
    refused_by_both(json!(["a text"]), FaultKind::BadValue, "/sources/0");
}

// ---- warnings ----

#[test]
fn each_provider_of_a_0_7_profile_that_the_table_does_not_hold_is_warned_of_at_its_field() {
    let doc = with_sources(json!([
        {"type": "vendor-doc", "provider": "a-new-firmware"},
        {"type": "contributed", "provider": "knobkraft"},
        {"type": "contributed"},
        {"type": "contributed", "provider": "another-one"}
    ]));
    let got: Vec<(String, WarningKind)> = osd::warnings(&read_ok(&doc)).into_iter().map(|w| (w.pointer, w.kind)).collect();
    assert_eq!(
        got,
        [
            ("/sources/0/provider".to_string(), WarningKind::UnknownProvider("a-new-firmware".to_string())),
            ("/sources/3/provider".to_string(), WarningKind::UnknownProvider("another-one".to_string())),
        ]
    );
}

// ---- conversion ----

#[test]
fn the_underscore_sources_of_a_0_6_profile_become_sources_at_the_same_place() {
    let doc = profile(Some(P06), json!({"identity": {"reply_text": "acme"}, "_sources": [{"type": "vendor-doc", "url": "https://x.org/m"}], "programs": {"program_change": true}, "_notes": "kept"}));
    let out = converted(&doc);
    let keys: Vec<&String> = out.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["$schema", "identity", "sources", "programs", "_notes"]);
    assert_eq!(out["sources"], json!([{"type": "vendor-doc", "url": "https://x.org/m"}]));
    assert_eq!(out["identity"], doc["identity"]);
    assert_eq!(out["programs"], doc["programs"]);
    assert_eq!(out["_notes"], "kept");
}

#[test]
fn the_conversion_of_a_profile_carries_the_general_keys_of_each_source() {
    let doc = profile(Some(P06), json!({"_sources": [
        {"type": "vendor-doc", "preset_name": "P", "revision": "3", "content_sha256": "ab", "url": "https://x.org/p/abc", "preset_id": "abc", "x-other": [1]},
        {"type": "community-re", "name": "N", "preset_id": "zzz", "url": "https://x.org/p/abc"}
    ]}));
    assert_eq!(
        converted(&doc)["sources"],
        json!([
            {"type": "vendor-doc", "title": "P", "version": "3", "sha256": "ab", "url": "https://x.org/p/abc", "x-other": [1]},
            {"type": "community-re", "title": "N", "preset_id": "zzz", "url": "https://x.org/p/abc"}
        ])
    );
}

#[test]
fn the_conversion_of_a_profile_keeps_the_date_of_a_source_as_a_note() {
    let doc = profile(Some(P06), json!({"_sources": [
        {"type": "vendor-doc", "imported_at": "2026-01-01"},
        {"type": "vendor-doc", "_imported_at": "taken", "imported_at": "2026-02-02"}
    ]}));
    assert_eq!(
        converted(&doc)["sources"],
        json!([
            {"type": "vendor-doc", "_imported_at": "2026-01-01"},
            {"type": "vendor-doc", "_imported_at": "taken", "_imported_at_2": "2026-02-02"}
        ])
    );
}

#[test]
fn the_conversion_of_a_profile_says_a_product_type_as_contributed_with_its_provider() {
    let doc = profile(Some(P06), json!({"_sources": [
        {"type": "electra-preset", "notes": "first\nsecond"},
        {"type": "midnam"},
        {"type": "knobkraft", "provider": "Not Well Formed"}
    ]}));
    assert_eq!(
        converted(&doc)["sources"],
        json!([
            {"type": "contributed", "provider": "electra-one", "notes": "type in 0.6: electra-preset\nfirst\nsecond"},
            {"type": "contributed", "notes": "type in 0.6: midnam"},
            {"type": "contributed", "_provider": "Not Well Formed", "provider": "knobkraft", "notes": "type in 0.6: knobkraft"}
        ])
    );
}

#[test]
fn an_empty_underscore_sources_of_a_profile_leaves_neither_sources_nor_a_note() {
    assert_eq!(converted(&profile(Some(P06), json!({"_sources": []}))), profile(Some(P07), json!({})));
}

#[test]
fn the_conversion_of_a_profile_leaves_its_underscore_midi_of_sources_a_note() {
    let note = json!([{"type": "vendor", "imported_at": "x"}]);
    let out = converted(&profile(Some(P06), json!({"_sources": [{"type": "vendor-doc"}], "_midi_of_sources": note.clone()})));
    assert_eq!(out["sources"], json!([{"type": "vendor-doc"}]));
    assert_eq!(out["_midi_of_sources"], note);
    assert!(out.get("midi_of_sources").is_none(), "{out}");
}

#[test]
fn a_0_5_profile_with_underscore_sources_reaches_sources_in_0_7() {
    let doc = profile(Some(P05), json!({"identity": {"reply_text": "acme"}, "_sources": [{"type": "pencilresearch", "revision": "3"}]}));
    assert_eq!(
        converted(&doc)["sources"],
        json!([{"type": "contributed", "provider": "pencilresearch", "version": "3", "notes": "type in 0.6: pencilresearch"}])
    );
}

#[test]
fn the_source_words_of_a_0_4_profile_go_through_the_tables_of_0_5_as_a_devices() {
    let doc = profile(None, json!({"identity": {"reply_text": "acme"}, "_sources": [
        {"type": "vendor", "license": "none"},
        {"type": "agent-contributed", "license": "CC0-1.0"},
        {"type": "hardware-verified-partial"},
        {"type": "electra-preset", "imported_at": "2026-01-01"}
    ]}));
    assert_eq!(
        converted(&doc)["sources"],
        json!([
            {"type": "vendor-doc", "license": "NONE"},
            {"type": "contributed", "license": "CC0-1.0"},
            {"type": "hardware-verified"},
            {"type": "contributed", "provider": "electra-one", "_imported_at": "2026-01-01", "notes": "type in 0.6: electra-preset"}
        ])
    );
}

#[test]
fn a_0_4_source_word_outside_the_tables_is_not_convertible_in_a_profile_as_in_a_device() {
    let word = profile(None, json!({"_sources": [{"type": "vendor-doc"}, {"type": "rumour"}]}));
    assert_eq!(not_converted(&word), at(FaultKind::NotConvertible, "/_sources/1/type"));
    let license = profile(None, json!({"_sources": [{"type": "vendor-doc", "license": "free"}]}));
    assert_eq!(not_converted(&license), at(FaultKind::NotConvertible, "/_sources/0/license"));
}

#[test]
fn the_tables_of_0_4_are_not_applied_to_a_0_5_or_0_6_profile() {
    for schema in [P05, P06] {
        for entry in [json!({"type": "vendor"}), json!({"type": "vendor-doc", "license": "none"})] {
            let doc = profile(Some(schema), json!({"_sources": [entry.clone()]}));
            let out = converted(&doc);
            assert_eq!(out["_sources_2"], json!([entry]), "{schema}");
            assert!(out.get("sources").is_none(), "{out}");
        }
    }
}

#[test]
fn underscore_sources_that_is_not_a_list_is_kept_whole_under_a_note_from_each_older_version_of_a_profile() {
    for value in [json!("scraped from the manual"), json!({"type": "vendor-doc"})] {
        for schema in [None, Some(P05), Some(P06)] {
            let doc = profile(schema, json!({"_sources": value.clone()}));
            let out = converted(&doc);
            assert_eq!(out["_sources_2"], value, "{doc}");
            assert!(out.get("sources").is_none(), "{out}");
        }
    }
}

#[test]
fn an_older_profile_credits_each_entry_0_7_can_hold_and_keeps_the_others_under_a_note() {
    for schema in [None, Some(P05), Some(P06)] {
        let doc = profile(schema, json!({"_sources": [{"title": "Manual"}, {"type": "vendor-doc", "id": "m"}, {"type": "community-re", "id": "m"}]}));
        let out = converted(&doc);
        assert_eq!(
            out["sources"],
            json!([{"type": "contributed", "title": "Manual", "notes": "no type in 0.6"}, {"type": "vendor-doc", "id": "m"}]),
            "{schema:?}"
        );
        assert_eq!(out["_sources_2"], json!([{"type": "community-re", "id": "m"}]), "{schema:?}");
    }
}
