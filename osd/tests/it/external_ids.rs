//! The identities other tools give a sound (0.6): `external_ids`, a top-level list of the sound
//! descriptor, each entry `{tool, by, algorithm, value}`. `tool` and `value` are required; `by`
//! names what in the tool computes the identity per device (a KnobKraft Orm adaptation);
//! `algorithm` names the hash when it is known (`md5` 32, `sha1` 40, `sha256` 64 lowercase
//! hexadecimal digits, names read without case); an unknown or absent algorithm only requires a
//! non-empty value. The tool `OSD` (any case) is refused: OSD's own identity is never in the list.
//! Two entries of the same computation (same `tool`, `by`, `algorithm`, each compared without
//! case, an absent one equal to an absent one) with different values are refused at the later
//! entry. A 0.4 or 0.5 sound refuses the field; `osd::convert` adds none.

use crate::common;

use osd::{FaultKind, Kind, Version};
use serde_json::{json, Value};

const MD5: &str = "0123456789abcdef0123456789abcdef";
const MD5_OTHER: &str = "fedcba9876543210fedcba9876543210";
const SHA1: &str = "0123456789abcdef0123456789abcdef01234567";
const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn file(path: &str) -> String {
    std::fs::read_to_string(common::repo().join(path)).unwrap()
}

/// The 0.6 sound with `external_ids` set to `ids`.
fn sound(ids: Value) -> Value {
    let mut doc: Value = serde_json::from_str(&file("examples/0.5/warm-pad.sound.json").replace("/osd/0.5/", "/osd/0.6/")).unwrap();
    doc["external_ids"] = ids;
    doc
}

fn read(doc: &Value) -> Result<osd::Descriptor, osd::Fault> {
    osd::read(&doc.to_string(), Kind::Sound)
}

fn refused(ids: Value, kind: FaultKind, pointer: &str) {
    let f = read(&sound(ids.clone())).expect_err(&format!("{ids} should be refused"));
    assert_eq!((f.kind, f.pointer.as_str()), (kind, pointer), "{ids}: {f}");
}

fn accepted(ids: Value) {
    if let Err(f) = read(&sound(ids.clone())) {
        panic!("{ids} should read: {f}");
    }
}

fn knobkraft(value: &str) -> Value {
    json!({"tool": "KnobKraft Orm", "by": "Korg-minilogue", "algorithm": "md5", "value": value})
}

#[test]
fn a_0_6_sound_reads_and_rewrites_its_external_ids_with_their_keys_order_and_values() {
    let ids = json!([
        knobkraft(MD5),
        {"tool": "KnobKraft Orm", "by": "Korg-minilogue", "algorithm": "sha1", "value": SHA1, "_note": "a note stays"},
        {"tool": "Other tool", "value": "Any-Opaque Value 42"},
        {"tool": "Third", "algorithm": "sha256", "value": SHA256}
    ]);
    let d = read(&sound(ids.clone())).unwrap_or_else(|f| panic!("{f}"));
    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert_eq!(back["external_ids"], ids);
    // The order of the entries is the order written.
    let text = osd::write(&d);
    let at = |s: &str| text.find(s).unwrap_or_else(|| panic!("{s} not written"));
    assert!(at(MD5) < at(SHA1) && at(SHA1) < at("Any-Opaque Value 42") && at("Any-Opaque Value 42") < at(SHA256), "{text}");
    // Reading what was written gives the same sound.
    assert_eq!(osd::read(&text, Kind::Sound).unwrap(), d);
}

#[test]
fn external_ids_is_allowed_for_every_kind_of_sound() {
    for kind in ["sound", "kit", "sequence", "project", "bank", "clip", "performance", "scene", "setlist"] {
        let mut doc = sound(json!([knobkraft(MD5)]));
        doc["kind"] = json!(kind);
        if let Err(f) = read(&doc) {
            panic!("{kind}: {f}");
        }
    }
}

#[test]
fn a_0_6_sound_without_external_ids_reads_and_writes_as_before() {
    let mut doc = sound(json!([]));
    doc.as_object_mut().unwrap().remove("external_ids");
    let d = read(&doc).unwrap_or_else(|f| panic!("{f}"));
    let back: Value = serde_json::from_str(&osd::write(&d)).unwrap();
    assert!(back.get("external_ids").is_none(), "{back}");
    assert_eq!(back, doc);
}

#[test]
fn an_entry_without_its_tool_or_its_value_is_missing_at_its_address() {
    refused(json!([{"algorithm": "md5", "value": MD5}]), FaultKind::Missing, "/external_ids/0/tool");
    refused(json!([{"tool": "KnobKraft Orm", "algorithm": "md5"}]), FaultKind::Missing, "/external_ids/0/value");
    refused(json!([knobkraft(MD5), {"value": "x"}]), FaultKind::Missing, "/external_ids/1/tool");
    refused(json!([knobkraft(MD5), {"tool": "T"}]), FaultKind::Missing, "/external_ids/1/value");
}

