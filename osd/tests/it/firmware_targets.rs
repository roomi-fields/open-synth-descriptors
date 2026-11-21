//! The targets of a 0.6 firmware descriptor: the devices it is made for, each with the base
//! firmware versions it accepts (an explicit list or a range); how two base versions compare;
//! the conversion of a 0.5 `device` + `based_on` into one target.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};
use std::cmp::Ordering;

const SCHEMA_06: &str = "https://w3id.org/osd/0.6/firmware.schema.json";
const SCHEMA_05: &str = "https://w3id.org/osd/0.5/firmware.schema.json";

fn digitone() -> Value {
    json!({"vendor": "Elektron", "model": "Digitone"})
}

/// A 0.6 firmware descriptor of kind `mod` holding `targets`.
fn fw(targets: Value) -> Value {
    json!({"$schema": SCHEMA_06, "name": "Elekloader mod", "version": "2.0", "kind": "mod", "targets": targets})
}

fn one_target(versions: Value) -> Value {
    fw(json!([{"device": digitone(), "based_on": {"name": "Elektron OS", "versions": versions}}]))
}

/// The refusal of `doc` by the reader and by the schema: the kind and pointer of the reader's,
/// and the pointer of the schema's single error (at the parent for a missing or unknown field).
fn refused(doc: &Value, kind: FaultKind, pointer: &str) {
    let fault = osd::read(&doc.to_string(), Kind::Firmware).expect_err(&format!("{doc} is refused"));
    assert_eq!((fault.kind, fault.pointer.as_str()), (kind, pointer), "the reader refuses {doc}: {fault}");
    let errors = common::Schemas::load("0.6").errors(Kind::Firmware, doc);
    let want = if matches!(kind, FaultKind::Missing | FaultKind::UnknownField) {
        pointer.rsplit_once('/').map_or("", |(parent, _)| parent)
    } else {
        pointer
    };
    let got: Vec<&str> = errors.iter().map(|(at, _)| at.as_str()).collect();
    assert!(!got.is_empty() && got.iter().all(|at| *at == want), "the schema refuses {doc} at {want}, not {errors:?}");
}

/// The firmware read, written back as a JSON value.
fn written(doc: &Value) -> Value {
    let d = osd::read(&doc.to_string(), Kind::Firmware).unwrap_or_else(|f| panic!("{doc}: {f}"));
    serde_json::from_str(&osd::write(&d)).unwrap()
}

fn valid_in_06(doc: &Value) {
    assert_eq!(common::Schemas::load("0.6").check(Kind::Firmware, doc), None, "{doc}");
}

/// The `index`-th target of a 0.6 firmware descriptor.
fn target(doc: &Value, index: usize) -> osd::Target {
    match osd::read(&doc.to_string(), Kind::Firmware).unwrap() {
        osd::Descriptor::Firmware(f) => f.targets.expect("a 0.6 firmware holds targets")[index].clone(),
        _ => unreachable!(),
    }
}

// ---- reading, one test per form ----

#[test]
fn a_target_gives_its_base_versions_as_an_explicit_list() {
    let doc = one_target(json!(["1.43", "1.44"]));
    valid_in_06(&doc);
    assert_eq!(written(&doc), doc);
}

#[test]
fn a_target_gives_its_base_versions_as_a_range_with_min_and_max() {
    let doc = one_target(json!({"min": "1.40B", "max": "1.41"}));
    valid_in_06(&doc);
    assert_eq!(written(&doc), doc);
}

#[test]
fn a_range_may_hold_a_min_alone_or_a_max_alone() {
    for range in [json!({"min": "1.40B"}), json!({"max": "1.41"})] {
        let doc = one_target(range);
        valid_in_06(&doc);
        assert_eq!(written(&doc), doc);
    }
}

#[test]
fn a_module_names_several_targets_of_different_devices_and_forms() {
    let doc = fw(json!([
        {"device": {"descriptor": "elektron/digitone", "vendor": "Elektron", "model": "Digitone"}, "based_on": {"name": "Elektron OS", "url": "https://www.elektron.se", "versions": ["1.43", "1.44"]}},
        {"device": {"vendor": "Elektron", "model": "Digitone Keys"}, "based_on": {"name": "Elektron OS", "versions": {"min": "1.43", "max": "1.44"}}},
        {"device": {"vendor": "Elektron", "model": "Digitakt"}}
    ]));
    valid_in_06(&doc);
    assert_eq!(written(&doc), doc);
}

#[test]
fn every_kind_of_firmware_writes_targets() {
    for kind in ["firmware", "mod", "script", "extension"] {
        let mut doc = one_target(json!(["1.0"]));
        doc["kind"] = json!(kind);
        valid_in_06(&doc);
        assert_eq!(written(&doc), doc, "{kind}");
    }
}

// ---- refusals, reader and schema at the same pointer ----

