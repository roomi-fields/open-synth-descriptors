//! A source of 0.7 names who supplies it by `provider`, an identifier of the table of providers:
//! the reader checks its form only, warns of a well-formed identifier its table does not hold, and
//! refuses the four source types named after a product.

// @lit spec/0.7.md schemas/0.7/** osd/providers.json

use crate::common;

use osd::{Descriptor, FaultKind, Kind, Version};
use serde_json::{json, Value};

const D07: &str = "https://w3id.org/osd/0.7/device.schema.json";
const D06: &str = "https://w3id.org/osd/0.6/device.schema.json";
const D05: &str = "https://w3id.org/osd/0.5/device.schema.json";

const LISTS: [&str; 2] = ["sources", "midi_of_sources"];

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
const PRODUCTS: [&str; 4] = ["electra-preset", "pencilresearch", "knobkraft", "midnam"];

/// A device naming `schema` (none for 0.4), with the blocks of `extra`.
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

fn device_07(list: &str, sources: Value) -> Value {
    device(Some(D07), json!({ list: sources }))
}

fn read(doc: &Value) -> Descriptor {
    osd::read(&doc.to_string(), Kind::Device).unwrap_or_else(|f| panic!("{f} in {doc}"))
}

fn written(d: &Descriptor) -> Value {
    serde_json::from_str(&osd::write(d)).unwrap()
}

fn refused(doc: &Value) -> (FaultKind, String) {
    let f = osd::read(&doc.to_string(), Kind::Device).expect_err(&format!("the reader accepts {doc}"));
    (f.kind, f.pointer)
}