#[test]
fn an_unknown_key_of_an_entry_is_refused_where_it_is() {
    let mut e = knobkraft(MD5);
    e["colour"] = json!("red");
    refused(json!([e]), FaultKind::UnknownField, "/external_ids/0/colour");
}

#[test]
fn an_entry_that_is_not_an_object_or_a_list_that_is_not_a_list_is_refused() {
    refused(json!(["md5"]), FaultKind::BadValue, "/external_ids/0");
    refused(json!({"tool": "T", "value": "v"}), FaultKind::BadValue, "/external_ids");
}

#[test]
fn an_empty_value_is_refused_at_its_value() {
    refused(json!([{"tool": "T", "value": ""}]), FaultKind::BadValue, "/external_ids/0/value");
    refused(json!([{"tool": "T", "algorithm": "md5", "value": ""}]), FaultKind::BadValue, "/external_ids/0/value");
}

#[test]
fn a_known_algorithm_fixes_the_form_of_the_value() {
    accepted(json!([{"tool": "T", "algorithm": "md5", "value": MD5}]));
    accepted(json!([{"tool": "T", "algorithm": "sha1", "value": SHA1}]));
    accepted(json!([{"tool": "T", "algorithm": "sha256", "value": SHA256}]));
    for (algorithm, value) in [
        ("md5", "0123456789abcdef"),                              // too short
        ("md5", "0123456789abcdef0123456789abcdef0"),             // too long
        ("md5", "0123456789abcdef0123456789abcdeg"),              // not hexadecimal
        ("md5", "0123456789ABCDEF0123456789ABCDEF"),              // upper case
        ("md5", "0123456789abcdef0123456789abcde "),              // a space
        ("sha1", MD5),                                            // an md5 is no sha1
        ("sha1", "0123456789ABCDEF0123456789ABCDEF01234567"),
        ("sha256", SHA1),
        ("sha256", "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"),
    ] {
        refused(json!([{"tool": "T", "algorithm": algorithm, "value": value}]), FaultKind::BadValue, "/external_ids/0/value");
    }
}

#[test]
fn an_algorithm_name_is_recognised_without_case() {
    accepted(json!([{"tool": "T", "algorithm": "MD5", "value": MD5}]));
    accepted(json!([{"tool": "T", "algorithm": "Sha256", "value": SHA256}]));
    refused(json!([{"tool": "T", "algorithm": "MD5", "value": "ABCDEF0123456789ABCDEF0123456789"}]), FaultKind::BadValue, "/external_ids/0/value");
    refused(json!([{"tool": "T", "algorithm": "SHA1", "value": MD5}]), FaultKind::BadValue, "/external_ids/0/value");
}

#[test]
fn an_unknown_or_absent_algorithm_only_requires_a_non_empty_value() {
    accepted(json!([{"tool": "T", "algorithm": "crc32", "value": "DEADBEEF"}]));
    accepted(json!([{"tool": "T", "value": "A-Tool-Written ID 7"}]));
    refused(json!([{"tool": "T", "algorithm": "crc32", "value": ""}]), FaultKind::BadValue, "/external_ids/0/value");
}

#[test]
fn the_tool_osd_is_refused_in_any_case_because_the_identity_of_osd_is_never_in_the_list() {
    for tool in ["OSD", "osd", "Osd", "oSD"] {
        refused(
            json!([{"tool": tool, "algorithm": "sha256", "value": SHA256}]),
            FaultKind::BadValue,
            "/external_ids/0/tool",
        );
    }
    refused(json!([knobkraft(MD5), {"tool": "OSD", "value": "x"}]), FaultKind::BadValue, "/external_ids/1/tool");
    // A tool whose name only contains OSD is another tool.
    accepted(json!([{"tool": "OSD Explorer", "value": "x"}]));
}

#[test]
fn two_entries_of_the_same_computation_with_different_values_are_refused_at_the_later_one() {
    refused(json!([knobkraft(MD5), knobkraft(MD5_OTHER)]), FaultKind::BadValue, "/external_ids/1");
    // Compared without case on the tool, the adaptation and the algorithm.
    let loud = json!({"tool": "KNOBKRAFT ORM", "by": "KORG-MINILOGUE", "algorithm": "MD5", "value": MD5_OTHER});
    refused(json!([knobkraft(MD5), loud]), FaultKind::BadValue, "/external_ids/1");
    // Both without `by`, both without `algorithm`.
    refused(json!([{"tool": "T", "value": "a"}, {"tool": "t", "value": "b"}]), FaultKind::BadValue, "/external_ids/1");
    // A later duplicate among other entries is the one refused.
    refused(
        json!([{"tool": "X", "value": "1"}, knobkraft(MD5), {"tool": "Y", "value": "2"}, knobkraft(MD5_OTHER)]),
        FaultKind::BadValue,
        "/external_ids/3",
    );
}