#[test]
fn a_target_without_device_is_refused() {
    refused(&fw(json!([{"based_on": {"name": "Elektron OS", "versions": ["1.43"]}}])), FaultKind::Missing, "/targets/0/device");
    refused(&fw(json!([{"device": digitone()}, {"device": {"vendor": "Elektron"}}])), FaultKind::Missing, "/targets/1/device/model");
}

#[test]
fn a_firmware_without_targets_or_with_an_empty_list_is_refused() {
    let mut doc = fw(json!([]));
    refused(&doc, FaultKind::BadValue, "/targets");
    doc.as_object_mut().unwrap().remove("targets");
    refused(&doc, FaultKind::Missing, "/targets");
}

#[test]
fn a_range_with_neither_min_nor_max_is_refused() {
    refused(&one_target(json!({})), FaultKind::BadValue, "/targets/0/based_on/versions");
}

#[test]
fn an_empty_list_of_versions_is_refused() {
    refused(&one_target(json!([])), FaultKind::BadValue, "/targets/0/based_on/versions");
}

#[test]
fn a_version_that_is_not_letters_digits_and_dots_with_a_digit_is_refused() {
    refused(&one_target(json!(["1.43", "1.44-beta"])), FaultKind::BadValue, "/targets/0/based_on/versions/1");
    refused(&one_target(json!(["abc"])), FaultKind::BadValue, "/targets/0/based_on/versions/0");
    refused(&one_target(json!([1.43])), FaultKind::BadValue, "/targets/0/based_on/versions/0");
    refused(&one_target(json!({"min": "", "max": "1.41"})), FaultKind::BadValue, "/targets/0/based_on/versions/min");
    refused(&one_target(json!({"min": "1.40", "max": "v 2"})), FaultKind::BadValue, "/targets/0/based_on/versions/max");
}

#[test]
fn a_base_firmware_without_name_is_refused() {
    refused(&fw(json!([{"device": digitone(), "based_on": {"versions": ["1.0"]}}])), FaultKind::Missing, "/targets/0/based_on/name");
}

#[test]
fn the_single_device_and_based_on_of_0_5_are_refused_in_0_6() {
    let mut doc = one_target(json!(["1.0"]));
    doc["device"] = digitone();
    refused(&doc, FaultKind::UnknownField, "/device");
    let mut doc = one_target(json!(["1.0"]));
    doc["based_on"] = json!({"name": "Elektron OS", "version": "1.0"});
    refused(&doc, FaultKind::UnknownField, "/based_on");
    let old = json!({"$schema": SCHEMA_06, "name": "x", "version": "1", "device": digitone()});
    assert!(osd::read(&old.to_string(), Kind::Firmware).is_err());
    assert!(common::Schemas::load("0.6").check(Kind::Firmware, &old).is_some());
}

// ---- comparing versions ----

#[test]
fn two_base_versions_compare_component_by_component_numbers_then_letters_without_case() {
    let less = [
        ("1.40", "1.40B"), ("1.40B", "1.40C"), ("1.40C", "1.41"), ("1.9", "1.10"), ("1.40.1", "1.40B"),
        ("1.2", "1.2.1"), ("2", "10"), ("1.40A", "1.40b"), ("1.40Z", "1.41"),
    ];
    for (a, b) in less {
        assert_eq!(osd::compare_versions(a, b), Ordering::Less, "{a} < {b}");
        assert_eq!(osd::compare_versions(b, a), Ordering::Greater, "{b} > {a}");
    }
    let equal = [("1.40", "1.40"), ("1.40", "1.40.0"), ("1", "1.0.0"), ("1.40c", "1.40C"), ("01.5", "1.5")];
    for (a, b) in equal {
        assert_eq!(osd::compare_versions(a, b), Ordering::Equal, "{a} = {b}");
        assert_eq!(osd::compare_versions(b, a), Ordering::Equal, "{b} = {a}");
    }
}

#[test]
fn a_list_of_base_versions_tests_by_the_same_rule_and_a_range_includes_its_bounds() {
    let list = target(&one_target(json!(["1.40", "1.43"])), 0);
    assert!(osd::version_in_target("1.40.0", &list));
    assert!(osd::version_in_target("1.43", &list));
    assert!(!osd::version_in_target("1.40B", &list));
    assert!(!osd::version_in_target("1.41", &list));
    let range = target(&one_target(json!({"min": "1.40B", "max": "1.41"})), 0);
    for v in ["1.40B", "1.40C", "1.41", "1.41.0"] {
        assert!(osd::version_in_target(v, &range), "{v}");
    }
    for v in ["1.40", "1.40A", "1.42", "1.9"] {
        assert!(!osd::version_in_target(v, &range), "{v}");
    }
}