fn warned(d: &Descriptor) -> Vec<(String, String)> {
    osd::warnings(d)
        .into_iter()
        .map(|w| match w.kind {
            osd::WarningKind::UnknownProvider(id) => (w.pointer, id),
            osd::WarningKind::PartialControl => panic!("no control of these devices is partial: {}", w.pointer),
            osd::WarningKind::UncreditedSource => panic!("a device keeps no source out of its credit: {}", w.pointer),
            _ => panic!("no place of these devices is to be confirmed: {}", w.pointer),
        })
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// The sources of the device `d`, then its midi_of sources.
fn providers_of(d: &Descriptor) -> Vec<Option<String>> {
    let Descriptor::Device(dev) = d else { panic!("a device") };
    [&dev.sources, &dev.midi_of_sources].into_iter().flatten().flatten().map(|s| s.provider.clone()).collect()
}

// ---- reading ----

#[test]
fn a_0_7_reader_holds_the_provider_of_a_source_of_either_list_and_writes_it_back_as_it_is() {
    let doc = device(
        Some(D07),
        json!({
            "sources": [{"type": "contributed", "provider": "electra-one"}, {"type": "vendor-doc"}],
            "midi_of_sources": [{"type": "vendor-doc", "provider": "acme-synths", "title": "Manual"}],
        }),
    );
    let d = read(&doc);
    assert_eq!(providers_of(&d), [Some("electra-one".to_string()), None, Some("acme-synths".to_string())]);
    assert_eq!(written(&d), doc);
    let Descriptor::Device(dev) = &d else { panic!("a device") };
    assert!(dev.sources.iter().flatten().all(|s| !s.extra.contains_key("provider")), "the field is not a kept key");
}

#[test]
fn a_provider_set_on_a_source_is_written_under_its_key() {
    let mut d = read(&device_07("sources", json!([{"type": "contributed"}])));
    if let Descriptor::Device(dev) = &mut d {
        dev.sources.as_mut().unwrap()[0].provider = Some("knobkraft".to_string());
    }
    assert_eq!(written(&d)["sources"], json!([{"type": "contributed", "provider": "knobkraft"}]));
}

#[test]
fn a_well_formed_identifier_of_any_shape_up_to_64_characters_is_accepted() {
    let longest = "a".repeat(64);
    let hyphens = format!("{}-{}", "a".repeat(31), "9".repeat(32));
    for id in ["a", "7", "electra-one", "a-new-firmware", "x-1-2-3", longest.as_str(), hyphens.as_str()] {
        let doc = device_07("sources", json!([{"type": "vendor-doc", "provider": id}]));
        assert_eq!(written(&read(&doc)), doc, "{id}");
    }
}

#[test]
fn a_provider_that_does_not_follow_the_form_is_refused_as_a_bad_value_at_its_pointer() {
    let long = "a".repeat(65);
    let bad = ["Electra One", "Electra", "electra_one", "electra one", "", "-a", "a-", "a--b", "-", "é", "acme.synths", "acme/synths", "a\n", long.as_str()];
    for list in LISTS {
        for id in bad {
            let doc = device_07(list, json!([{"type": "vendor-doc"}, {"type": "vendor-doc", "provider": id}]));
            assert_eq!(refused(&doc), (FaultKind::BadValue, format!("/{list}/1/provider")), "{id:?}");
        }
    }
}

#[test]
fn a_provider_that_is_not_a_text_is_refused_as_a_bad_value_at_its_pointer() {
    for list in LISTS {
        for value in [json!(null), json!(12), json!(true), json!([]), json!(["electra-one"]), json!({"id": "electra-one"})] {
            let doc = device_07(list, json!([{"type": "vendor-doc", "provider": value}]));
            assert_eq!(refused(&doc), (FaultKind::BadValue, format!("/{list}/0/provider")), "{value}");
        }
    }
}

#[test]
fn before_0_7_the_key_provider_is_kept_and_never_checked() {
    let long = "a".repeat(65);
    let values = [json!("electra-one"), json!("Electra One"), json!(long), json!(""), json!(12), json!(null), json!({"x": [1]})];
    for schema in [None, Some(D05), Some(D06)] {
        for value in &values {
            let doc = device(schema, json!({"_sources": [{"type": "vendor-doc", "provider": value}]}));
            let d = read(&doc);
            assert_eq!(written(&d)["_sources"], doc["_sources"], "{schema:?} {value}");
            assert_eq!(warned(&d), pairs(&[]), "{schema:?} {value}");
        }
    }
}

// ---- the source types of 0.7 ----

#[test]
fn a_source_type_named_after_a_product_is_refused_in_0_7_at_the_type() {
    for list in LISTS {
        for word in PRODUCTS {
            let doc = device_07(list, json!([{"type": "vendor-doc"}, {"type": word}]));
            assert_eq!(refused(&doc), (FaultKind::BadValue, format!("/{list}/1/type")), "{word}");
            let named = device_07(list, json!([{"type": word, "provider": "electra-one"}]));
            assert_eq!(refused(&named), (FaultKind::BadValue, format!("/{list}/0/type")), "{word}");
        }
    }
}

#[test]
fn the_ten_terms_of_evidence_and_the_extensions_are_source_types_of_0_7() {
    for list in LISTS {
        for word in EVIDENCE.iter().copied().chain(["x-acme-scraped", "x-osd-knobkraft"]) {
            let doc = device_07(list, json!([{"type": word}]));
            assert_eq!(written(&read(&doc)), doc, "{word}");
        }
    }
}

#[test]
fn before_0_7_the_four_product_types_still_read() {
    for schema in [None, Some(D05), Some(D06)] {
        for word in PRODUCTS {
            let doc = device(schema, json!({"_sources": [{"type": word}]}));
            assert_eq!(written(&read(&doc))["_sources"], doc["_sources"], "{schema:?} {word}");
        }
    }
}

#[test]
fn the_conversion_from_0_4_to_0_5_keeps_the_product_types() {
    for word in PRODUCTS {
        let doc = device(None, json!({"_sources": [{"type": word, "notes": "n"}]}));
        let out = osd::convert_to(&doc.to_string(), Kind::Device, Version::V0_5).unwrap_or_else(|f| panic!("{f}"));
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["_sources"], doc["_sources"], "{word}");
    }
}

// ---- the table the reader carries, and its warnings ----

fn table() -> Value {
    let text = std::fs::read_to_string(common::repo().join("osd/providers.json")).expect("osd/providers.json");
    serde_json::from_str(&text).unwrap()
}

#[test]
fn the_reader_publishes_the_version_of_the_table_it_carries() {
    assert_eq!(osd::PROVIDERS_VERSION, table()["version"].as_str().expect("a version"));
}

#[test]
fn a_well_formed_provider_the_table_does_not_hold_is_read_as_it_is_and_warned_of_at_its_pointer() {
    let doc = device(
        Some(D07),
        json!({
            "sources": [
                {"type": "contributed", "provider": "electra-one"},
                {"type": "contributed", "provider": "electra-oen"},
                {"type": "vendor-doc"},
                {"type": "vendor-doc", "provider": "a-new-firmware"},
            ],
            "midi_of_sources": [{"type": "vendor-doc", "provider": "acme-synths"}],
        }),
    );
    let d = read(&doc);
    let before = d.clone();
    assert_eq!(
        warned(&d),
        pairs(&[
            ("/sources/1/provider", "electra-oen"),
            ("/sources/3/provider", "a-new-firmware"),
            ("/midi_of_sources/0/provider", "acme-synths"),
        ])
    );
    assert_eq!(d, before);
    assert_eq!(written(&d), doc);
}

