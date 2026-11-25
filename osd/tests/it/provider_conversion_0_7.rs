//! The converter to 0.7 says a source type named after a product as a source `contributed`: the
//! partner goes to `provider` (none for `midnam`, a format and not a supplier) and the old word to
//! the first line of `notes`. The ten terms of evidence stay as they are.

use osd::{Descriptor, Kind, Version};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const D05: &str = "https://w3id.org/osd/0.5/device.schema.json";

/// Each source type named after a product, with the provider the converter writes for it.
const PRODUCTS: [(&str, Option<&str>); 4] = [
    ("electra-preset", Some("electra-one")),
    ("pencilresearch", Some("pencilresearch")),
    ("knobkraft", Some("knobkraft")),
    ("midnam", None),
];
const EVIDENCE: [&str; 10] = [
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
];

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

/// The text carried to 0.7: read back in 0.7, and converting to itself.
fn converted(doc: &Value) -> Value {
    let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {doc}"));
    osd::read_as(&out, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {out}"));
    let again = osd::convert_to(&out, Kind::Device, Version::V0_7).unwrap();
    assert_eq!(again, out, "the converted text converts to itself");
    serde_json::from_str(&out).unwrap()
}

/// The sources of a 0.6 text, converted.
fn sources_of(list: Value) -> Value {
    converted(&device(Some(D06), json!({"_sources": list})))["sources"].clone()
}

/// What the converter writes for a source of a product type with no other key but `notes`.
fn contributed(word: &str, provider: Option<&str>, notes: Option<&str>) -> Value {
    let line = format!("type in 0.6: {word}");
    let notes = notes.map_or(line.clone(), |n| format!("{line}\n{n}"));
    let mut s = json!({"type": "contributed", "notes": notes});
    if let Some(p) = provider {
        s["provider"] = json!(p);
    }
    s
}

#[test]
fn a_product_type_without_notes_becomes_contributed_with_its_provider_and_one_line_of_notes() {
    for (word, provider) in PRODUCTS {
        assert_eq!(sources_of(json!([{"type": word}])), json!([contributed(word, provider, None)]), "{word}");
    }
}

#[test]
fn the_old_word_is_the_first_line_of_notes_and_the_text_there_follows_unchanged() {
    for (word, provider) in PRODUCTS {
        for text in ["a preset, converted", "two lines\n  the second indented\n", "type in 0.6: knobkraft"] {
            assert_eq!(sources_of(json!([{"type": word, "notes": text}])), json!([contributed(word, provider, Some(text))]), "{word} {text:?}");
        }
    }
}

#[test]
fn a_source_midnam_gets_no_provider() {
    let out = sources_of(json!([{"type": "midnam", "url": "https://x.org/a.midnam"}]));
    assert_eq!(out, json!([{"type": "contributed", "url": "https://x.org/a.midnam", "notes": "type in 0.6: midnam"}]));
}

#[test]
fn the_other_keys_of_a_product_source_stay_and_are_carried_as_general_fields() {
    let out = sources_of(json!([{"type": "electra-preset", "preset_name": "P", "revision": "3", "url": "https://x.org/p/abc", "preset_id": "abc", "author": "Dee", "license": "CC-BY-4.0", "x-other": [1]}]));
    assert_eq!(
        out,
        json!([{"type": "contributed", "provider": "electra-one", "title": "P", "version": "3", "url": "https://x.org/p/abc", "author": "Dee", "license": "CC-BY-4.0", "x-other": [1], "notes": "type in 0.6: electra-preset"}])
    );
}

#[test]
fn the_ten_terms_of_evidence_get_no_provider_and_no_line_of_notes() {
    for word in EVIDENCE {
        let list = json!([{"type": word}, {"type": word, "notes": "n"}]);
        assert_eq!(sources_of(list.clone()), list, "{word}");
    }
    let extension = json!([{"type": "x-acme-scraped", "notes": "n"}]);
    assert_eq!(sources_of(extension.clone()), extension);
}

#[test]
fn the_midi_of_sources_follow_the_same_rule_each_source_on_its_own() {
    let doc = device(Some(D06), json!({"midi_of": "acme/b", "_midi_of_sources": [{"type": "vendor-doc"}, {"type": "knobkraft", "notes": "n"}, {"type": "midnam"}]}));
    assert_eq!(
        converted(&doc)["midi_of_sources"],
        json!([{"type": "vendor-doc"}, contributed("knobkraft", Some("knobkraft"), Some("n")), contributed("midnam", None, None)])
    );
}

#[test]
fn a_0_5_and_a_0_4_text_reach_0_7_by_the_same_rule() {
    for schema in [None, Some(D05)] {
        for (word, provider) in PRODUCTS {
            let out = converted(&device(schema, json!({"_sources": [{"type": word, "notes": "n"}]})));
            assert_eq!(out["sources"], json!([contributed(word, provider, Some("n"))]), "{schema:?} {word}");
        }
    }
}

#[test]
fn a_well_formed_provider_a_0_6_source_already_carries_is_kept_and_wins() {
    let out = sources_of(json!([
        {"type": "knobkraft", "provider": "acme-synths"},
        {"type": "midnam", "provider": "acme-synths"},
        {"type": "vendor-doc", "provider": "acme-synths"},
    ]));
    assert_eq!(
        out,
        json!([
            contributed("knobkraft", Some("acme-synths"), None),
            contributed("midnam", Some("acme-synths"), None),
            {"type": "vendor-doc", "provider": "acme-synths"},
        ])
    );
}

#[test]
fn a_provider_that_is_not_well_formed_goes_under_underscore_provider_and_the_partner_takes_its_place() {
    for value in [json!("Electra One"), json!("a".repeat(65)), json!(""), json!(5), json!(null), json!({"x": 1})] {
        let out = sources_of(json!([
            {"type": "electra-preset", "provider": value},
            {"type": "midnam", "provider": value},
            {"type": "vendor-doc", "provider": value},
        ]));
        let mut electra = contributed("electra-preset", Some("electra-one"), None);
        electra["_provider"] = value.clone();
        let mut midnam = contributed("midnam", None, None);
        midnam["_provider"] = value.clone();
        assert_eq!(out, json!([electra, midnam, {"type": "vendor-doc", "_provider": value}]), "{value}");
    }
}

#[test]
fn every_provider_the_converter_writes_is_in_the_reader_s_table() {
    let out = converted(&device(Some(D06), json!({"_sources": [{"type": "electra-preset"}, {"type": "pencilresearch"}, {"type": "knobkraft"}, {"type": "midnam"}]})));
    let d = osd::read(&out.to_string(), Kind::Device).unwrap();
    assert!(osd::warnings(&d).is_empty());
}

// ---- the resolver ----

fn catalogue(files: Vec<(&str, Value)>) -> BTreeMap<String, String> {
    files.into_iter().map(|(id, v)| (id.to_string(), v.to_string())).collect()
}

fn resolved(id: &str, files: &BTreeMap<String, String>) -> Value {
    let r = osd::resolve(id, files).unwrap_or_else(|f| panic!("{f}"));
    let text = osd::write(&Descriptor::Device(Box::new(r.device)));
    osd::read_as(&text, Kind::Device, Version::V0_7).unwrap_or_else(|f| panic!("{f} in {text}"));
    serde_json::from_str(&text).unwrap()
}

#[test]
fn a_resolution_carries_the_sources_of_a_0_6_target_joined_to_a_0_7_file() {
    let files = catalogue(vec![
        ("acme/a", device(Some(D07), json!({"midi_of": "acme/b", "sources": [{"type": "vendor-doc"}]}))),
        ("acme/b", device(Some(D06), json!({"cc_params": {"entries": [{"osc": "/x", "cc": 1}]}, "_sources": [{"type": "knobkraft", "notes": "B"}, {"type": "midnam"}]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert_eq!(doc["sources"], json!([{"type": "vendor-doc"}]));
    assert_eq!(doc["midi_of_sources"], json!([contributed("knobkraft", Some("knobkraft"), Some("B")), contributed("midnam", None, None)]));
}

#[test]
fn a_resolution_carries_the_sources_of_a_0_6_file_joined_to_a_0_7_target() {
    let files = catalogue(vec![
        ("acme/a", device(Some(D06), json!({"midi_of": "acme/b", "_sources": [{"type": "pencilresearch"}]}))),
        ("acme/b", device(Some(D07), json!({"cc_params": {"entries": [{"osc": "/x", "ways": [{"ctrlType": "cc", "ctrlIndex": [1]}]}]}, "sources": [{"type": "vendor-doc", "provider": "acme-synths"}]}))),
    ]);
    let doc = resolved("acme/a", &files);
    assert_eq!(doc["sources"], json!([contributed("pencilresearch", Some("pencilresearch"), None)]));
    assert_eq!(doc["midi_of_sources"], json!([{"type": "vendor-doc", "provider": "acme-synths"}]));
}