#[test]
fn entries_that_differ_by_tool_by_or_algorithm_are_not_duplicates() {
    // The md5 and the sha1 of the same adaptation.
    accepted(json!([knobkraft(MD5), {"tool": "KnobKraft Orm", "by": "Korg-minilogue", "algorithm": "sha1", "value": SHA1}]));
    // Another adaptation, another tool, one with `by` and one without.
    accepted(json!([knobkraft(MD5), {"tool": "KnobKraft Orm", "by": "Other", "algorithm": "md5", "value": MD5_OTHER}]));
    accepted(json!([knobkraft(MD5), {"tool": "Other tool", "by": "Korg-minilogue", "algorithm": "md5", "value": MD5_OTHER}]));
    accepted(json!([knobkraft(MD5), {"tool": "KnobKraft Orm", "algorithm": "md5", "value": MD5_OTHER}]));
    // Same computation, same value: not a duplicate.
    accepted(json!([knobkraft(MD5), knobkraft(MD5)]));
}

#[test]
fn a_0_4_or_0_5_sound_refuses_external_ids_as_an_unknown_field() {
    for version in ["0.4", "0.5"] {
        let mut doc: Value = serde_json::from_str(&file(if version == "0.4" { "examples/0.4/warm-pad.sound.json" } else { "examples/0.5/warm-pad.sound.json" })).unwrap();
        doc["external_ids"] = json!([knobkraft(MD5)]);
        let f = osd::read(&doc.to_string(), Kind::Sound).expect_err(version);
        assert_eq!((f.kind, f.pointer.as_str()), (FaultKind::UnknownField, "/external_ids"), "{version}: {f}");
    }
}

#[test]
fn convert_from_0_5_adds_no_external_ids() {
    let text = file("examples/0.5/warm-pad.sound.json");
    for out in [osd::convert(&text, Kind::Sound).unwrap(), osd::convert_to(&text, Kind::Sound, Version::V0_6).unwrap()] {
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert!(doc.get("external_ids").is_none(), "{out}");
    }
}

#[test]
fn the_0_6_sound_schema_carries_the_field_and_a_pattern_per_algorithm_and_agrees_with_the_reader() {
    let schemas = common::Schemas::load("0.6");
    let ok = sound(json!([knobkraft(MD5), {"tool": "T", "value": "free form"}]));
    assert!(schemas.errors(Kind::Sound, &ok).is_empty(), "{:?}", schemas.errors(Kind::Sound, &ok));
    let at = |ids: Value| schemas.errors(Kind::Sound, &sound(ids)).into_iter().map(|(p, _)| p).collect::<Vec<_>>();
    assert!(at(json!([{"algorithm": "md5", "value": MD5}])).iter().any(|p| p.starts_with("/external_ids/0")));
    assert!(at(json!([{"tool": "T"}])).iter().any(|p| p.starts_with("/external_ids/0")));
    assert!(at(json!([{"tool": "T", "value": "v", "colour": "red"}])).iter().any(|p| p.starts_with("/external_ids/0")));
    for (algorithm, value) in [("md5", "0123456789abcdef"), ("md5", "0123456789ABCDEF0123456789ABCDEF"), ("sha1", MD5), ("sha256", SHA1), ("MD5", "short")] {
        let errors = at(json!([{"tool": "T", "algorithm": algorithm, "value": value}]));
        assert!(errors.iter().any(|p| p == "/external_ids/0/value"), "{algorithm} {value}: {errors:?}");
    }
    assert!(at(json!([{"tool": "T", "algorithm": "SHA256", "value": SHA256}, {"tool": "T", "algorithm": "crc32", "value": "FF"}])).is_empty());
}

#[test]
fn the_0_6_conformance_cases_give_a_knobkraft_fingerprint_and_one_case_per_refusal() {
    let read = common::repo().join("conformance/0.6/read");
    for case in [
        "external-ids-knobkraft-md5",
        "external-ids-tool-missing-refused",
        "external-ids-value-missing-refused",
        "external-ids-same-computation-different-values-refused",
        "external-ids-md5-form-refused",
        "external-ids-osd-tool-refused",
    ] {
        assert!(read.join(case).join("case.json").is_file(), "{case}");
        assert!(read.join(case).join("descriptor.json").is_file(), "{case}");
    }
    let ok: Value = serde_json::from_str(&std::fs::read_to_string(read.join("external-ids-knobkraft-md5/descriptor.json")).unwrap()).unwrap();
    assert_eq!(ok["external_ids"][0]["tool"], "KnobKraft Orm");
    assert_eq!(ok["external_ids"][0]["algorithm"], "md5");
}