#[test]
fn a_range_with_a_min_alone_is_open_upward_and_a_max_alone_downward() {
    let up = target(&one_target(json!({"min": "1.40B"})), 0);
    assert!(osd::version_in_target("1.40B", &up) && osd::version_in_target("2.0", &up) && osd::version_in_target("1.100", &up));
    assert!(!osd::version_in_target("1.40", &up));
    let down = target(&one_target(json!({"max": "1.41"})), 0);
    assert!(osd::version_in_target("1.41", &down) && osd::version_in_target("0.1", &down));
    assert!(!osd::version_in_target("1.41.1", &down) && !osd::version_in_target("1.42", &down));
}

// ---- conversion ----

fn doc_05() -> Value {
    json!({
        "$schema": SCHEMA_05, "name": "TRIBE", "version": "0.3-beta", "kind": "mod",
        "device": {"descriptor": "m-vave/fm-1", "vendor": "M-VAVE", "model": "FM-1"},
        "based_on": {"name": "Elektron OS", "version": "1.43", "url": "https://example.org/os"},
        "license": "MIT", "official": false,
        "author": [{"name": "Someone"}]
    })
}

fn converted(doc: &Value) -> Value {
    serde_json::from_str(&osd::convert_to(&doc.to_string(), Kind::Firmware, Version::V0_6).unwrap_or_else(|f| panic!("{doc}: {f}"))).unwrap()
}

#[test]
fn a_0_5_device_and_based_on_become_one_target_with_a_list_of_one_version() {
    let out = converted(&doc_05());
    assert_eq!(out["$schema"], SCHEMA_06);
    assert_eq!(
        out["targets"],
        json!([{"device": {"descriptor": "m-vave/fm-1", "vendor": "M-VAVE", "model": "FM-1"},
                "based_on": {"name": "Elektron OS", "url": "https://example.org/os", "versions": ["1.43"]}}])
    );
    assert!(out.get("device").is_none() && out.get("based_on").is_none());
    for kept in ["name", "version", "kind", "license", "official", "author"] {
        assert_eq!(out[kept], doc_05()[kept], "{kept}");
    }
    valid_in_06(&out);
    assert!(osd::read_as(&out.to_string(), Kind::Firmware, Version::V0_6).is_ok());
}

#[test]
fn a_0_5_firmware_without_based_on_becomes_a_target_without_base_versions() {
    let mut doc = doc_05();
    doc.as_object_mut().unwrap().remove("based_on");
    let out = converted(&doc);
    assert_eq!(out["targets"], json!([{"device": doc["device"]}]));
    valid_in_06(&out);
}

#[test]
fn a_0_5_based_on_without_version_becomes_a_base_without_versions() {
    let mut doc = doc_05();
    doc["based_on"] = json!({"name": "Elektron OS"});
    let out = converted(&doc);
    assert_eq!(out["targets"][0]["based_on"], json!({"name": "Elektron OS"}));
    valid_in_06(&out);
}

#[test]
fn every_kind_of_a_0_5_firmware_converts_to_one_target() {
    for kind in ["firmware", "mod", "script", "extension"] {
        let mut doc = doc_05();
        doc["kind"] = json!(kind);
        let out = converted(&doc);
        assert_eq!(out["targets"].as_array().map(Vec::len), Some(1), "{kind}");
        valid_in_06(&out);
    }
}

#[test]
fn a_converted_firmware_converts_to_itself_and_goes_forward_only() {
    let out = converted(&doc_05());
    let text = out.to_string();
    assert_eq!(osd::convert_to(&text, Kind::Firmware, Version::V0_6).map(|t| serde_json::from_str::<Value>(&t).unwrap()), Ok(out.clone()));
    assert!(osd::convert_to(&text, Kind::Firmware, Version::V0_5).is_err());
}

#[test]
fn a_0_4_firmware_converts_through_0_5_to_one_target() {
    let text = std::fs::read_to_string(common::repo().join("examples/tribe-0.3-beta.firmware.json")).unwrap();
    let old: Value = serde_json::from_str(&text).unwrap();
    let out = converted(&old);
    assert_eq!(out["targets"].as_array().map(Vec::len), Some(1));
    assert_eq!(out["targets"][0]["device"], old["device"]);
    assert!(out.get("device").is_none());
    valid_in_06(&out);
}

#[test]
fn the_conversion_of_a_0_5_firmware_reads_as_the_equivalent_0_6_one() {
    let mut doc = doc_05();
    doc["based_on"] = json!({"name": "Elektron OS", "version": "1.43"});
    let want = fw(json!([{"device": doc["device"], "based_on": {"name": "Elektron OS", "versions": ["1.43"]}}]));
    let out = converted(&doc);
    let mut want_full = want.clone();
    for k in ["name", "version", "kind", "license", "official", "author"] {
        want_full[k] = doc[k].clone();
    }
    assert_eq!(written(&out), want_full);
}