#[test]
fn each_source_that_cites_the_same_unknown_provider_gets_its_own_warning() {
    let d = read(&device_07("midi_of_sources", json!([{"type": "vendor-doc", "provider": "zz"}, {"type": "contributed", "provider": "zz"}])));
    assert_eq!(warned(&d), pairs(&[("/midi_of_sources/0/provider", "zz"), ("/midi_of_sources/1/provider", "zz")]));
}

#[test]
fn every_identifier_of_the_table_and_a_source_without_provider_give_no_warning() {
    let table = table();
    let mut sources: Vec<Value> = table["providers"]
        .as_array()
        .expect("providers")
        .iter()
        .map(|e| json!({"type": "contributed", "provider": e["id"]}))
        .collect();
    assert!(!sources.is_empty());
    sources.push(json!({"type": "vendor-doc"}));
    let d = read(&device_07("sources", Value::Array(sources)));
    assert_eq!(warned(&d), pairs(&[]));
}

#[test]
fn the_three_partners_of_the_former_source_types_are_known_to_the_reader_and_midnam_is_not() {
    let known = json!([
        {"type": "contributed", "provider": "electra-one"},
        {"type": "contributed", "provider": "pencilresearch"},
        {"type": "contributed", "provider": "knobkraft"},
    ]);
    assert_eq!(warned(&read(&device_07("sources", known))), pairs(&[]));
    let midnam = device_07("sources", json!([{"type": "contributed", "provider": "midnam"}]));
    assert_eq!(warned(&read(&midnam)), pairs(&[("/sources/0/provider", "midnam")]));
}

#[test]
fn a_descriptor_without_sources_gives_no_warning() {
    assert_eq!(warned(&read(&device(Some(D07), json!({})))), pairs(&[]));
}

// ---- the schema and the specification ----

const FORM: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";

#[test]
fn the_0_7_device_schema_describes_provider_by_its_form_and_length_without_a_list_of_identifiers() {
    let text = std::fs::read_to_string(common::repo().join("schemas/0.7/device.schema.json")).unwrap();
    let schema: Value = serde_json::from_str(&text).unwrap();
    let field = &schema["$defs"]["source"]["properties"]["provider"];
    assert_eq!(field["type"], "string", "{field}");
    assert_eq!(field["pattern"], FORM, "{field}");
    assert_eq!(field["maxLength"], 64, "{field}");
    assert!(field["description"].as_str().is_some_and(|d| !d.is_empty()), "{field}");
    assert!(field.get("enum").is_none() && field.get("anyOf").is_none() && field.get("oneOf").is_none(), "{field}");
}

#[test]
fn the_0_7_device_schema_judges_provider_and_source_types_as_the_reader_does() {
    let schemas = common::Schemas::load("0.7");
    let long = "a".repeat(65);
    for list in LISTS {
        for word in EVIDENCE.iter().copied().chain(["x-acme-scraped"]) {
            let doc = device_07(list, json!([{"type": word, "provider": "a-new-firmware"}]));
            assert_eq!(schemas.check(Kind::Device, &doc), None, "{doc}");
        }
        for word in PRODUCTS {
            let doc = device_07(list, json!([{"type": word}]));
            let errors = schemas.errors(Kind::Device, &doc);
            assert!(errors.iter().any(|(at, _)| at == &format!("/{list}/0/type")), "{word}: {errors:?}");
        }
        for id in [json!("Electra One"), json!("a--b"), json!(""), json!(long), json!(7)] {
            let doc = device_07(list, json!([{"type": "vendor-doc", "provider": id}]));
            let errors = schemas.errors(Kind::Device, &doc);
            assert!(errors.iter().any(|(at, _)| at == &format!("/{list}/0/provider")), "{id}: {errors:?}");
        }
    }
}

#[test]
fn the_text_of_0_7_names_the_field_and_the_table_of_providers() {
    let spec = std::fs::read_to_string(common::repo().join("spec/0.7.md")).unwrap();
    for needed in ["`provider`", "schemas/providers.json", FORM] {
        assert!(spec.contains(needed), "spec/0.7.md does not say {needed}");
    }
}
